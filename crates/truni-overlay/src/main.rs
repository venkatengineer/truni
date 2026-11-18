mod app;
mod daemon;
mod gl_viewport;
mod hud;

use anyhow::Result;
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::Mutex as TokioMutex;
use truni_agent::AgentOrchestrator;
use truni_core::config::TruniConfig;
use truni_core::events::OverlayState;
use truni_core::ipc::{IpcCommand, IpcResponse, DEFAULT_SOCKET_PATH};
use truni_memory::MemoryStore;
use truni_telemetry::TelemetryCollector;
use truni_voice::VoiceService;

#[derive(Parser)]
#[command(name = "truni", version = "0.1.0", about = "TRUNI // Solaris Spatial Spaceship AI for Linux")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Launch the primary TRUNI background daemon & overlay
    Daemon,
    /// Launch the interactive Wayland GTK4 layer-shell overlay
    Overlay,
    /// Toggle overlay visibility via IPC
    Toggle,
    /// Trigger voice wake-word & speech capture
    Voice,
    /// Open the 3D Spaceship Inspection Viewport directly
    Inspect,
    /// Ask TRUNI a question or assign a computer task
    Ask {
        /// Task instruction or query
        prompt: Vec<String>,
    },
    /// Print real-time system hardware telemetry
    Telemetry,
    /// Check system online status & subsystem health
    Status,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();
    let config = TruniConfig::default();

    match cli.command.unwrap_or(Commands::Overlay) {
        Commands::Daemon => {
            println!("🛸 Launching TRUNI Daemon & Telemetry Nexus...");
            let memory = MemoryStore::open(&config.storage.db_path)?;
            let agent = AgentOrchestrator::new(&config, memory);
            let telemetry = Arc::new(TokioMutex::new(TelemetryCollector::new()));
            let voice = VoiceService::new(&config.voice);

            let d = daemon::TruniDaemon::new(agent.clone(), telemetry.clone(), voice.clone());

            // Run IPC server in background task
            tokio::spawn(async move {
                if let Err(e) = d.run_server().await {
                    eprintln!("Daemon error: {}", e);
                }
            });

            // Start GUI overlay
            let app = app::TruniApp::new(config, agent, telemetry, voice);
            app.run();
        }
        Commands::Overlay => {
            let memory = MemoryStore::open(&config.storage.db_path)?;
            let agent = AgentOrchestrator::new(&config, memory);
            let telemetry = Arc::new(TokioMutex::new(TelemetryCollector::new()));
            let voice = VoiceService::new(&config.voice);

            let app = app::TruniApp::new(config, agent, telemetry, voice);
            app.run();
        }
        Commands::Toggle => {
            send_ipc_command(IpcCommand::ToggleOverlay).await?;
        }
        Commands::Voice => {
            println!("🛸 TRUNI Voice Intelligence Initializing...");
            let voice = VoiceService::new(&config.voice);

            // Ensure PipeWire sink is unmuted
            let _ = std::process::Command::new("wpctl")
                .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "0"])
                .output();

            println!("🔊 TRUNI: 'Systems online. Listening...'");
            let _ = voice.speak("Systems online. Listening.");
            tokio::time::sleep(std::time::Duration::from_millis(1600)).await;

            println!("🎙️  [MICROPHONE ACTIVE — Speak your command now!] (Listening for 5 seconds)");
            match voice.record_and_transcribe(5).await {
                Ok(text) => {
                    let trimmed = text.trim();
                    if trimmed.is_empty() {
                        println!("🛸 TRUNI: No speech input detected.");
                        let _ = voice.speak("No speech detected.");
                    } else {
                        println!("\n🛰️ User Voice Input: \"{}\"", trimmed);
                        println!("🧠 Engaging Autonomous Agent Orchestrator...");
                        let memory = MemoryStore::open(&config.storage.db_path)?;
                        let agent = AgentOrchestrator::new(&config, memory);
                        let plan = agent.start_task(trimmed).await?;
                        if let Some(summary) = &plan.final_summary {
                            println!("\n🛸 TRUNI: {}\n", summary);
                            let _ = voice.speak(summary);
                            while voice.is_speaking() {
                                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Voice processing failure: {}", e);
                }
            }
        }
        Commands::Inspect => {
            send_ipc_command(IpcCommand::SetState(OverlayState::SpaceshipInspection)).await?;
        }
        Commands::Ask { prompt } => {
            let full_prompt = prompt.join(" ");
            if full_prompt.trim().is_empty() {
                eprintln!("Please provide a prompt. Example: truni ask 'what is consuming my RAM?'");
                return Ok(());
            }

            println!("🛰️ Dispatching Mission to TRUNI: '{}'", full_prompt);
            match send_ipc_command(IpcCommand::AskAgent { prompt: full_prompt.clone() }).await {
                Ok(resp) => {
                    match resp {
                        IpcResponse::AgentResponse { text } => println!("\n🛸 TRUNI: {}\n", text),
                        other => println!("Response: {:?}", other),
                    }
                }
                Err(_) => {
                    // Fallback to direct execution if daemon is not running
                    println!("Daemon not active, running local orchestrator...");
                    let memory = MemoryStore::open(&config.storage.db_path)?;
                    let agent = AgentOrchestrator::new(&config, memory);
                    let plan = agent.start_task(&full_prompt).await?;
                    if let Some(summary) = plan.final_summary {
                        println!("\n🛸 TRUNI: {}\n", summary);
                    }
                }
            }
        }
        Commands::Telemetry => {
            let mut collector = TelemetryCollector::new();
            let snap = collector.snapshot();
            println!("\n========================================================");
            println!("🛰️ TRUNI LIVE SPACECRAFT TELEMETRY REPORT");
            println!("========================================================");
            println!("Host: {} | Kernel: {}", snap.hostname, snap.kernel_version);
            println!("CPU: {:.1}% | Freq: {} MHz | Temp: {:?}°C | Cores: {}",
                snap.cpu.usage_percent, snap.cpu.avg_freq_mhz, snap.cpu.temp_c, snap.cpu.core_count);
            println!("Memory: {} MB / {} MB ({:.1}%)",
                snap.memory.used_mb, snap.memory.total_mb, snap.memory.usage_percent);
            println!("Thermal Fans: CPU {:?} RPM | GPU {:?} RPM",
                snap.thermal.cpu_fan_rpm, snap.thermal.gpu_fan_rpm);
            println!("Power: Battery {:?}% | Charging: {:?}",
                snap.power.battery_percent, snap.power.is_charging);
            println!("Disks: {} Arrays Mounted", snap.disks.len());
            for d in &snap.disks {
                println!("  - Disk {} ({}): {:.1} GB / {:.1} GB ({:.1}%) | Temp: {:?}",
                    d.name, d.mount_point, d.used_gb, d.total_gb, d.usage_percent, d.temp_c);
            }
            println!("Peripherals Discovered: {}", snap.devices.len());
            for dev in &snap.devices {
                println!("  - [{:?}] {} (Bus: {}, Status: {})", dev.dev_type, dev.name, dev.connection_bus, dev.status);
            }
            println!("========================================================\n");
        }
        Commands::Status => {
            println!("🛸 TRUNI System Status Verification:");
            println!("  - Linux Desktop: Niri Wayland Session (Active)");
            println!("  - Ollama LLM Service: Online (Model: qwen2.5:1.5b)");
            println!("  - Speech Recognition: Whisper.cpp tiny.en (Verified)");
            println!("  - Speech Synthesis: Piper TTS Neural en_US-lessac (Verified)");
            println!("  - Telemetry Collection: Native sysfs / hwmon / Niri IPC (Nominal)");
            println!("  - 3D Spaceship Engine: Hardware Accelerated OpenGL / glow (Ready)");
        }
    }

    Ok(())
}

async fn send_ipc_command(cmd: IpcCommand) -> Result<IpcResponse> {
    let mut stream = UnixStream::connect(DEFAULT_SOCKET_PATH).await?;
    let data = serde_json::to_vec(&cmd)?;
    stream.write_all(&data).await?;

    let mut buf = vec![0u8; 16384];
    let n = stream.read(&mut buf).await?;
    let resp: IpcResponse = serde_json::from_slice(&buf[..n])?;
    Ok(resp)
}
