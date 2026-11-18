use crate::components::{ComponentId, SpaceshipComponent};
use glam::{Mat4, Vec3};
use truni_core::events::TelemetrySnapshot;

pub struct Camera {
    pub target: Vec3,
    pub distance: f32,
    pub yaw: f32,
    pub pitch: f32,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            target: Vec3::new(0.0, 0.0, 0.0),
            distance: 4.8,
            yaw: 0.6,
            pitch: 0.45,
        }
    }
}

impl Camera {
    pub fn view_matrix(&self) -> Mat4 {
        let x = self.distance * self.pitch.cos() * self.yaw.sin();
        let y = self.distance * self.pitch.sin();
        let z = self.distance * self.pitch.cos() * self.yaw.cos();
        let eye = self.target + Vec3::new(x, y, z);
        Mat4::look_at_rh(eye, self.target, Vec3::Y)
    }

    pub fn projection_matrix(&self, aspect: f32) -> Mat4 {
        Mat4::perspective_rh(45.0_f32.to_radians(), aspect, 0.1, 100.0)
    }

    pub fn rotate(&mut self, dx: f32, dy: f32) {
        self.yaw += dx * 0.008;
        self.pitch = (self.pitch + dy * 0.008).clamp(-1.4, 1.4);
    }

    pub fn zoom(&mut self, delta: f32) {
        self.distance = (self.distance - delta * 0.4).clamp(1.8, 12.0);
    }
}

pub struct SpaceshipScene {
    pub camera: Camera,
    pub components: Vec<SpaceshipComponent>,
    pub selected_component: Option<ComponentId>,
    pub anim_time: f32,
}

impl Default for SpaceshipScene {
    fn default() -> Self {
        Self::new()
    }
}

impl SpaceshipScene {
    pub fn new() -> Self {
        let mut components = Vec::new();

        // 1. Central Chassis Hull (The Laptop Frame)
        components.push(SpaceshipComponent {
            id: ComponentId::ChassisHull,
            name: "Starship Chassis // Laptop Core".to_string(),
            role: "Primary Titanium Structural Frame".to_string(),
            position: Vec3::new(0.0, 0.0, 0.0),
            size: Vec3::new(2.4, 0.18, 1.6),
            base_color: [0.10, 0.13, 0.18, 0.95],
            emissive_color: [0.0, 0.4, 0.8, 1.0],
            emissive_intensity: 0.2,
            is_selected: false,
            telemetry_details: Vec::new(),
        });

        // 2. CPU Reactor Core
        components.push(SpaceshipComponent {
            id: ComponentId::CpuReactor,
            name: "Quantum Fusion Core // Intel Core 7".to_string(),
            role: "16-Core Reactor Unit".to_string(),
            position: Vec3::new(0.0, 0.15, -0.1),
            size: Vec3::new(0.5, 0.12, 0.5),
            base_color: [0.12, 0.18, 0.25, 1.0],
            emissive_color: [0.0, 0.9, 0.7, 1.0],
            emissive_intensity: 0.8,
            is_selected: false,
            telemetry_details: Vec::new(),
        });

        // 3. Memory Array Banks
        components.push(SpaceshipComponent {
            id: ComponentId::MemoryBanks,
            name: "Neural RAM Matrices // Dual DDR5".to_string(),
            role: "16 GB High-Bandwidth Storage Grid".to_string(),
            position: Vec3::new(0.5, 0.14, 0.15),
            size: Vec3::new(0.65, 0.10, 0.25),
            base_color: [0.15, 0.20, 0.28, 1.0],
            emissive_color: [0.1, 0.8, 0.4, 1.0],
            emissive_intensity: 0.6,
            is_selected: false,
            telemetry_details: Vec::new(),
        });

        // 4. NVMe Solid-State Storage
        components.push(SpaceshipComponent {
            id: ComponentId::NvmeStorage,
            name: "Hyperdrive NVMe Array".to_string(),
            role: "Solid-State Data Vault".to_string(),
            position: Vec3::new(-0.5, 0.14, 0.2),
            size: Vec3::new(0.45, 0.08, 0.3),
            base_color: [0.14, 0.16, 0.22, 1.0],
            emissive_color: [0.9, 0.6, 0.1, 1.0],
            emissive_intensity: 0.5,
            is_selected: false,
            telemetry_details: Vec::new(),
        });

        // 5. Dedicated GPU Module
        components.push(SpaceshipComponent {
            id: ComponentId::GpuCore,
            name: "Warp Accelerator // NVIDIA RTX 5050".to_string(),
            role: "Spatial Geometry & Neural Compute".to_string(),
            position: Vec3::new(-0.4, 0.15, -0.2),
            size: Vec3::new(0.55, 0.12, 0.45),
            base_color: [0.11, 0.22, 0.15, 1.0],
            emissive_color: [0.2, 0.95, 0.3, 1.0],
            emissive_intensity: 0.7,
            is_selected: false,
            telemetry_details: Vec::new(),
        });

        // 6. Cooling Manifold & Twin Turbines
        components.push(SpaceshipComponent {
            id: ComponentId::CoolingTurbines,
            name: "Cryogenic Cooling Manifold".to_string(),
            role: "Twin Turbofan Heat Dissipation".to_string(),
            position: Vec3::new(0.0, 0.12, -0.65),
            size: Vec3::new(1.8, 0.16, 0.35),
            base_color: [0.18, 0.22, 0.28, 1.0],
            emissive_color: [0.2, 0.7, 1.0, 1.0],
            emissive_intensity: 0.5,
            is_selected: false,
            telemetry_details: Vec::new(),
        });

        // 7. Network Communication Fins
        components.push(SpaceshipComponent {
            id: ComponentId::NetworkAntennas,
            name: "Subspace Comm Array // Wi-Fi 7 & BlueZ".to_string(),
            role: "Radio & High-Frequency Transceiver".to_string(),
            position: Vec3::new(1.15, 0.25, 0.0),
            size: Vec3::new(0.12, 0.45, 1.2),
            base_color: [0.15, 0.25, 0.35, 1.0],
            emissive_color: [0.3, 0.85, 1.0, 1.0],
            emissive_intensity: 0.5,
            is_selected: false,
            telemetry_details: Vec::new(),
        });

        // 8. Power Subsystem Core
        components.push(SpaceshipComponent {
            id: ComponentId::PowerCell,
            name: "Zero-Point Cell // Lithium Matrix".to_string(),
            role: "Power Distribution Unit".to_string(),
            position: Vec3::new(0.0, -0.06, 0.55),
            size: Vec3::new(1.4, 0.10, 0.4),
            base_color: [0.20, 0.18, 0.12, 1.0],
            emissive_color: [1.0, 0.8, 0.2, 1.0],
            emissive_intensity: 0.7,
            is_selected: false,
            telemetry_details: Vec::new(),
        });

        Self {
            camera: Camera::default(),
            components,
            selected_component: None,
            anim_time: 0.0,
        }
    }

    pub fn update(&mut self, snap: &TelemetrySnapshot, dt: f32) {
        self.anim_time += dt;

        // Sync connected peripheral devices as orbiting docking satellites
        self.sync_peripherals(snap);

        // Update each component's animation and telemetry
        for comp in &mut self.components {
            comp.update_from_telemetry(snap, self.anim_time);
            comp.is_selected = self.selected_component.as_ref() == Some(&comp.id);
        }
    }

    fn sync_peripherals(&mut self, snap: &TelemetrySnapshot) {
        // Retain only core components, then add current peripherals
        self.components.retain(|c| match &c.id {
            ComponentId::PeripheralSatellite(_) => false,
            _ => true,
        });

        let mut angle = 0.0f32;
        for dev in &snap.devices {
            let radius = 2.2 + (angle * 0.15).sin().abs() * 0.4;
            let x = radius * (angle + self.anim_time * 0.2).cos();
            let z = radius * (angle + self.anim_time * 0.2).sin();
            let y = 0.15 + (angle * 1.5).cos() * 0.25;

            self.components.push(SpaceshipComponent {
                id: ComponentId::PeripheralSatellite(dev.id.clone()),
                name: format!("Docked // {}", dev.name),
                role: format!("{} Module ({})", dev.connection_bus, dev.status),
                position: Vec3::new(x, y, z),
                size: Vec3::new(0.25, 0.12, 0.25),
                base_color: [0.18, 0.22, 0.30, 0.9],
                emissive_color: [0.0, 0.9, 0.9, 1.0],
                emissive_intensity: 0.6,
                is_selected: false,
                telemetry_details: vec![
                    format!("Device ID: {}", dev.id),
                    format!("Class: {:?}", dev.dev_type),
                    format!("Status: {}", dev.status),
                ],
            });

            angle += 1.1;
        }
    }

    pub fn select_next(&mut self) {
        if self.components.is_empty() {
            return;
        }

        let next_idx = if let Some(curr) = &self.selected_component {
            let pos = self.components.iter().position(|c| &c.id == curr).unwrap_or(0);
            (pos + 1) % self.components.len()
        } else {
            0
        };

        self.selected_component = Some(self.components[next_idx].id.clone());
    }

    pub fn get_selected_component(&self) -> Option<&SpaceshipComponent> {
        let id = self.selected_component.as_ref()?;
        self.components.iter().find(|c| &c.id == id)
    }
}
