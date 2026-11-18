use crate::cpu::read_cpu_telemetry;
use crate::devices::read_connected_devices;
use crate::disks::read_disk_telemetry;
use crate::memory::read_memory_telemetry;
use crate::network::NetworkTracker;
use crate::power::read_power_telemetry;
use crate::thermal::read_thermal_telemetry;
use chrono::Utc;
use sysinfo::System;
use truni_core::events::TelemetrySnapshot;

pub struct TelemetryCollector {
    sys: System,
    net_tracker: NetworkTracker,
    hostname: String,
    kernel_version: String,
}

impl Default for TelemetryCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl TelemetryCollector {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        let hostname = System::host_name().unwrap_or_else(|| "solaris".to_string());
        let kernel_version = System::kernel_version().unwrap_or_else(|| "linux".to_string());
        let net_tracker = NetworkTracker::new();

        Self {
            sys,
            net_tracker,
            hostname,
            kernel_version,
        }
    }

    pub fn snapshot(&mut self) -> TelemetrySnapshot {
        let cpu = read_cpu_telemetry(&mut self.sys);
        let memory = read_memory_telemetry(&mut self.sys);
        let disks = read_disk_telemetry();
        let network = self.net_tracker.sample();
        let thermal = read_thermal_telemetry();
        let power = read_power_telemetry();
        let devices = read_connected_devices();

        TelemetrySnapshot {
            timestamp: Utc::now(),
            hostname: self.hostname.clone(),
            kernel_version: self.kernel_version.clone(),
            cpu,
            memory,
            disks,
            network,
            thermal,
            power,
            devices,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_live_telemetry_snapshot() {
        let mut collector = TelemetryCollector::new();
        let snap = collector.snapshot();
        println!("Live Telemetry Snapshot:");
        println!("  Hostname: {}", snap.hostname);
        println!("  Kernel: {}", snap.kernel_version);
        println!("  CPU Cores: {}, Total Usage: {:.1}%", snap.cpu.core_count, snap.cpu.usage_percent);
        println!("  Memory: {} MB / {} MB ({:.1}%)", snap.memory.used_mb, snap.memory.total_mb, snap.memory.usage_percent);
        println!("  Thermal CPU Temp: {:?}°C, Fan RPM: {:?}", snap.thermal.cpu_temp_c, snap.thermal.cpu_fan_rpm);
        println!("  Battery: {:?}%, Charging: {:?}", snap.power.battery_percent, snap.power.is_charging);
        println!("  Disks: {}", snap.disks.len());
        println!("  Connected Devices: {}", snap.devices.len());

        assert!(snap.cpu.core_count > 0);
        assert!(snap.memory.total_mb > 0);
        assert!(!snap.disks.is_empty());
    }
}

