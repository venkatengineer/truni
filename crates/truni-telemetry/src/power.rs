use std::fs;
use truni_core::events::PowerTelemetry;

pub fn read_power_telemetry() -> PowerTelemetry {
    let bat_dir = std::path::Path::new("/sys/class/power_supply/BAT0");
    if !bat_dir.exists() {
        return PowerTelemetry {
            has_battery: false,
            battery_percent: None,
            is_charging: None,
            power_watts: None,
            voltage_volts: None,
        };
    }

    let battery_percent = fs::read_to_string(bat_dir.join("capacity"))
        .ok()
        .and_then(|s| s.trim().parse::<f32>().ok());

    let is_charging = fs::read_to_string(bat_dir.join("status"))
        .ok()
        .map(|s| s.trim().eq_ignore_ascii_case("charging"));

    let voltage_volts = fs::read_to_string(bat_dir.join("voltage_now"))
        .ok()
        .and_then(|s| s.trim().parse::<f32>().ok().map(|v| v / 1_000_000.0));

    let power_watts = fs::read_to_string(bat_dir.join("power_now"))
        .ok()
        .and_then(|s| s.trim().parse::<f32>().ok().map(|p| p / 1_000_000.0))
        .or_else(|| {
            // calculate from current_now * voltage_now
            let current = fs::read_to_string(bat_dir.join("current_now"))
                .ok()
                .and_then(|s| s.trim().parse::<f32>().ok().map(|c| c / 1_000_000.0))?;
            let voltage = voltage_volts?;
            Some(current * voltage)
        });

    PowerTelemetry {
        has_battery: true,
        battery_percent,
        is_charging,
        power_watts,
        voltage_volts,
    }
}
