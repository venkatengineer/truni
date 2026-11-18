use crate::gl_viewport::SpaceshipViewport;
use gtk4::prelude::*;
use gtk4::{Box as GtkBox, Button, Entry, Label, Orientation, ScrolledWindow, Stack};
use truni_core::events::{OverlayState, StepStatus, TaskPlan, TelemetrySnapshot};

#[allow(dead_code)]
pub struct HudUI {
    pub root_box: GtkBox,
    pub stack: Stack,
    // Canopy Header
    pub chrono_time_lbl: Label,
    pub top_telemetry_lbl: Label,
    pub dismiss_btn: Button,
    // Left Bulkhead Gauges
    pub side_cpu_lbl: Label,
    pub side_ram_lbl: Label,
    pub side_fan_lbl: Label,
    pub side_disk_lbl: Label,
    pub side_power_lbl: Label,
    pub side_devices_lbl: Label,
    // Page Standby
    pub standby_status_lbl: Label,
    pub standby_telemetry_lbl: Label,
    // Page Conversation
    pub chat_history_box: GtkBox,
    pub chat_scroll: ScrolledWindow,
    pub input_entry: Entry,
    pub send_btn: Button,
    pub voice_btn: Button,
    // Page Task Execution
    pub task_title_lbl: Label,
    pub task_steps_box: GtkBox,
    pub approval_banner: GtkBox,
    pub approval_desc_lbl: Label,
    pub approve_btn: Button,
    pub deny_btn: Button,
    // Page Spaceship Inspection
    pub viewport: SpaceshipViewport,
    pub comp_name_lbl: Label,
    pub comp_role_lbl: Label,
    pub comp_details_box: GtkBox,
    pub cycle_comp_btn: Button,
    // Nav Buttons
    pub nav_standby_btn: Button,
    pub nav_conv_btn: Button,
    pub nav_task_btn: Button,
    pub nav_ship_btn: Button,
}

impl HudUI {
    pub fn new() -> Self {
        let root_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .margin_top(12)
            .margin_bottom(12)
            .margin_start(16)
            .margin_end(16)
            .hexpand(true)
            .vexpand(true)
            .css_classes(["solaris-glass-deck"])
            .build();

        // Top Navigation Bar (Canopy Rail)
        let nav_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .css_classes(["cockpit-top-rail"])
            .build();

        let title_lbl = Label::builder()
            .label("🛸 SOLARIS BRIDGE // TRUNI SPATIAL AI")
            .css_classes(["hud-header"])
            .xalign(0.0)
            .build();

        let chrono_time_lbl = Label::builder()
            .label("CHRONO: NOMINAL")
            .css_classes(["hud-pill"])
            .build();

        let top_telemetry_lbl = Label::builder()
            .label("[  CPU: --% ] [ 󰍛 RAM: --M ] [ 󰈐 FANS: -- RPM ]")
            .css_classes(["hud-pill-green"])
            .hexpand(true)
            .xalign(0.5)
            .build();

        let nav_conv_btn = Button::with_label("⬡ COMM DECK [F2]");
        let nav_task_btn = Button::with_label("◈ MISSION OPS [F3]");
        let nav_ship_btn = Button::with_label("⌬ 3D STARSHIP [F4]");
        let nav_standby_btn = Button::with_label("⬡ STANDBY [F1]");
        let dismiss_btn = Button::with_label("✕ DISMISS [ESC]");

        nav_standby_btn.add_css_class("hud-nav-btn");
        nav_conv_btn.add_css_class("hud-nav-btn");
        nav_task_btn.add_css_class("hud-nav-btn");
        nav_ship_btn.add_css_class("hud-nav-btn-ship");
        dismiss_btn.add_css_class("btn-danger");

        nav_box.append(&title_lbl);
        nav_box.append(&chrono_time_lbl);
        nav_box.append(&top_telemetry_lbl);
        nav_box.append(&nav_conv_btn);
        nav_box.append(&nav_task_btn);
        nav_box.append(&nav_ship_btn);
        nav_box.append(&nav_standby_btn);
        nav_box.append(&dismiss_btn);

        root_box.append(&nav_box);

        let stack = Stack::builder()
            .transition_type(gtk4::StackTransitionType::Crossfade)
            .transition_duration(200)
            .hexpand(true)
            .vexpand(true)
            .build();

        // 1. STATE A: Standby Page
        let standby_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(12)
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .build();

        let standby_status_lbl = Label::builder()
            .label("🛸 TRUNI STANDBY // Awaiting Voice ('TRUNI') or Shortcut")
            .css_classes(["hud-header", "hud-pill-green"])
            .build();

        let standby_telemetry_lbl = Label::builder()
            .label("Telemetry: CPU 0% | RAM 0 MB | Fans Nominal | Battery 100%")
            .css_classes(["hud-panel"])
            .build();

        standby_box.append(&standby_status_lbl);
        standby_box.append(&standby_telemetry_lbl);
        stack.add_named(&standby_box, Some("standby"));

        // 2. STATE B: Activation Page
        let activation_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(16)
            .valign(gtk4::Align::Center)
            .halign(gtk4::Align::Center)
            .build();

        let act_lbl = Label::builder()
            .label("⚡ INITIALIZING SOLARIS SPATIAL CORE...\nSystems Online. Neural Link Active.")
            .css_classes(["hud-header", "hud-pill-amber"])
            .build();
        activation_box.append(&act_lbl);
        stack.add_named(&activation_box, Some("activation"));

        // 3. STATE C: Conversation Page (2-Column Solaris Cockpit Layout)
        let conv_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(14)
            .hexpand(true)
            .vexpand(true)
            .build();

        // Left Bulkhead Avionics Instrumentation Sidebar
        let bulkhead_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .width_request(280)
            .css_classes(["hud-panel", "bulkhead-sidebar"])
            .build();

        let side_title_lbl = Label::builder()
            .label("⬡ AVIONICS TELEMETRY")
            .css_classes(["hud-header"])
            .xalign(0.0)
            .build();

        let side_status_lbl = Label::builder()
            .label("FLIGHT DECK: ONLINE")
            .css_classes(["hud-pill-green"])
            .xalign(0.0)
            .build();

        let side_cpu_lbl = Label::builder()
            .label(" CPU CORE // LOAD: --%\n  Freq: -- MHz | Temp: --°C")
            .wrap(true)
            .xalign(0.0)
            .css_classes(["gauge-card"])
            .build();

        let side_ram_lbl = Label::builder()
            .label("󰍛 MEMORY // USED: -- MB\n  Total: -- MB | Free: -- MB")
            .wrap(true)
            .xalign(0.0)
            .css_classes(["gauge-card"])
            .build();

        let side_fan_lbl = Label::builder()
            .label("󰈐 COOLING // -- RPM\n  Thermal State: Nominal")
            .wrap(true)
            .xalign(0.0)
            .css_classes(["gauge-card"])
            .build();

        let side_disk_lbl = Label::builder()
            .label("󰋊 STORAGE // -- GB FREE\n  Capacity: -- GB | Temp: --°C")
            .wrap(true)
            .xalign(0.0)
            .css_classes(["gauge-card"])
            .build();

        let side_power_lbl = Label::builder()
            .label("󰁹 POWER // --%\n  State: Nominal")
            .wrap(true)
            .xalign(0.0)
            .css_classes(["gauge-card"])
            .build();

        let side_devices_lbl = Label::builder()
            .label("󰒋 PERIPHERALS // -- NODES\n  Bus: USB / BT / Display")
            .wrap(true)
            .xalign(0.0)
            .css_classes(["gauge-card"])
            .build();

        let side_footer_lbl = Label::builder()
            .label("HOST: SOLARIS-ARCH\nKERNEL: Linux Zen 7.2")
            .xalign(0.0)
            .css_classes(["hud-pill"])
            .vexpand(false)
            .build();

        bulkhead_box.append(&side_title_lbl);
        bulkhead_box.append(&side_status_lbl);
        bulkhead_box.append(&side_cpu_lbl);
        bulkhead_box.append(&side_ram_lbl);
        bulkhead_box.append(&side_fan_lbl);
        bulkhead_box.append(&side_disk_lbl);
        bulkhead_box.append(&side_power_lbl);
        bulkhead_box.append(&side_devices_lbl);
        bulkhead_box.append(&side_footer_lbl);

        // Right Column: Holographic Chat & Terminal Command
        let flight_deck_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .hexpand(true)
            .vexpand(true)
            .build();

        let chat_history_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .vexpand(true)
            .build();

        let chat_scroll = ScrolledWindow::builder()
            .child(&chat_history_box)
            .hscrollbar_policy(gtk4::PolicyType::Never)
            .vscrollbar_policy(gtk4::PolicyType::Automatic)
            .hexpand(true)
            .vexpand(true)
            .css_classes(["hud-panel"])
            .build();

        // Preset Chips Bar
        let chips_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .build();

        let chip1 = Button::with_label("⚡ What's consuming RAM?");
        let chip2 = Button::with_label("🔥 Why is laptop getting hot?");
        let chip3 = Button::with_label("📂 Organise Downloads");
        let chip4 = Button::with_label("🛰️ Connected Devices");
        let chip5 = Button::with_label("⚛️ Quantum Computing");

        chip1.add_css_class("hud-chip");
        chip2.add_css_class("hud-chip");
        chip3.add_css_class("hud-chip");
        chip4.add_css_class("hud-chip");
        chip5.add_css_class("hud-chip");

        chips_box.append(&chip1);
        chips_box.append(&chip2);
        chips_box.append(&chip3);
        chips_box.append(&chip4);
        chips_box.append(&chip5);

        // Input bar
        let input_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(8)
            .build();

        let input_entry = Entry::builder()
            .placeholder_text("Ask TRUNI or assign computer tasks...")
            .hexpand(true)
            .css_classes(["hud-input"])
            .build();

        let send_btn = Button::with_label("⚡ TRANSMIT");
        send_btn.add_css_class("hud-btn-transmit");

        let voice_btn = Button::with_label("🎙️ COMM VOICE");
        voice_btn.add_css_class("hud-btn-voice");

        input_box.append(&input_entry);
        input_box.append(&voice_btn);
        input_box.append(&send_btn);

        flight_deck_box.append(&chat_scroll);
        flight_deck_box.append(&chips_box);
        flight_deck_box.append(&input_box);

        conv_box.append(&bulkhead_box);
        conv_box.append(&flight_deck_box);

        stack.add_named(&conv_box, Some("conversation"));

        // 4. STATE D: Task Execution Page
        let task_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .hexpand(true)
            .vexpand(true)
            .build();

        let task_title_lbl = Label::builder()
            .label("ACTIVE MISSION PLAN: Standby")
            .css_classes(["hud-header"])
            .xalign(0.0)
            .build();

        let task_steps_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(8)
            .hexpand(true)
            .vexpand(true)
            .build();

        let task_scroll = ScrolledWindow::builder()
            .child(&task_steps_box)
            .css_classes(["hud-panel"])
            .hexpand(true)
            .vexpand(true)
            .build();

        // Operator Approval Banner
        let approval_banner = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .visible(false)
            .css_classes(["hud-panel"])
            .build();

        let approval_desc_lbl = Label::builder()
            .label("⚠️ OPERATOR APPROVAL REQUIRED")
            .css_classes(["hud-pill-amber"])
            .hexpand(true)
            .xalign(0.0)
            .build();

        let approve_btn = Button::with_label("APPROVE ACTION");
        approve_btn.add_css_class("btn-success");

        let deny_btn = Button::with_label("ABORT");
        deny_btn.add_css_class("btn-danger");

        approval_banner.append(&approval_desc_lbl);
        approval_banner.append(&approve_btn);
        approval_banner.append(&deny_btn);

        task_box.append(&task_title_lbl);
        task_box.append(&task_scroll);
        task_box.append(&approval_banner);

        stack.add_named(&task_box, Some("task_execution"));

        // 5. STATE E: Spaceship Inspection Page
        let ship_box = GtkBox::builder()
            .orientation(Orientation::Horizontal)
            .spacing(12)
            .hexpand(true)
            .vexpand(true)
            .build();

        let viewport = SpaceshipViewport::new();

        // Right side Telemetry HUD panel
        let hud_side_panel = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(10)
            .width_request(340)
            .css_classes(["hud-panel"])
            .build();

        let comp_name_lbl = Label::builder()
            .label("Component: Chassis Hull")
            .css_classes(["hud-header"])
            .xalign(0.0)
            .build();

        let comp_role_lbl = Label::builder()
            .label("Role: Primary Structure")
            .css_classes(["hud-pill-green"])
            .xalign(0.0)
            .build();

        let comp_details_box = GtkBox::builder()
            .orientation(Orientation::Vertical)
            .spacing(6)
            .hexpand(true)
            .vexpand(true)
            .build();

        let cycle_comp_btn = Button::with_label("CYCLE HARDWARE COMPONENT");
        cycle_comp_btn.add_css_class("hud-pill-amber");

        hud_side_panel.append(&comp_name_lbl);
        hud_side_panel.append(&comp_role_lbl);
        hud_side_panel.append(&comp_details_box);
        hud_side_panel.append(&cycle_comp_btn);

        ship_box.append(&viewport.gl_area);
        ship_box.append(&hud_side_panel);

        stack.add_named(&ship_box, Some("spaceship_inspection"));

        root_box.append(&stack);

        // Connect Preset Chips
        {
            let input_entry = input_entry.clone();
            chip1.connect_clicked(move |_| input_entry.set_text("What is consuming my RAM?"));
        }
        {
            let input_entry = input_entry.clone();
            chip2.connect_clicked(move |_| input_entry.set_text("Why is my laptop getting hot?"));
        }
        {
            let input_entry = input_entry.clone();
            chip3.connect_clicked(move |_| input_entry.set_text("Organise my Downloads folder"));
        }
        {
            let input_entry = input_entry.clone();
            chip4.connect_clicked(move |_| input_entry.set_text("Show me all connected devices"));
        }
        {
            let input_entry = input_entry.clone();
            chip5.connect_clicked(move |_| input_entry.set_text("Research the latest developments in quantum computing"));
        }

        Self {
            root_box,
            stack,
            chrono_time_lbl,
            top_telemetry_lbl,
            dismiss_btn,
            side_cpu_lbl,
            side_ram_lbl,
            side_fan_lbl,
            side_disk_lbl,
            side_power_lbl,
            side_devices_lbl,
            standby_status_lbl,
            standby_telemetry_lbl,
            chat_history_box,
            chat_scroll,
            input_entry,
            send_btn,
            voice_btn,
            task_title_lbl,
            task_steps_box,
            approval_banner,
            approval_desc_lbl,
            approve_btn,
            deny_btn,
            viewport,
            comp_name_lbl,
            comp_role_lbl,
            comp_details_box,
            cycle_comp_btn,
            nav_standby_btn,
            nav_conv_btn,
            nav_task_btn,
            nav_ship_btn,
        }
    }

    pub fn switch_state(&self, state: OverlayState) {
        let page_name = match state {
            OverlayState::Standby => "standby",
            OverlayState::Activation => "activation",
            OverlayState::Conversation => "conversation",
            OverlayState::TaskExecution => "task_execution",
            OverlayState::SpaceshipInspection => "spaceship_inspection",
        };
        self.stack.set_visible_child_name(page_name);
    }

    pub fn append_chat_message(&self, is_user: bool, text: &str) {
        let bubble = Label::builder()
            .label(text)
            .wrap(true)
            .xalign(0.0)
            .css_classes([if is_user {
                "chat-bubble-user"
            } else {
                "chat-bubble-truni"
            }])
            .build();

        self.chat_history_box.append(&bubble);
    }

    pub fn update_task_plan(&self, plan: &TaskPlan) {
        self.task_title_lbl.set_text(&format!("ACTIVE MISSION PLAN: {}", plan.prompt));

        // Clear existing steps
        while let Some(child) = self.task_steps_box.first_child() {
            self.task_steps_box.remove(&child);
        }

        for step in &plan.steps {
            let (icon, css_class) = match step.status {
                StepStatus::Completed => ("✓", "step-card-completed"),
                StepStatus::Running => ("⚙", "step-card-running"),
                StepStatus::Failed => ("✗", "step-card-failed"),
                StepStatus::AwaitingApproval => ("⚠️", "step-card-running"),
                StepStatus::Pending => ("○", "step-card"),
            };

            let log_text = step.log.as_deref().unwrap_or("Awaiting execution phase...");
            let step_lbl = Label::builder()
                .label(&format!("[{}] Step {}: {}\n   Status: {:?} | Log: {}", icon, step.index + 1, step.title, step.status, log_text))
                .wrap(true)
                .xalign(0.0)
                .css_classes(["step-card", css_class])
                .build();

            self.task_steps_box.append(&step_lbl);
        }

        if let Some(req) = &plan.pending_approval {
            self.approval_banner.set_visible(true);
            self.approval_desc_lbl.set_text(&format!("⚠️ APPROVAL REQUIRED: {}\nTarget: {}", req.description, req.target));
        } else {
            self.approval_banner.set_visible(false);
        }
    }

    pub fn update_telemetry_hud(&self, snap: &TelemetrySnapshot, dt: f32) {
        // Update 3D OpenGL spaceship
        self.viewport.update_telemetry(snap, dt);

        let now_str = chrono::Local::now().format("%H:%M:%S").to_string();
        self.chrono_time_lbl.set_text(&format!("CHRONO: {}", now_str));

        let cpu_temp = snap.cpu.temp_c.unwrap_or(45.0);
        let fan_rpm = snap.thermal.cpu_fan_rpm.unwrap_or(0);
        let bat_pct = snap.power.battery_percent.unwrap_or(100.0);

        // Update top canopy telemetry
        self.top_telemetry_lbl.set_text(&format!(
            "[  CPU: {:.0}% ] [ 󰍛 RAM: {} MB ] [ 󰈐 FAN: {} RPM ] [ 󰁹 {:.0}% ]",
            snap.cpu.usage_percent,
            snap.memory.used_mb,
            fan_rpm,
            bat_pct
        ));

        // Update Left Bulkhead Gauges
        self.side_cpu_lbl.set_text(&format!(
            " CPU CORE // LOAD: {:.1}%\n  Freq: {:.0} MHz | Temp: {:.1}°C",
            snap.cpu.usage_percent,
            snap.cpu.avg_freq_mhz,
            cpu_temp
        ));

        self.side_ram_lbl.set_text(&format!(
            "󰍛 MEMORY // USED: {} MB\n  Total: {} MB | Free: {} MB",
            snap.memory.used_mb,
            snap.memory.total_mb,
            snap.memory.available_mb
        ));

        self.side_fan_lbl.set_text(&format!(
            "󰈐 COOLING // {} RPM\n  Thermal State: Nominal",
            fan_rpm
        ));

        let disk_free = snap.disks.first().map(|d| d.free_gb).unwrap_or(0.0);
        let disk_total = snap.disks.first().map(|d| d.total_gb).unwrap_or(0.0);
        let disk_temp = snap.disks.first().and_then(|d| d.temp_c).unwrap_or(40.0);
        self.side_disk_lbl.set_text(&format!(
            "󰋊 STORAGE // {:.1} GB FREE\n  Capacity: {:.1} GB | Temp: {:.1}°C",
            disk_free,
            disk_total,
            disk_temp
        ));

        let charging_str = if snap.power.is_charging.unwrap_or(false) { "Charging ⚡" } else { "Discharging 🔋" };
        self.side_power_lbl.set_text(&format!(
            "󰁹 POWER // {:.0}%\n  State: {}",
            bat_pct,
            charging_str
        ));

        self.side_devices_lbl.set_text(&format!(
            "󰒋 PERIPHERALS // {} NODES\n  Bus: USB / BT / Display",
            snap.devices.len()
        ));

        // Update standby text
        self.standby_telemetry_lbl.set_text(&format!(
            "Telemetry: CPU {:.1}% ({:.1}°C, {} RPM) | RAM {} MB / {} MB | NVMe {:.1}°C | Battery {:.0}% | Devices: {}",
            snap.cpu.usage_percent,
            cpu_temp,
            fan_rpm,
            snap.memory.used_mb,
            snap.memory.total_mb,
            disk_temp,
            bat_pct,
            snap.devices.len()
        ));

        // Update selected component details card
        let scene = self.viewport.scene.borrow();
        if let Some(comp) = scene.get_selected_component() {
            self.comp_name_lbl.set_text(&comp.name);
            self.comp_role_lbl.set_text(&comp.role);

            // Rebuild details lines
            while let Some(child) = self.comp_details_box.first_child() {
                self.comp_details_box.remove(&child);
            }

            for line in &comp.telemetry_details {
                let lbl = Label::builder()
                    .label(line)
                    .xalign(0.0)
                    .css_classes(["hud-pill"])
                    .build();
                self.comp_details_box.append(&lbl);
            }
        }
    }
}
