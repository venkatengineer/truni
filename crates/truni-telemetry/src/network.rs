use std::collections::HashMap;
use std::fs;
use std::time::Instant;
use truni_core::events::{NetworkInterfaceTelemetry, NetworkTelemetry};

pub struct NetworkTracker {
    last_sample_time: Instant,
    last_samples: HashMap<String, (u64, u64)>, // iface -> (rx_bytes, tx_bytes)
}

impl Default for NetworkTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkTracker {
    pub fn new() -> Self {
        let mut tracker = Self {
            last_sample_time: Instant::now(),
            last_samples: HashMap::new(),
        };
        // Initial sample
        tracker.read_raw_dev();
        tracker
    }

    fn read_raw_dev(&mut self) -> HashMap<String, (u64, u64)> {
        let mut map = HashMap::new();
        if let Ok(content) = fs::read_to_string("/proc/net/dev") {
            for line in content.lines().skip(2) {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 10 {
                    let iface = parts[0].trim_end_matches(':').to_string();
                    let rx_bytes = parts[1].parse::<u64>().unwrap_or(0);
                    let tx_bytes = parts[9].parse::<u64>().unwrap_or(0);
                    map.insert(iface, (rx_bytes, tx_bytes));
                }
            }
        }
        map
    }

    pub fn sample(&mut self) -> NetworkTelemetry {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_sample_time).as_secs_f64();
        let current_samples = self.read_raw_dev();

        let mut interfaces = Vec::new();
        let mut total_rx_bps = 0u64;
        let mut total_tx_bps = 0u64;

        for (iface, &(curr_rx, curr_tx)) in &current_samples {
            if iface == "lo" {
                continue;
            }

            let (rx_rate, tx_rate) = if elapsed > 0.0 {
                if let Some(&(prev_rx, prev_tx)) = self.last_samples.get(iface) {
                    let d_rx = curr_rx.saturating_sub(prev_rx);
                    let d_tx = curr_tx.saturating_sub(prev_tx);
                    ((d_rx as f64 / elapsed) as u64, (d_tx as f64 / elapsed) as u64)
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            };

            total_rx_bps += rx_rate;
            total_tx_bps += tx_rate;

            let is_up = fs::read_to_string(format!("/sys/class/net/{}/operstate", iface))
                .map(|s| s.trim() == "up")
                .unwrap_or(false);

            let ip_addresses = read_iface_ips(iface);

            interfaces.push(NetworkInterfaceTelemetry {
                name: iface.clone(),
                is_up,
                rx_bytes_per_sec: rx_rate,
                tx_bytes_per_sec: tx_rate,
                ip_addresses,
            });
        }

        self.last_sample_time = now;
        self.last_samples = current_samples;

        NetworkTelemetry {
            total_rx_rate_bps: total_rx_bps,
            total_tx_rate_bps: total_tx_bps,
            interfaces,
        }
    }
}

fn read_iface_ips(iface: &str) -> Vec<String> {
    let mut ips = Vec::new();
    if let Ok(output) = std::process::Command::new("ip")
        .args(["-o", "-4", "addr", "show", iface])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 4 {
                ips.push(parts[3].to_string());
            }
        }
    }
    ips
}
