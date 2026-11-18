use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OverlayState {
    /// State A: Small unobtrusive standby indicator
    Standby,
    /// State B: Cinematic spaceship system activation sequence
    Activation,
    /// State C: Central intelligence visual, live voice waveform, text input, conversation
    Conversation,
    /// State D: Task execution steps, tool activity, progress, errors, approval requests
    TaskExecution,
    /// State E: Interactive 3D spacecraft view with live laptop telemetry
    SpaceshipInspection,
}

impl Default for OverlayState {
    fn default() -> Self {
        OverlayState::Standby
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessSummary {
    pub pid: u32,
    pub name: String,
    pub cpu_percent: f32,
    pub mem_mb: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CpuTelemetry {
    pub usage_percent: f32,
    pub avg_freq_mhz: u64,
    pub temp_c: Option<f32>,
    pub core_count: usize,
    pub per_core_usage: Vec<f32>,
    pub top_processes: Vec<ProcessSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryTelemetry {
    pub total_mb: u64,
    pub used_mb: u64,
    pub available_mb: u64,
    pub usage_percent: f32,
    pub spd_temp_c: Option<f32>,
    pub top_processes: Vec<ProcessSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskTelemetry {
    pub name: String,
    pub mount_point: String,
    pub total_gb: f32,
    pub used_gb: f32,
    pub free_gb: f32,
    pub usage_percent: f32,
    pub temp_c: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterfaceTelemetry {
    pub name: String,
    pub is_up: bool,
    pub rx_bytes_per_sec: u64,
    pub tx_bytes_per_sec: u64,
    pub ip_addresses: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkTelemetry {
    pub total_rx_rate_bps: u64,
    pub total_tx_rate_bps: u64,
    pub interfaces: Vec<NetworkInterfaceTelemetry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalZone {
    pub name: String,
    pub temp_c: f32,
    pub high_c: Option<f32>,
    pub crit_c: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ThermalTelemetry {
    pub cpu_temp_c: Option<f32>,
    pub gpu_temp_c: Option<f32>,
    pub cpu_fan_rpm: Option<u32>,
    pub gpu_fan_rpm: Option<u32>,
    pub zones: Vec<ThermalZone>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PowerTelemetry {
    pub has_battery: bool,
    pub battery_percent: Option<f32>,
    pub is_charging: Option<bool>,
    pub power_watts: Option<f32>,
    pub voltage_volts: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum DeviceType {
    Usb,
    Bluetooth,
    Audio,
    Monitor,
    Input,
    Storage,
    Network,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectedDevice {
    pub id: String,
    pub name: String,
    pub dev_type: DeviceType,
    pub connection_bus: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TelemetrySnapshot {
    pub timestamp: DateTime<Utc>,
    pub hostname: String,
    pub kernel_version: String,
    pub cpu: CpuTelemetry,
    pub memory: MemoryTelemetry,
    pub disks: Vec<DiskTelemetry>,
    pub network: NetworkTelemetry,
    pub thermal: ThermalTelemetry,
    pub power: PowerTelemetry,
    pub devices: Vec<ConnectedDevice>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StepStatus {
    Pending,
    Running,
    Completed,
    Failed,
    AwaitingApproval,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStep {
    pub index: usize,
    pub title: String,
    pub tool: String,
    #[serde(default)]
    pub command: Option<String>,
    pub status: StepStatus,
    pub log: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub request_id: String,
    pub action_name: String,
    pub description: String,
    pub is_destructive: bool,
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub task_id: String,
    pub prompt: String,
    pub steps: Vec<TaskStep>,
    pub current_step: usize,
    pub pending_approval: Option<ApprovalRequest>,
    pub final_summary: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ChatRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
    pub timestamp: DateTime<Utc>,
}
