use crate::llm::{ChatMessagePayload, LlmClient};
use serde::{Deserialize, Serialize};
use truni_core::events::{StepStatus, TaskPlan, TaskStep};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    DismissOrClose,
    HardwareQuery,
    FilesystemAction,
    SystemAction,
    Development,
    WebResearch,
    GeneralConversation,
}

#[derive(Debug, Deserialize)]
struct LlmPlanResponse {
    steps: Vec<LlmStepItem>,
}

#[derive(Debug, Deserialize)]
struct LlmStepItem {
    title: String,
    tool: String,
    command: Option<String>,
}

pub struct Planner;

impl Planner {
    pub fn classify_intent(prompt: &str) -> Intent {
        let p = prompt.to_lowercase();
        if p.contains("handle it")
            || p.contains("handle this")
            || p.contains("i got this")
            || p.contains("close")
            || p.contains("dismiss")
            || p.contains("standby")
            || p.contains("go to sleep")
            || p.contains("nevermind")
            || p.contains("never mind")
            || p.contains("goodbye")
            || p.contains("bye")
            || p.contains("that's all")
            || p.contains("stop listening")
            || p.contains("cancel")
            || p.contains("hide overlay")
            || p.contains("hide")
            || p.contains("exit")
            || p.contains("quit")
        {
            Intent::DismissOrClose
        } else if p.contains("ram")
            || p.contains("cpu")
            || p.contains("gpu")
            || p.contains("temp")
            || p.contains("hot")
            || p.contains("fan")
            || p.contains("battery")
            || p.contains("hardware")
            || p.contains("consuming")
            || p.contains("telemetry")
        {
            Intent::HardwareQuery
        } else if p.contains("device")
            || p.contains("peripheral")
            || p.contains("usb")
            || p.contains("bluetooth")
            || p.contains("monitor")
            || p.contains("open")
            || p.contains("launch")
            || p.contains("kill")
            || p.contains("process")
            || p.contains("terminal")
            || p.contains("service")
        {
            Intent::SystemAction
        } else if p.contains("find")
            || p.contains("search file")
            || p.contains("organise")
            || p.contains("organize")
            || p.contains("download")
            || p.contains("duplicate")
            || p.contains("read file")
            || p.contains("document")
            || p.contains("notes")
            || p.contains("delete")
            || p.contains("remove")
        {
            Intent::FilesystemAction
        } else if p.contains("research")
            || p.contains("search web")
            || p.contains("latest")
            || p.contains("news")
            || p.contains("internet")
        {
            Intent::WebResearch
        } else if p.contains("bug")
            || p.contains("code")
            || p.contains("project")
            || p.contains("python")
            || p.contains("rust")
            || p.contains("develop")
            || p.contains("git")
            || p.contains("test")
        {
            Intent::Development
        } else {
            Intent::GeneralConversation
        }
    }

    pub fn create_plan(task_id: &str, prompt: &str) -> TaskPlan {
        let intent = Self::classify_intent(prompt);
        let p_lower = prompt.to_lowercase();

        let steps = match intent {
            Intent::DismissOrClose => vec![TaskStep {
                index: 0,
                title: "Disengage Flight Deck Overlay".to_string(),
                tool: "dismiss".to_string(),
                command: None,
                status: StepStatus::Pending,
                log: None,
            }],
            Intent::HardwareQuery => vec![
                TaskStep {
                    index: 0,
                    title: "Inspect System Sensors & Telemetry".to_string(),
                    tool: "telemetry".to_string(),
                    command: Some("snapshot".to_string()),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 1,
                    title: "Analyze Hardware Readings & Identify Root Causes".to_string(),
                    tool: "telemetry_diagnose".to_string(),
                    command: Some(prompt.to_string()),
                    status: StepStatus::Pending,
                    log: None,
                },
            ],
            Intent::FilesystemAction => {
                if p_lower.contains("organise") || p_lower.contains("organize") {
                    vec![TaskStep {
                        index: 0,
                        title: "Organize Downloads Directory by Category".to_string(),
                        tool: "fs_organize".to_string(),
                        command: Some("~/Downloads".to_string()),
                        status: StepStatus::Pending,
                        log: None,
                    }]
                } else {
                    let search_term = prompt.replace("find", "").replace("search", "").replace("my", "").replace("file", "").trim().to_string();
                    vec![
                        TaskStep {
                            index: 0,
                            title: format!("Search Local Filesystem for '{}'", search_term),
                            tool: "fs_search".to_string(),
                            command: Some(search_term),
                            status: StepStatus::Pending,
                            log: None,
                        },
                    ]
                }
            }
            Intent::WebResearch => vec![
                TaskStep {
                    index: 0,
                    title: format!("Execute Web Intelligence Query: '{}'", prompt),
                    tool: "web_search".to_string(),
                    command: Some(prompt.to_string()),
                    status: StepStatus::Pending,
                    log: None,
                },
            ],
            _ => {
                // If it asks to open or play something
                if p_lower.contains("open") || p_lower.contains("play") || p_lower.contains("google") || p_lower.contains("youtube") {
                    let url = if p_lower.contains("youtube") || p_lower.contains("play") || p_lower.contains("song") || p_lower.contains("music") {
                        let query = prompt.replace("open", "").replace("play", "").replace("the", "").replace("google", "").replace("and", "").trim().to_string();
                        format!("https://www.youtube.com/results?search_query={}", urlencoding::encode(&query))
                    } else if p_lower.contains("google") {
                        "https://www.google.com".to_string()
                    } else {
                        "https://www.google.com".to_string()
                    };

                    vec![TaskStep {
                        index: 0,
                        title: format!("Launch Desktop Browser: {}", url),
                        tool: "browser_open".to_string(),
                        command: Some(url),
                        status: StepStatus::Pending,
                        log: None,
                    }]
                } else {
                    vec![TaskStep {
                        index: 0,
                        title: format!("Execute System Agent Mission: '{}'", prompt),
                        tool: "bash".to_string(),
                        command: Some(prompt.to_string()),
                        status: StepStatus::Pending,
                        log: None,
                    }]
                }
            }
        };

        TaskPlan {
            task_id: task_id.to_string(),
            prompt: prompt.to_string(),
            steps,
            current_step: 0,
            pending_approval: None,
            final_summary: None,
        }
    }

    pub async fn create_plan_dynamic(llm: &LlmClient, task_id: &str, prompt: &str) -> TaskPlan {
        let intent = Self::classify_intent(prompt);

        // Fast path for immediate local dismissal
        if intent == Intent::DismissOrClose {
            return Self::create_plan(task_id, prompt);
        }

        // Fast path for pure hardware questions to avoid LLM hallucination of telemetry
        if intent == Intent::HardwareQuery {
            return Self::create_plan(task_id, prompt);
        }

        let system_prompt = "\
You are TRUNI, an autonomous Linux Wayland desktop AI operating agent.
Generate a concrete, executable JSON plan with 1 to 3 steps to complete the user's request.

Available tools:
- \"browser_open\": Opens a website, search, video, or song in the user's default browser via xdg-open.
  `command` MUST be a valid full URL (e.g. \"https://www.google.com\", \"https://www.youtube.com/results?search_query=...\", \"https://github.com\").
  For playing music or searching video, use a YouTube search URL.
- \"bash\": Run a shell command, launch a Linux desktop app, or execute a script.
  `command` MUST be the executable bash command (e.g. \"xdg-open ~\", \"python3 file.py\", \"ls -la\").
- \"web_search\": Search DuckDuckGo for facts or latest news.
  `command` is the search query.
- \"fs_search\": Search files in the user home directory.
  `command` is the filename or keyword.
- \"fs_organize\": Sort files in a folder into categories.
  `command` is the folder path (e.g. \"~/Downloads\").
- \"telemetry\": Query hardware sensors.
  `command` is \"all\", \"cpu\", \"ram\", or \"devices\".

Output ONLY a JSON object with this schema:
{\"steps\": [{\"title\": \"Description\", \"tool\": \"browser_open\"|\"bash\"|\"web_search\"|\"fs_search\"|\"fs_organize\"|\"telemetry\", \"command\": \"exact url or command\"}]}";

        let messages = vec![
            ChatMessagePayload {
                role: "system".to_string(),
                content: system_prompt.to_string(),
            },
            ChatMessagePayload {
                role: "user".to_string(),
                content: prompt.to_string(),
            },
        ];

        if let Ok(response) = llm.chat(&messages).await {
            if let Some(json_slice) = extract_json(&response) {
                if let Ok(parsed) = serde_json::from_str::<LlmPlanResponse>(json_slice) {
                    if !parsed.steps.is_empty() {
                        let mut steps = Vec::new();
                        for (i, s) in parsed.steps.into_iter().enumerate() {
                            steps.push(TaskStep {
                                index: i,
                                title: s.title,
                                tool: s.tool,
                                command: s.command,
                                status: StepStatus::Pending,
                                log: None,
                            });
                        }
                        return TaskPlan {
                            task_id: task_id.to_string(),
                            prompt: prompt.to_string(),
                            steps,
                            current_step: 0,
                            pending_approval: None,
                            final_summary: None,
                        };
                    }
                }
            }
        }

        // Fallback to static plan if LLM is unavailable
        Self::create_plan(task_id, prompt)
    }
}

fn extract_json(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end > start {
        Some(&raw[start..=end])
    } else {
        None
    }
}
