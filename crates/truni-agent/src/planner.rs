use serde::{Deserialize, Serialize};
use truni_core::events::{StepStatus, TaskPlan, TaskStep};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Intent {
    DismissOrClose,
    HardwareQuery,
    AppLaunch,
    FilesystemAction,
    WebExplicit,
    SystemCommand,
    DirectConversation,
}

pub struct Planner;

impl Planner {
    pub fn classify_intent(prompt: &str) -> Intent {
        let p = prompt.to_lowercase();
        let trimmed = p.trim();

        // 1. Natural Language Dismissal
        if trimmed.contains("handle it")
            || trimmed.contains("handle this")
            || trimmed.contains("i got this")
            || trimmed.contains("close")
            || trimmed.contains("dismiss")
            || trimmed.contains("standby")
            || trimmed.contains("go to sleep")
            || trimmed.contains("nevermind")
            || trimmed.contains("never mind")
            || trimmed.contains("goodbye")
            || trimmed.contains("bye")
            || trimmed.contains("stop listening")
            || trimmed.contains("cancel")
            || trimmed.contains("hide overlay")
            || trimmed == "hide"
            || trimmed == "exit"
            || trimmed == "quit"
        {
            return Intent::DismissOrClose;
        }

        // 2. Hardware / Telemetry inspection
        if trimmed.contains("ram")
            || trimmed.contains("memory")
            || trimmed.contains("cpu")
            || trimmed.contains("gpu")
            || trimmed.contains("temp")
            || trimmed.contains("hot")
            || trimmed.contains("fan")
            || trimmed.contains("battery")
            || trimmed.contains("consuming")
            || trimmed.contains("telemetry")
            || trimmed.contains("hardware")
            || trimmed.contains("connected device")
            || trimmed.contains("usb device")
            || trimmed.contains("bluetooth device")
        {
            return Intent::HardwareQuery;
        }

        // 3. Explicit Web Browsing / Online Media (ONLY when explicitly targeting web or online URL)
        if trimmed.contains("youtube.com")
            || trimmed.contains("google.com")
            || trimmed.contains("github.com")
            || trimmed.contains("reddit.com")
            || trimmed.contains("wikipedia.org")
            || trimmed.contains("http://")
            || trimmed.contains("https://")
            || trimmed.contains("open google")
            || trimmed.contains("open youtube")
            || trimmed.contains("search google")
            || trimmed.contains("google search")
            || trimmed.contains("search web")
            || trimmed.contains("search the web")
            || trimmed.contains("browse to")
            || trimmed.contains("on youtube")
            || trimmed.contains("open website")
            || (trimmed.contains("play") && (trimmed.contains("song") || trimmed.contains("music") || trimmed.contains("video")))
        {
            return Intent::WebExplicit;
        }

        // 4. Local Desktop Application Launching (MUST NOT open web browser!)
        if (trimmed.contains("open ") || trimmed.contains("launch ") || trimmed.contains("start "))
            && (trimmed.contains("terminal")
                || trimmed.contains("console")
                || trimmed.contains("kitty")
                || trimmed.contains("alacritty")
                || trimmed.contains("code")
                || trimmed.contains("editor")
                || trimmed.contains("ide")
                || trimmed.contains("development")
                || trimmed.contains("file manager")
                || trimmed.contains("files")
                || trimmed.contains("calculator")
                || trimmed.contains("browser")
                || trimmed.contains("settings"))
        {
            return Intent::AppLaunch;
        }

        // 5. Local Filesystem Actions
        if trimmed.contains("find my")
            || trimmed.contains("find file")
            || trimmed.contains("search file")
            || trimmed.contains("organise")
            || trimmed.contains("organize")
            || trimmed.contains("downloads folder")
            || trimmed.contains("delete file")
            || trimmed.contains("read file")
            || trimmed.contains("duplicate file")
        {
            return Intent::FilesystemAction;
        }

        // 6. Direct Conversation / Knowledge / Coding / Questions (Default!)
        Intent::DirectConversation
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
                    title: "Sample Hardware Telemetry Sensors".to_string(),
                    tool: "telemetry".to_string(),
                    command: Some(prompt.to_string()),
                    status: StepStatus::Pending,
                    log: None,
                },
            ],
            Intent::AppLaunch => {
                let app_cmd = if p_lower.contains("terminal") || p_lower.contains("console") || p_lower.contains("kitty") || p_lower.contains("alacritty") {
                    "kitty"
                } else if p_lower.contains("code") || p_lower.contains("editor") || p_lower.contains("ide") || p_lower.contains("development") {
                    "code"
                } else if p_lower.contains("files") || p_lower.contains("file manager") {
                    "nautilus"
                } else if p_lower.contains("browser") {
                    "xdg-open https://www.google.com"
                } else {
                    "kitty"
                };

                vec![TaskStep {
                    index: 0,
                    title: format!("Launch Desktop Application: {}", app_cmd),
                    tool: "app_launch".to_string(),
                    command: Some(app_cmd.to_string()),
                    status: StepStatus::Pending,
                    log: None,
                }]
            }
            Intent::WebExplicit => {
                let url = if p_lower.contains("youtube") || p_lower.contains("song") || p_lower.contains("music") || p_lower.contains("play") {
                    let query = prompt.replace("open", "").replace("play", "").replace("the", "").replace("youtube", "").replace("google", "").replace("on", "").trim().to_string();
                    format!("https://www.youtube.com/results?search_query={}", urlencoding::encode(&query))
                } else if p_lower.contains("github") {
                    "https://github.com".to_string()
                } else if p_lower.contains("reddit") {
                    "https://reddit.com".to_string()
                } else {
                    "https://www.google.com".to_string()
                };

                vec![TaskStep {
                    index: 0,
                    title: format!("Navigate Web: {}", url),
                    tool: "web_open".to_string(),
                    command: Some(url),
                    status: StepStatus::Pending,
                    log: None,
                }]
            }
            Intent::FilesystemAction => {
                if p_lower.contains("organise") || p_lower.contains("organize") {
                    vec![TaskStep {
                        index: 0,
                        title: "Organize Downloads Directory".to_string(),
                        tool: "fs_organize".to_string(),
                        command: Some("~/Downloads".to_string()),
                        status: StepStatus::Pending,
                        log: None,
                    }]
                } else {
                    let search_term = prompt.replace("find", "").replace("search", "").replace("my", "").replace("file", "").trim().to_string();
                    vec![TaskStep {
                        index: 0,
                        title: format!("Search Filesystem for '{}'", search_term),
                        tool: "fs_search".to_string(),
                        command: Some(search_term),
                        status: StepStatus::Pending,
                        log: None,
                    }]
                }
            }
            Intent::SystemCommand => vec![TaskStep {
                index: 0,
                title: format!("Execute Command: {}", prompt),
                tool: "bash".to_string(),
                command: Some(prompt.to_string()),
                status: StepStatus::Pending,
                log: None,
            }],
            Intent::DirectConversation => vec![TaskStep {
                index: 0,
                title: "Synthesize Spacecraft Intelligence Briefing".to_string(),
                tool: "direct_ai".to_string(),
                command: Some(prompt.to_string()),
                status: StepStatus::Pending,
                log: None,
            }],
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
