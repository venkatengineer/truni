use serde::{Deserialize, Serialize};
use truni_core::events::{StepStatus, TaskPlan, TaskStep};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    HardwareQuery,
    FilesystemAction,
    SystemAction,
    Development,
    WebResearch,
    GeneralConversation,
}

pub struct Planner;

impl Planner {
    pub fn classify_intent(prompt: &str) -> Intent {
        let p = prompt.to_lowercase();
        if p.contains("ram")
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
        let steps = match intent {
            Intent::HardwareQuery => vec![
                TaskStep {
                    index: 0,
                    title: "Inspect System Sensors & Telemetry".to_string(),
                    tool: "telemetry".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 1,
                    title: "Analyze Hardware Readings & Identify Root Causes".to_string(),
                    tool: "analysis".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 2,
                    title: "Synthesize Spacecraft Diagnostic Report".to_string(),
                    tool: "report".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
            ],
            Intent::FilesystemAction => vec![
                TaskStep {
                    index: 0,
                    title: "Locate Target Files & Directories".to_string(),
                    tool: "filesystem_search".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 1,
                    title: "Perform File Operation (Read/Organize/Extract)".to_string(),
                    tool: "filesystem_action".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 2,
                    title: "Verify File Integrity & Outcome".to_string(),
                    tool: "verification".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
            ],
            Intent::WebResearch => vec![
                TaskStep {
                    index: 0,
                    title: "Execute Multi-Source Web Query".to_string(),
                    tool: "browser_search".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 1,
                    title: "Extract & Cross-Reference Key Findings".to_string(),
                    tool: "research_extract".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 2,
                    title: "Compile Synthesized Intelligence Brief".to_string(),
                    tool: "report".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
            ],
            Intent::Development => vec![
                TaskStep {
                    index: 0,
                    title: "Scan Project Architecture & Sources".to_string(),
                    tool: "codebase_scan".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 1,
                    title: "Execute Implementation / Diagnostic Steps".to_string(),
                    tool: "dev_action".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 2,
                    title: "Verify Compilation & Test Execution".to_string(),
                    tool: "verification".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
            ],
            Intent::SystemAction => vec![
                TaskStep {
                    index: 0,
                    title: "Query Linux System / Hardware Subsystem".to_string(),
                    tool: "system_query".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 1,
                    title: "Execute Controlled Command".to_string(),
                    tool: "bash_exec".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 2,
                    title: "Verify Subsystem Status".to_string(),
                    tool: "verification".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
            ],
            Intent::GeneralConversation => vec![
                TaskStep {
                    index: 0,
                    title: "Retrieve Persistent Spacecraft Memory".to_string(),
                    tool: "memory_lookup".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
                TaskStep {
                    index: 1,
                    title: "Synthesize Intelligent Response".to_string(),
                    tool: "llm_generate".to_string(),
                    status: StepStatus::Pending,
                    log: None,
                },
            ],
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
}
