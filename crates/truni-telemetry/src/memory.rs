use std::fs;
use sysinfo::{MemoryRefreshKind, ProcessRefreshKind, RefreshKind, System};
use truni_core::events::{MemoryTelemetry, ProcessSummary};

pub fn read_memory_telemetry(sys: &mut System) -> MemoryTelemetry {
    sys.refresh_specifics(
        RefreshKind::nothing()
            .with_memory(MemoryRefreshKind::everything())
            .with_processes(ProcessRefreshKind::everything()),
    );

    let total = sys.total_memory() / (1024 * 1024);
    let used = sys.used_memory() / (1024 * 1024);
    let available = sys.available_memory() / (1024 * 1024);
    let usage_percent = if total > 0 {
        (used as f32 / total as f32) * 100.0
    } else {
        0.0
    };

    let spd_temp_c = read_spd_temp();

    // Top 5 processes by memory usage
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

    processes.sort_by(|a, b| b.mem_mb.cmp(&a.mem_mb));
    processes.truncate(5);

    MemoryTelemetry {
        total_mb: total,
        used_mb: used,
        available_mb: available,
        usage_percent,
        spd_temp_c,
        top_processes: processes,
    }
}

fn read_spd_temp() -> Option<f32> {
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(name) = fs::read_to_string(path.join("name")) {
                if name.trim() == "spd5118" {
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
