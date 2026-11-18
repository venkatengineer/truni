use crate::events::{OverlayState, TaskPlan, TelemetrySnapshot};
use serde::{Deserialize, Serialize};

pub const DEFAULT_SOCKET_PATH: &str = "/tmp/truni_ipc.sock";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcCommand {
    Ping,
    ToggleOverlay,
    SetState(OverlayState),
    StartVoiceListen,
    StopVoiceListen,
    SendVoiceText(String),
    AskAgent { prompt: String },
    ApproveAction { request_id: String, approved: bool },
    GetTelemetry,
    GetStatus,
    Shutdown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum IpcResponse {
    Pong,
    Ok { message: String },
    Telemetry(Box<TelemetrySnapshot>),
    Status {
        overlay_visible: bool,
        current_state: OverlayState,
        active_plan: Option<TaskPlan>,
        is_listening: bool,
    },
    AgentResponse { text: String },
    Error { message: String },
}
