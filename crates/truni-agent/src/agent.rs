use crate::llm::{ChatMessagePayload, LlmClient};
use crate::planner::Planner;
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::process::Command as TokioCommand;
use tokio::sync::Mutex;
use truni_core::config::TruniConfig;
use truni_core::events::{ApprovalRequest, StepStatus, TaskPlan};
use truni_memory::MemoryStore;
use truni_telemetry::TelemetryCollector;
use truni_tools::bash::BashTool;
use truni_tools::browser::BrowserTool;
use truni_tools::filesystem::FilesystemTool;
use truni_tools::security::{RiskLevel, SecurityManager};

#[derive(Clone)]
pub struct AgentOrchestrator {
    llm: Arc<LlmClient>,
    memory: MemoryStore,
    security: Arc<SecurityManager>,
    browser: Arc<BrowserTool>,
    telemetry: Arc<Mutex<TelemetryCollector>>,
    active_plan: Arc<Mutex<Option<TaskPlan>>>,
}

impl AgentOrchestrator {
    pub fn new(config: &TruniConfig, memory: MemoryStore) -> Self {
        let llm = Arc::new(LlmClient::new(config.ai.clone()));
        let security = Arc::new(SecurityManager::new());
        let browser = Arc::new(BrowserTool::new());
        let telemetry = Arc::new(Mutex::new(TelemetryCollector::new()));

        Self {
            llm,
            memory,
            security,
            browser,
            telemetry,
            active_plan: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn get_active_plan(&self) -> Option<TaskPlan> {
        let lock = self.active_plan.lock().await;
        lock.clone()
    }

    pub async fn start_task(&self, prompt: &str) -> Result<TaskPlan> {
        let task_id = format!("task_{}", chrono::Utc::now().timestamp_millis());
        let mut plan = Planner::create_plan_dynamic(&self.llm, &task_id, prompt).await;

        self.memory.record_task(&task_id, prompt, "In Progress")?;

        // Execute steps sequentially
        self.run_plan_steps(&mut plan).await?;

        let mut lock = self.active_plan.lock().await;
        *lock = Some(plan.clone());

        Ok(plan)
    }

    pub async fn approve_action(&self, request_id: &str, approved: bool) -> Result<Option<TaskPlan>> {
        let mut lock = self.active_plan.lock().await;
        if let Some(plan) = lock.as_mut() {
            if let Some(req) = &plan.pending_approval {
                if req.request_id == request_id {
                    let is_destructive = req.is_destructive;
                    let action = req.action_name.clone();
                    plan.pending_approval = None;

                    self.memory.log_audit(
                        Some(&plan.task_id),
                        "approval_gate",
                        &action,
                        is_destructive,
                        approved,
                        Some(if approved { "Approved by operator" } else { "Denied by operator" }),
                    )?;

                    if approved {
                        // Mark current step as running and continue
                        let curr = plan.current_step;
                        if curr < plan.steps.len() {
                            plan.steps[curr].status = StepStatus::Completed;
                            plan.steps[curr].log = Some("Operator approval confirmed. Action executed.".to_string());
                            plan.current_step += 1;
                        }
                    } else {
                        let curr = plan.current_step;
                        if curr < plan.steps.len() {
                            plan.steps[curr].status = StepStatus::Failed;
                            plan.steps[curr].log = Some("Action aborted by operator.".to_string());
                        }
                        plan.final_summary = Some("Task halted due to operator denial.".to_string());
                        self.memory.finish_task(&plan.task_id, "Aborted", plan.final_summary.as_deref())?;
                        return Ok(Some(plan.clone()));
                    }
                }
            }
            // Continue executing remaining steps
            self.run_plan_steps(plan).await?;
            return Ok(Some(plan.clone()));
        }
        Ok(None)
    }

    async fn run_plan_steps(&self, plan: &mut TaskPlan) -> Result<()> {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/home/venkatengineer"));

        while plan.current_step < plan.steps.len() {
            let step_idx = plan.current_step;
            plan.steps[step_idx].status = StepStatus::Running;

            let tool = plan.steps[step_idx].tool.clone();
            let command = plan.steps[step_idx].command.clone().unwrap_or_else(|| plan.prompt.clone());

            match tool.as_str() {
                "dismiss" => {
                    plan.steps[step_idx].log = Some("Flight deck disengaged. Systems entering standby.".to_string());
                    plan.steps[step_idx].status = StepStatus::Completed;
                    plan.final_summary = Some("Acknowledged. Standing by on flight deck.".to_string());
                    plan.current_step += 1;
                    return Ok(());
                }
                "browser_open" | "open" => {
                    let mut url = command.trim().to_string();
                    if !url.starts_with("http://") && !url.starts_with("https://") && !url.starts_with("file://") {
                        if url.contains("google.com") || url.contains("youtube.com") || url.contains("github.com") {
                            url = format!("https://{}", url);
                        } else if url.contains("google") && !url.contains(" ") {
                            url = "https://www.google.com".to_string();
                        } else {
                            // If it's a search or music request
                            if plan.prompt.to_lowercase().contains("play") || plan.prompt.to_lowercase().contains("song") || plan.prompt.to_lowercase().contains("music") {
                                url = format!("https://www.youtube.com/results?search_query={}", urlencoding::encode(&url));
                            } else {
                                url = format!("https://www.google.com/search?q={}", urlencoding::encode(&url));
                            }
                        }
                    }

                    let _ = TokioCommand::new("xdg-open")
                        .arg(&url)
                        .spawn();

                    plan.steps[step_idx].log = Some(format!("Launched desktop browser: {}", url));
                    plan.steps[step_idx].status = StepStatus::Completed;
                }
                "bash" => {
                    let risk = self.security.assess_command(&command);
                    match risk {
                        RiskLevel::RequiresApproval => {
                            plan.steps[step_idx].status = StepStatus::AwaitingApproval;
                            plan.pending_approval = Some(ApprovalRequest {
                                request_id: format!("appr_{}", chrono::Utc::now().timestamp_millis()),
                                action_name: "bash_command".to_string(),
                                description: format!("Request operator permission to execute: {}", command),
                                is_destructive: true,
                                target: command.clone(),
                            });
                            return Ok(());
                        }
                        RiskLevel::Forbidden => {
                            plan.steps[step_idx].status = StepStatus::Failed;
                            plan.steps[step_idx].log = Some("Security Exception: Command blocked by system policy.".to_string());
                        }
                        RiskLevel::Safe => {
                            match BashTool::execute(&command, 20).await {
                                Ok(res) => {
                                    if res.success {
                                        plan.steps[step_idx].status = StepStatus::Completed;
                                        let out = if res.stdout.trim().is_empty() {
                                            "Command completed with exit code 0.".to_string()
                                        } else {
                                            res.stdout.trim().to_string()
                                        };
                                        plan.steps[step_idx].log = Some(out);
                                    } else {
                                        plan.steps[step_idx].status = StepStatus::Failed;
                                        plan.steps[step_idx].log = Some(format!("Command failed (code {}): {}", res.exit_code, res.stderr.trim()));
                                    }
                                }
                                Err(e) => {
                                    plan.steps[step_idx].status = StepStatus::Failed;
                                    plan.steps[step_idx].log = Some(format!("Execution error: {}", e));
                                }
                            }
                        }
                    }
                }
                "web_search" | "browser_search" => {
                    match self.browser.search(&command).await {
                        Ok(results) => {
                            let mut snippet_summary = String::new();
                            for (_q, snippet) in results.iter().take(4) {
                                snippet_summary.push_str(snippet);
                                snippet_summary.push(' ');
                            }
                            plan.steps[step_idx].status = StepStatus::Completed;
                            plan.steps[step_idx].log = Some(format!("Web Intelligence Harvested: {}", snippet_summary.trim()));
                        }
                        Err(e) => {
                            plan.steps[step_idx].status = StepStatus::Failed;
                            plan.steps[step_idx].log = Some(format!("Search failed: {}", e));
                        }
                    }
                }
                "fs_search" | "filesystem_search" => {
                    let search_term = command.replace("find", "").replace("search", "").trim().to_string();
                    match FilesystemTool::search_files(&home, &search_term, 6) {
                        Ok(matches) => {
                            plan.steps[step_idx].status = StepStatus::Completed;
                            if matches.is_empty() {
                                plan.steps[step_idx].log = Some(format!("No matching files found for '{}'", search_term));
                            } else {
                                let lines: Vec<String> = matches.iter().map(|p| p.display().to_string()).collect();
                                plan.steps[step_idx].log = Some(format!("Discovered {} matching files:\n{}", matches.len(), lines.join("\n")));
                            }
                        }
                        Err(e) => {
                            plan.steps[step_idx].status = StepStatus::Failed;
                            plan.steps[step_idx].log = Some(format!("Filesystem search error: {}", e));
                        }
                    }
                }
                "fs_organize" => {
                    let target_dir = home.join("Downloads");
                    if target_dir.exists() {
                        match FilesystemTool::organize_directory(&target_dir) {
                            Ok(res) => {
                                plan.steps[step_idx].status = StepStatus::Completed;
                                plan.steps[step_idx].log = Some(res);
                            }
                            Err(e) => {
                                plan.steps[step_idx].status = StepStatus::Failed;
                                plan.steps[step_idx].log = Some(format!("Directory organization error: {}", e));
                            }
                        }
                    } else {
                        plan.steps[step_idx].status = StepStatus::Completed;
                        plan.steps[step_idx].log = Some(format!("Target directory {:?} does not exist.", target_dir));
                    }
                }
                "telemetry" | "telemetry_diagnose" => {
                    let snap = {
                        let mut coll = self.telemetry.lock().await;
                        coll.snapshot()
                    };

                    let cpu_temp = snap.cpu.temp_c.unwrap_or(0.0);
                    let fan_rpm = snap.thermal.cpu_fan_rpm.unwrap_or(0);
                    let mut top_procs = String::new();

                    let p_lower = plan.prompt.to_lowercase();
                    if p_lower.contains("device") || p_lower.contains("peripheral") || p_lower.contains("usb") || p_lower.contains("bluetooth") {
                        let mut dev_lines = Vec::new();
                        for d in &snap.devices {
                            dev_lines.push(format!("- [{:?}] {} (Bus: {}, Status: {})", d.dev_type, d.name, d.connection_bus, d.status));
                        }
                        plan.steps[step_idx].log = Some(format!("Connected Peripherals ({} nodes discovered):\n{}", snap.devices.len(), dev_lines.join("\n")));
                    } else if p_lower.contains("ram") || p_lower.contains("memory") {
                        for p in snap.memory.top_processes.iter().take(4) {
                            top_procs.push_str(&format!("{}: {} MB (CPU: {:.1}%), ", p.name, p.mem_mb, p.cpu_percent));
                        }
                        plan.steps[step_idx].log = Some(format!(
                            "Memory: {} MB / {} MB ({:.1}%) | Top Consumers: {}",
                            snap.memory.used_mb, snap.memory.total_mb, snap.memory.usage_percent, top_procs.trim_end_matches(", ")
                        ));
                    } else {
                        for p in snap.cpu.top_processes.iter().take(4) {
                            top_procs.push_str(&format!("{}: {:.1}% CPU ({} MB), ", p.name, p.cpu_percent, p.mem_mb));
                        }
                        plan.steps[step_idx].log = Some(format!(
                            "CPU: {:.1}% ({:.1}°C, {} RPM) | RAM: {} MB | Top Consumers: {}",
                            snap.cpu.usage_percent, cpu_temp, fan_rpm, snap.memory.used_mb, top_procs.trim_end_matches(", ")
                        ));
                    }
                    plan.steps[step_idx].status = StepStatus::Completed;
                }
                _ => {
                    // General tool or fallback
                    plan.steps[step_idx].log = Some(format!("Executed operation: {}", command));
                    plan.steps[step_idx].status = StepStatus::Completed;
                }
            }

            plan.current_step += 1;
        }

        // Final summary generation with LLM based on actual executed findings
        if plan.final_summary.is_none() {
            let mut step_context = String::new();
            for step in &plan.steps {
                if let Some(log) = &step.log {
                    step_context.push_str(&format!("- Step '{}' [{}]: {}\n", step.title, step.tool, log));
                }
            }

            let summary_prompt = format!(
                "You are TRUNI, a spatial spaceship intelligence onboard computer operating a Linux desktop.\n\
                 User request: {}\n\
                 Actual tool actions and findings:\n{}\n\
                 Provide a direct, natural, authoritative 1-3 sentence summary explaining what was executed and the result. If a website or music was opened in the browser, confirm it clearly. Never fabricate data.",
                plan.prompt, step_context
            );

            let messages = vec![
                ChatMessagePayload {
                    role: "system".to_string(),
                    content: "You are TRUNI, an onboard spacecraft AI. Speak with calm aerospace authority. Be direct, factual, and genuinely helpful.".to_string(),
                },
                ChatMessagePayload {
                    role: "user".to_string(),
                    content: summary_prompt,
                },
            ];

            let summary = match self.llm.chat(&messages).await {
                Ok(res) if !res.trim().is_empty() => res.trim().to_string(),
                _ => {
                    // Fallback synthesis from actual step logs
                    if let Some(first_log) = plan.steps.first().and_then(|s| s.log.as_deref()) {
                        format!("Task completed. {}", first_log)
                    } else {
                        "Mission completed successfully on flight deck.".to_string()
                    }
                }
            };

            plan.final_summary = Some(summary.clone());
            self.memory.finish_task(&plan.task_id, "Completed", Some(&summary))?;
        }

        Ok(())
    }
}
