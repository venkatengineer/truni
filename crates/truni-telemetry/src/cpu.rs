use std::fs;
use sysinfo::{CpuRefreshKind, ProcessRefreshKind, RefreshKind, System};
use truni_core::events::{CpuTelemetry, ProcessSummary};

pub fn read_cpu_telemetry(sys: &mut System) -> CpuTelemetry {
    sys.refresh_specifics(
        RefreshKind::nothing()
            .with_cpu(CpuRefreshKind::everything())
            .with_processes(ProcessRefreshKind::everything()),
    );

    let global_cpu = sys.global_cpu_usage();
    let cpus = sys.cpus();
    let core_count = cpus.len();
    let per_core_usage: Vec<f32> = cpus.iter().map(|c| c.cpu_usage()).collect();

    // Average CPU frequency in MHz
    let total_freq: u64 = cpus.iter().map(|c| c.frequency()).sum();
    let avg_freq_mhz = if core_count > 0 {
        total_freq / (core_count as u64)
    } else {
        0
    };

    // CPU Temperature from hwmon or sysfs
    let temp_c = read_cpu_temp();

    // Top 5 processes by CPU usage
    let mut processes: Vec<ProcessSummary> = sys
        .processes()
        .iter()
        .map(|(pid, proc)| ProcessSummary {
            pid: pid.as_u32(),
            name: proc.name().to_string_lossy().to_string(),
            cpu_percent: proc.cpu_usage(),
            mem_mb: proc.memory() / (1024 * 1024),
        })
        .collect();

    processes.sort_by(|a, b| b.cpu_percent.partial_cmp(&a.cpu_percent).unwrap_or(std::cmp::Ordering::Equal));
    processes.truncate(5);

    CpuTelemetry {
        usage_percent: global_cpu,
        avg_freq_mhz,
        temp_c,
        core_count,
        per_core_usage,
        top_processes: processes,
    }
}

fn read_cpu_temp() -> Option<f32> {
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(name) = fs::read_to_string(path.join("name")) {
                let name = name.trim();
                if name == "coretemp" || name == "alienware_wmi" || name == "dell_smm" {
                    if let Ok(temp_str) = fs::read_to_string(path.join("temp1_input")) {
                        if let Ok(val) = temp_str.trim().parse::<f32>() {
                            return Some(val / 1000.0);
                        }
                    }
                }
            }
        }
    }
    None
}
