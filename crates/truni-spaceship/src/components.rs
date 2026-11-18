use glam::Vec3;
use serde::{Deserialize, Serialize};
use truni_core::events::TelemetrySnapshot;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ComponentId {
    ChassisHull,
    CpuReactor,
    MemoryBanks,
    NvmeStorage,
    GpuCore,
    CoolingTurbines,
    NetworkAntennas,
    PowerCell,
    PeripheralSatellite(String),
}

#[derive(Debug, Clone)]
pub struct SpaceshipComponent {
    pub id: ComponentId,
    pub name: String,
    pub role: String,
    pub position: Vec3,
    pub size: Vec3,
    pub base_color: [f32; 4],
    pub emissive_color: [f32; 4],
    pub emissive_intensity: f32,
    pub is_selected: bool,
    pub telemetry_details: Vec<String>,
}

impl SpaceshipComponent {
    pub fn update_from_telemetry(&mut self, snap: &TelemetrySnapshot, anim_time: f32) {
        match &self.id {
            ComponentId::ChassisHull => {
                self.telemetry_details = vec![
                    format!("Host: {}", snap.hostname),
                    format!("Architecture: Linux {}", snap.kernel_version),
                    "Primary Frame: Aerospace Alloy Chassis".to_string(),
                ];
            }
            ComponentId::CpuReactor => {
                let usage = snap.cpu.usage_percent;
                let temp = snap.cpu.temp_c.unwrap_or(0.0);
                self.emissive_intensity = 0.3 + (usage / 100.0) * 0.7 + (anim_time * 3.0).sin().abs() * 0.2;

                // Color transitions from emerald cyan to intense amber/red under high temp
                if temp > 80.0 {
                    self.emissive_color = [1.0, 0.25, 0.1, 1.0]; // Hot red/orange
                } else if temp > 65.0 {
                    self.emissive_color = [1.0, 0.7, 0.1, 1.0]; // Amber
                } else {
                    self.emissive_color = [0.0, 0.9, 0.7, 1.0]; // Emerald cyan
                }

                self.telemetry_details = vec![
                    format!("Cores: {} Cores (16 Threads)", snap.cpu.core_count),
                    format!("Utilization: {:.1}%", usage),
                    format!("Average Clock: {} MHz", snap.cpu.avg_freq_mhz),
                    format!("Core Temp: {:.1}°C", temp),
                ];
                if let Some(top) = snap.cpu.top_processes.first() {
                    self.telemetry_details.push(format!("Top Load: {} ({:.1}%)", top.name, top.cpu_percent));
                }
            }
            ComponentId::MemoryBanks => {
                let mem_percent = snap.memory.usage_percent;
                self.emissive_intensity = 0.4 + (mem_percent / 100.0) * 0.6;
                self.telemetry_details = vec![
                    format!("Allocated: {} MB / {} MB", snap.memory.used_mb, snap.memory.total_mb),
                    format!("Available: {} MB", snap.memory.available_mb),
                    format!("Utilization: {:.1}%", mem_percent),
                ];
                if let Some(t) = snap.memory.spd_temp_c {
                    self.telemetry_details.push(format!("SPD Sensor: {:.1}°C", t));
                }
                if let Some(top) = snap.memory.top_processes.first() {
                    self.telemetry_details.push(format!("Top Consumer: {} ({} MB)", top.name, top.mem_mb));
                }
            }
            ComponentId::NvmeStorage => {
                let primary = snap.disks.first();
                let used_gb = primary.map(|d| d.used_gb).unwrap_or(0.0);
                let total_gb = primary.map(|d| d.total_gb).unwrap_or(0.0);
                let usage_pct = primary.map(|d| d.usage_percent).unwrap_or(0.0);
                let temp = primary.and_then(|d| d.temp_c).unwrap_or(45.0);

                self.emissive_intensity = 0.5 + (anim_time * 2.0).sin().abs() * 0.3;
                self.telemetry_details = vec![
                    format!("Storage Arrays: {} Disks", snap.disks.len()),
                    format!("Root NVMe: {:.1} GB / {:.1} GB ({:.1}%)", used_gb, total_gb, usage_pct),
                    format!("NVMe Temp: {:.1}°C", temp),
                ];
            }
            ComponentId::GpuCore => {
                let gpu_temp = snap.thermal.gpu_temp_c.unwrap_or(50.0);
                self.telemetry_details = vec![
                    "GPU: NVIDIA RTX 5050 Mobile / Intel Raptor".to_string(),
                    format!("Thermal Core: {:.1}°C", gpu_temp),
                    "VRAM: 8151 MB Dedicated".to_string(),
                ];
            }
            ComponentId::CoolingTurbines => {
                let cpu_fan = snap.thermal.cpu_fan_rpm.unwrap_or(0);
                let gpu_fan = snap.thermal.gpu_fan_rpm.unwrap_or(0);
                self.emissive_intensity = 0.6 + ((cpu_fan as f32 / 6000.0).clamp(0.0, 1.0)) * 0.4;
                self.telemetry_details = vec![
                    format!("CPU Turbine: {} RPM", cpu_fan),
                    format!("GPU Turbine: {} RPM", gpu_fan),
                    "Cooling Manifold: Active Dual Impellers".to_string(),
                ];
            }
            ComponentId::NetworkAntennas => {
                let rx_kb = snap.network.total_rx_rate_bps / 1024;
                let tx_kb = snap.network.total_tx_rate_bps / 1024;
                self.emissive_intensity = 0.3 + ((rx_kb + tx_kb) as f32 / 500.0).clamp(0.1, 0.7);
                self.telemetry_details = vec![
                    format!("Downlink Rate: {} KB/s", rx_kb),
                    format!("Uplink Rate: {} KB/s", tx_kb),
                    format!("Active Interfaces: {}", snap.network.interfaces.len()),
                ];
            }
            ComponentId::PowerCell => {
                let pct = snap.power.battery_percent.unwrap_or(100.0);
                let charging = snap.power.is_charging.unwrap_or(false);
                let v = snap.power.voltage_volts.unwrap_or(13.3);
                self.telemetry_details = vec![
                    format!("Charge Level: {:.1}%", pct),
                    format!("Status: {}", if charging { "Charging" } else { "Discharging / AC" }),
                    format!("Potential: {:.2} V", v),
                ];
            }
            ComponentId::PeripheralSatellite(id) => {
                if let Some(dev) = snap.devices.iter().find(|d| &d.id == id) {
                    self.telemetry_details = vec![
                        format!("Device: {}", dev.name),
                        format!("Interface: {}", dev.connection_bus),
                        format!("Status: {}", dev.status),
                    ];
                }
            }
        }
    }
}
