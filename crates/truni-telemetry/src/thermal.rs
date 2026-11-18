use std::fs;
use truni_core::events::{ThermalTelemetry, ThermalZone};

pub fn read_thermal_telemetry() -> ThermalTelemetry {
    let mut cpu_temp_c = None;
    let mut gpu_temp_c = None;
    let mut cpu_fan_rpm = None;
    let mut gpu_fan_rpm = None;
    let mut zones = Vec::new();

    if let Ok(entries) = fs::read_dir("/sys/class/hwmon") {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = fs::read_to_string(path.join("name"))
                .unwrap_or_default()
                .trim()
                .to_string();

            // Check fans
            if name == "alienware_wmi" || name == "dell_smm" {
                if let Ok(val) = fs::read_to_string(path.join("fan1_input")) {
                    if let Ok(rpm) = val.trim().parse::<u32>() {
                        cpu_fan_rpm = Some(rpm);
                    }
                }
                if let Ok(val) = fs::read_to_string(path.join("fan2_input")) {
                    if let Ok(rpm) = val.trim().parse::<u32>() {
                        gpu_fan_rpm = Some(rpm);
                    }
                }
                // Also check GPU temperature from alienware_wmi (temp2_input is GPU)
                if let Ok(val) = fs::read_to_string(path.join("temp2_input")) {
                    if let Ok(temp_raw) = val.trim().parse::<f32>() {
                        gpu_temp_c = Some(temp_raw / 1000.0);
                    }
                }
            }

            // Check CPU temp from coretemp
            if name == "coretemp" {
                if let Ok(val) = fs::read_to_string(path.join("temp1_input")) {
                    if let Ok(temp_raw) = val.trim().parse::<f32>() {
                        cpu_temp_c = Some(temp_raw / 1000.0);
                    }
                }
            }

            // Read generic thermal zone entries
            for i in 1..=8 {
                let temp_file = path.join(format!("temp{}_input", i));
                if temp_file.exists() {
                    if let Ok(val) = fs::read_to_string(&temp_file) {
                        if let Ok(temp_raw) = val.trim().parse::<f32>() {
                            let label = fs::read_to_string(path.join(format!("temp{}_label", i)))
                                .map(|s| s.trim().to_string())
                                .unwrap_or_else(|_| format!("{}_temp{}", name, i));

                            let high_c = fs::read_to_string(path.join(format!("temp{}_max", i)))
                                .ok()
                                .and_then(|s| s.trim().parse::<f32>().ok().map(|v| v / 1000.0));

                            let crit_c = fs::read_to_string(path.join(format!("temp{}_crit", i)))
                                .ok()
                                .and_then(|s| s.trim().parse::<f32>().ok().map(|v| v / 1000.0));

                            zones.push(ThermalZone {
                                name: label,
                                temp_c: temp_raw / 1000.0,
                                high_c,
                                crit_c,
                            });
                        }
                    }
                }
            }
        }
    }

    // Fallback CPU temp if coretemp wasn't found
    if cpu_temp_c.is_none() {
        if let Some(zone) = zones.iter().find(|z| z.name.to_lowercase().contains("cpu") || z.name.to_lowercase().contains("package")) {
            cpu_temp_c = Some(zone.temp_c);
        }
    }

    ThermalTelemetry {
        cpu_temp_c,
        gpu_temp_c,
        cpu_fan_rpm,
        gpu_fan_rpm,
        zones,
    }
}
