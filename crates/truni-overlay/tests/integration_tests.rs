use anyhow::Result;
use tempfile::tempdir;
use truni_agent::planner::{Intent, Planner};
use truni_agent::AgentOrchestrator;
use truni_core::config::TruniConfig;
use truni_core::events::StepStatus;
use truni_memory::MemoryStore;
use truni_telemetry::TelemetryCollector;
use truni_tools::bash::BashTool;
use truni_tools::filesystem::FilesystemTool;
use truni_tools::security::{RiskLevel, SecurityManager};

#[test]
fn test_telemetry_live_hardware_readings() {
    let mut collector = TelemetryCollector::new();
    let snap = collector.snapshot();

    assert!(!snap.hostname.is_empty());
    assert!(!snap.kernel_version.is_empty());
    assert!(snap.cpu.core_count >= 1);
    assert!(snap.memory.total_mb > 0);
    assert!(!snap.disks.is_empty());
    assert!(!snap.devices.is_empty());
}

#[test]
fn test_memory_store_audit_and_retrieval() -> Result<()> {
    let store = MemoryStore::open_in_memory()?;
    store.set_memory("starship_callsign", "config", "SOLARIS-NX01")?;

    let item = store.get_memory("starship_callsign")?.expect("Found");
    assert_eq!(item.content, "SOLARIS-NX01");

    store.log_audit(
        Some("task_99"),
        "bash",
        "ls -la /tmp",
        false,
        true,
        Some("total 0"),
    )?;

    Ok(())
}

#[test]
fn test_security_manager_assessments() {
    let sec = SecurityManager::new();

    // Safe commands
    assert_eq!(sec.assess_command("ls -la"), RiskLevel::Safe);
    assert_eq!(sec.assess_command("echo hello"), RiskLevel::Safe);

    // Destructive commands requiring approval
    assert_eq!(sec.assess_command("rm -rf /tmp/test"), RiskLevel::RequiresApproval);
    assert_eq!(sec.assess_command("pacman -R package"), RiskLevel::RequiresApproval);
    assert_eq!(sec.assess_command("reboot"), RiskLevel::RequiresApproval);

    // Forbidden commands
    assert_eq!(sec.assess_command("sudo rm -rf /"), RiskLevel::Forbidden);
}

#[test]
fn test_filesystem_tools() -> Result<()> {
    let tmp = tempdir()?;
    let file1 = tmp.path().join("college_networking_notes.txt");
    FilesystemTool::write_file(&file1, "TCP/IP stack, BGP routing, Subnet masks")?;

    let content = FilesystemTool::read_file(&file1, 10)?;
    assert!(content.contains("TCP/IP stack"));

    let matches = FilesystemTool::search_files(tmp.path(), "networking", 5)?;
    assert_eq!(matches.len(), 1);

    Ok(())
}

#[tokio::test]
async fn test_bash_execution() -> Result<()> {
    let res = BashTool::execute("echo 'TRUNI online'", 5).await?;
    assert!(res.success);
    assert!(res.stdout.contains("TRUNI online"));
    Ok(())
}

#[test]
fn test_planner_intent_classification() {
    assert_eq!(Planner::classify_intent("okay truni i can handle it"), Intent::DismissOrClose);
    assert_eq!(Planner::classify_intent("i got this, thanks"), Intent::DismissOrClose);
    assert_eq!(Planner::classify_intent("truni close"), Intent::DismissOrClose);
    assert_eq!(Planner::classify_intent("dismiss overlay"), Intent::DismissOrClose);
    assert_eq!(Planner::classify_intent("nevermind truni"), Intent::DismissOrClose);
    assert_eq!(Planner::classify_intent("what is consuming my RAM?"), Intent::HardwareQuery);
    assert_eq!(Planner::classify_intent("why is my laptop getting hot?"), Intent::HardwareQuery);
    assert_eq!(Planner::classify_intent("find my college networking notes"), Intent::FilesystemAction);
    assert_eq!(Planner::classify_intent("organise my Downloads folder"), Intent::FilesystemAction);
    assert_eq!(Planner::classify_intent("research the latest developments in quantum computing"), Intent::WebResearch);
    assert_eq!(Planner::classify_intent("show me all connected devices"), Intent::SystemAction);
}

#[tokio::test]
async fn test_nlp_dismiss_workflow() -> Result<()> {
    let config = TruniConfig::default();
    let memory = MemoryStore::open_in_memory()?;
    let agent = AgentOrchestrator::new(&config, memory);

    let plan = agent.start_task("okay truni i can handle it").await?;
    assert_eq!(plan.steps.len(), 1);
    assert_eq!(plan.steps[0].status, StepStatus::Completed);
    assert!(plan.final_summary.as_ref().unwrap().contains("Standing by on flight deck"));

    Ok(())
}

#[tokio::test]
async fn test_agent_approval_workflow() -> Result<()> {
    let config = TruniConfig::default();
    let memory = MemoryStore::open_in_memory()?;
    let agent = AgentOrchestrator::new(&config, memory);

    // Task requesting deletion should trigger approval gate
    let plan = agent.start_task("delete old cache files with rm -rf /tmp/cache").await?;
    assert!(plan.pending_approval.is_some());
    let step_awaiting = plan.steps.iter().find(|s| s.status == StepStatus::AwaitingApproval);
    assert!(step_awaiting.is_some());

    // Denying the approval
    let req_id = plan.pending_approval.unwrap().request_id;
    let updated = agent.approve_action(&req_id, false).await?.expect("Updated plan");
    let step_failed = updated.steps.iter().find(|s| s.status == StepStatus::Failed);
    assert!(step_failed.is_some());
    assert!(updated.final_summary.unwrap().contains("halted"));

    Ok(())
}
