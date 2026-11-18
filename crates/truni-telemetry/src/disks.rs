use std::fs;
use sysinfo::Disks;
use truni_core::events::DiskTelemetry;

pub fn read_disk_telemetry() -> Vec<DiskTelemetry> {
    let disks = Disks::new_with_refreshed_list();
    let nvme_temp = read_nvme_temp();

    let mut result = Vec::new();
    for disk in disks.iter() {
        let name = disk.name().to_string_lossy().to_string();
        let mount_point = disk.mount_point().to_string_lossy().to_string();
        let total_b = disk.total_space();
        let avail_b = disk.available_space();
        let used_b = total_b.saturating_sub(avail_b);

        let total_gb = total_b as f32 / (1024.0 * 1024.0 * 1024.0);
        let used_gb = used_b as f32 / (1024.0 * 1024.0 * 1024.0);
        let free_gb = avail_b as f32 / (1024.0 * 1024.0 * 1024.0);
        let usage_percent = if total_gb > 0.0 {
            (used_gb / total_gb) * 100.0
        } else {
            0.0
        };

        // Attach NVMe temperature if it is an nvme device
        let temp_c = if name.contains("nvme") || mount_point == "/" {
            nvme_temp
        } else {
            None
        };

        result.push(DiskTelemetry {
            name,
            mount_point,
            total_gb,
            used_gb,
            free_gb,
            usage_percent,
            temp_c,
        });
    }

    result
}

fn read_nvme_temp() -> Option<f32> {
    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let Ok(name) = fs::read_to_string(path.join("name")) {
                if name.trim() == "nvme" {
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
