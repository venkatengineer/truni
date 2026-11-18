use std::fs;
use std::process::Command;
use truni_core::events::{ConnectedDevice, DeviceType};

pub fn read_connected_devices() -> Vec<ConnectedDevice> {
    let mut devices = Vec::new();

    // 1. USB Devices
    if let Ok(entries) = fs::read_dir("/sys/bus/usb/devices") {
        for entry in entries.flatten() {
            let path = entry.path();
            if let (Ok(vendor), Ok(product)) = (
                fs::read_to_string(path.join("idVendor")),
                fs::read_to_string(path.join("idProduct")),
            ) {
                let name = fs::read_to_string(path.join("product"))
                    .unwrap_or_else(|_| "Generic USB Device".to_string())
                    .trim()
                    .to_string();

                let manufacturer = fs::read_to_string(path.join("manufacturer"))
                    .unwrap_or_default()
                    .trim()
                    .to_string();

                let full_name = if !manufacturer.is_empty() {
                    format!("{} {}", manufacturer, name)
                } else {
                    name
                };

                let dev_id = format!("{}:{}", vendor.trim(), product.trim());

                let dev_type = if full_name.to_lowercase().contains("mouse")
                    || full_name.to_lowercase().contains("keyboard")
                {
                    DeviceType::Input
                } else if full_name.to_lowercase().contains("cam") {
                    DeviceType::Audio // Camera / A/V
                } else if full_name.to_lowercase().contains("wireless")
                    || full_name.to_lowercase().contains("bluetooth")
                {
                    DeviceType::Network
                } else if full_name.to_lowercase().contains("storage")
                    || full_name.to_lowercase().contains("drive")
                {
                    DeviceType::Storage
                } else {
                    DeviceType::Usb
                };

                devices.push(ConnectedDevice {
                    id: dev_id,
                    name: full_name,
                    dev_type,
                    connection_bus: "USB".to_string(),
                    status: "Connected".to_string(),
                });
            }
        }
    }

    // 2. Bluetooth Devices via bluetoothctl
    if let Ok(output) = Command::new("bluetoothctl").arg("devices").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            // format: Device XX:XX:XX:XX:XX:XX Name
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == "Device" {
                let mac = parts[1].to_string();
                let name = parts[2..].join(" ");
                devices.push(ConnectedDevice {
                    id: mac,
                    name,
                    dev_type: DeviceType::Bluetooth,
                    connection_bus: "Bluetooth".to_string(),
                    status: "Paired".to_string(),
                });
            }
        }
    }

    // 3. Displays from Niri IPC
    if let Ok(output) = Command::new("niri").args(["msg", "-j", "outputs"]).output() {
        if let Ok(val) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
            if let Some(map) = val.as_object() {
                for (name, out_info) in map {
                    let make = out_info.get("make").and_then(|m| m.as_str()).unwrap_or("Display");
                    let model = out_info.get("model").and_then(|m| m.as_str()).unwrap_or(name);
                    let display_name = format!("{} ({})", make, model);

                    devices.push(ConnectedDevice {
                        id: name.clone(),
                        name: display_name,
                        dev_type: DeviceType::Monitor,
                        connection_bus: "DisplayPort/eDP".to_string(),
                        status: "Active".to_string(),
                    });
                }
            }
        }
    }

    devices
}
