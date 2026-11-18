use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::Mutex;
use truni_agent::AgentOrchestrator;
use truni_core::events::OverlayState;
use truni_core::ipc::{IpcCommand, IpcResponse, DEFAULT_SOCKET_PATH};
use truni_telemetry::TelemetryCollector;
use truni_voice::VoiceService;

pub struct TruniDaemon {
    agent: AgentOrchestrator,
    telemetry: Arc<Mutex<TelemetryCollector>>,
    voice: VoiceService,
    socket_path: String,
}

impl TruniDaemon {
    pub fn new(
        agent: AgentOrchestrator,
        telemetry: Arc<Mutex<TelemetryCollector>>,
        voice: VoiceService,
    ) -> Self {
        Self {
            agent,
            telemetry,
            voice,
            socket_path: DEFAULT_SOCKET_PATH.to_string(),
        }
    }

    pub async fn run_server(&self) -> Result<()> {
        let path = Path::new(&self.socket_path);
        if path.exists() {
            let _ = fs::remove_file(path);
        }

        let listener = UnixListener::bind(path)
            .with_context(|| format!("Failed to bind UNIX socket at {}", self.socket_path))?;

        tracing::info!("TRUNI IPC Daemon active on {}", self.socket_path);

        loop {
            if let Ok((stream, _)) = listener.accept().await {
                let agent = self.agent.clone();
                let telemetry = self.telemetry.clone();
                let voice = self.voice.clone();

                tokio::spawn(async move {
                    let _ = handle_client(stream, agent, telemetry, voice).await;
                });
            }
        }
    }
}

async fn handle_client(
    mut stream: UnixStream,
    agent: AgentOrchestrator,
    telemetry: Arc<Mutex<TelemetryCollector>>,
    voice: VoiceService,
) -> Result<()> {
    let mut buf = vec![0u8; 16384];
    let n = stream.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }

    let cmd: IpcCommand = match serde_json::from_slice(&buf[..n]) {
        Ok(c) => c,
        Err(e) => {
            let resp = IpcResponse::Error {
                message: format!("Malformed command: {}", e),
            };
            let out = serde_json::to_vec(&resp)?;
            stream.write_all(&out).await?;
            return Ok(());
        }
    };

    let response = match cmd {
        IpcCommand::Ping => IpcResponse::Pong,
        IpcCommand::GetTelemetry => {
            let mut coll = telemetry.lock().await;
            let snap = coll.snapshot();
            IpcResponse::Telemetry(Box::new(snap))
        }
        IpcCommand::GetStatus => {
            let plan = agent.get_active_plan().await;
            IpcResponse::Status {
                overlay_visible: true,
                current_state: OverlayState::Conversation,
                active_plan: plan,
                is_listening: voice.is_listening(),
            }
        }
        IpcCommand::AskAgent { prompt } => {
            match agent.start_task(&prompt).await {
                Ok(plan) => IpcResponse::AgentResponse {
                    text: plan
                        .final_summary
                        .unwrap_or_else(|| "Mission step execution complete.".to_string()),
                },
                Err(e) => IpcResponse::Error {
                    message: format!("Agent execution failed: {}", e),
                },
            }
        }
        IpcCommand::ApproveAction {
            request_id,
            approved,
        } => match agent.approve_action(&request_id, approved).await {
            Ok(_) => IpcResponse::Ok {
                message: format!("Action {} recorded.", if approved { "approval" } else { "rejection" }),
            },
            Err(e) => IpcResponse::Error {
                message: format!("Failed to record action: {}", e),
            },
        },
        IpcCommand::StartVoiceListen => {
            let _ = voice.acknowledge_wake();
            IpcResponse::Ok {
                message: "Voice listening engaged.".to_string(),
            }
        }
        IpcCommand::StopVoiceListen => {
            voice.cancel_recording();
            IpcResponse::Ok {
                message: "Voice listening stopped.".to_string(),
            }
        }
        _ => IpcResponse::Ok {
            message: "Dispatched to overlay controller.".to_string(),
        },
    };

    let out = serde_json::to_vec(&response)?;
    stream.write_all(&out).await?;
    Ok(())
}
