use crate::hud::HudUI;
use gtk4::gdk::Key;
use gtk4::glib;
use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, CssProvider, EventControllerKey};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;
use truni_agent::AgentOrchestrator;
use truni_core::config::TruniConfig;
use truni_core::events::OverlayState;
use truni_telemetry::TelemetryCollector;
use truni_voice::VoiceService;

pub struct TruniApp {
    config: TruniConfig,
    agent: AgentOrchestrator,
    telemetry: Arc<TokioMutex<TelemetryCollector>>,
    voice: VoiceService,
}

impl TruniApp {
    pub fn new(
        config: TruniConfig,
        agent: AgentOrchestrator,
        telemetry: Arc<TokioMutex<TelemetryCollector>>,
        voice: VoiceService,
    ) -> Self {
        Self {
            config,
            agent,
            telemetry,
            voice,
        }
    }

    pub fn run(&self) {
        let app = Application::builder()
            .application_id("org.solaris.truni")
            .build();

        let config = self.config.clone();
        let agent = self.agent.clone();
        let telemetry = self.telemetry.clone();
        let voice = self.voice.clone();

        app.connect_activate(move |application| {
            // Load custom CSS
            let provider = CssProvider::new();
            provider.load_from_data(include_str!("style.css"));
            if let Some(display) = gtk4::gdk::Display::default() {
                gtk4::style_context_add_provider_for_display(
                    &display,
                    &provider,
                    gtk4::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }

            let window = ApplicationWindow::builder()
                .application(application)
                .title("Solaris TRUNI Spatial Command")
                .default_width(config.overlay.width)
                .default_height(config.overlay.height)
                .decorated(false)
                .build();

            window.add_css_class("transparent-window");

            // Wayland Layer Shell Integration for Niri
            if gtk4_layer_shell::is_supported() {
                use gtk4_layer_shell::LayerShell;
                window.init_layer_shell();
                window.set_layer(gtk4_layer_shell::Layer::Overlay);
                window.set_keyboard_mode(
                    gtk4_layer_shell::KeyboardMode::OnDemand,
                );
                window.set_namespace(Some("truni-overlay"));
            }

            let hud = Rc::new(HudUI::new());
            window.set_child(Some(&hud.root_box));

            // Initial state
            hud.switch_state(OverlayState::Conversation);

            // Connect Navigation & Dismiss Buttons
            {
                let window_clone = window.clone();
                hud.dismiss_btn.connect_clicked(move |_| {
                    window_clone.close();
                });
            }
            {
                let hud_clone = hud.clone();
                hud.nav_standby_btn.connect_clicked(move |_| {
                    hud_clone.switch_state(OverlayState::Standby);
                });
            }
            {
                let hud_clone = hud.clone();
                hud.nav_conv_btn.connect_clicked(move |_| {
                    hud_clone.switch_state(OverlayState::Conversation);
                });
            }
            {
                let hud_clone = hud.clone();
                hud.nav_task_btn.connect_clicked(move |_| {
                    hud_clone.switch_state(OverlayState::TaskExecution);
                });
            }
            {
                let hud_clone = hud.clone();
                hud.nav_ship_btn.connect_clicked(move |_| {
                    hud_clone.switch_state(OverlayState::SpaceshipInspection);
                });
            }
            {
                let hud_clone = hud.clone();
                hud.cycle_comp_btn.connect_clicked(move |_| {
                    hud_clone.viewport.scene.borrow_mut().select_next();
                    hud_clone.viewport.gl_area.queue_render();
                });
            }

            // Chat & Mission dispatch
            {
                let hud_clone = hud.clone();
                let agent_clone = agent.clone();
                let voice_clone = voice.clone();
                let window_clone = window.clone();

                let trigger_task = move |prompt: String| {
                    if prompt.trim().is_empty() {
                        return;
                    }

                    hud_clone.append_chat_message(true, &prompt);
                    hud_clone.input_entry.set_text("");

                    let hud_inner = hud_clone.clone();
                    let agent_inner = agent_clone.clone();
                    let voice_inner = voice_clone.clone();
                    let window_inner = window_clone.clone();

                    glib::spawn_future_local(async move {
                        hud_inner.switch_state(OverlayState::TaskExecution);
                        match agent_inner.start_task(&prompt).await {
                            Ok(plan) => {
                                hud_inner.update_task_plan(&plan);
                                if let Some(summary) = &plan.final_summary {
                                    hud_inner.append_chat_message(false, summary);
                                    let _ = voice_inner.speak(summary);
                                    if summary.contains("Standing by on flight deck")
                                        || summary.contains("Flight deck disengaged")
                                    {
                                        let win = window_inner.clone();
                                        glib::timeout_add_local_once(
                                            std::time::Duration::from_millis(1500),
                                            move || {
                                                win.close();
                                            },
                                        );
                                    }
                                }
                            }
                            Err(e) => {
                                hud_inner.append_chat_message(false, &format!("Mission Error: {}", e));
                            }
                        }
                    });
                };

                let trigger_rc = Rc::new(trigger_task);

                // Entry activate (Enter key)
                {
                    let trigger_rc = trigger_rc.clone();
                    let entry = hud.input_entry.clone();
                    entry.connect_activate(move |e| {
                        let text = e.text().to_string();
                        trigger_rc(text);
                    });
                }

                // Send button click
                {
                    let trigger_rc = trigger_rc.clone();
                    let entry = hud.input_entry.clone();
                    hud.send_btn.connect_clicked(move |_| {
                        let text = entry.text().to_string();
                        trigger_rc(text);
                    });
                }
            }

            // Voice Button Click
            {
                let hud_clone = hud.clone();
                let voice_clone = voice.clone();
                let agent_clone = agent.clone();
                let window_clone = window.clone();

                hud.voice_btn.connect_clicked(move |_| {
                    let hud_inner = hud_clone.clone();
                    let voice_inner = voice_clone.clone();
                    let agent_inner = agent_clone.clone();
                    let window_inner = window_clone.clone();

                    glib::spawn_future_local(async move {
                        hud_inner.append_chat_message(false, "🎙️ Systems online. Listening...");
                        // Ensure PipeWire sink is unmuted
                        let _ = std::process::Command::new("wpctl")
                            .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "0"])
                            .output();
                        let _ = voice_inner.speak("Systems online. Listening.");
                        tokio::time::sleep(std::time::Duration::from_millis(1600)).await;

                        match voice_inner.record_and_transcribe(5).await {
                            Ok(text) => {
                                let trimmed = text.trim();
                                if !trimmed.is_empty() {
                                    hud_inner.append_chat_message(true, trimmed);
                                    hud_inner.switch_state(OverlayState::TaskExecution);
                                    if let Ok(plan) = agent_inner.start_task(trimmed).await {
                                        hud_inner.update_task_plan(&plan);
                                        if let Some(summary) = &plan.final_summary {
                                            hud_inner.append_chat_message(false, summary);
                                            let _ = voice_inner.speak(summary);
                                            if summary.contains("Standing by on flight deck")
                                                || summary.contains("Flight deck disengaged")
                                            {
                                                let win = window_inner.clone();
                                                glib::timeout_add_local_once(
                                                    std::time::Duration::from_millis(1500),
                                                    move || {
                                                        win.close();
                                                    },
                                                );
                                            }
                                        }
                                    }
                                } else {
                                    hud_inner.append_chat_message(false, "No speech detected.");
                                }
                            }
                            Err(e) => {
                                hud_inner.append_chat_message(false, &format!("Voice capture error: {}", e));
                            }
                        }
                    });
                });
            }

            // Keyboard navigation controller
            let key_ctrl = EventControllerKey::new();
            {
                let hud_clone = hud.clone();
                let window_clone = window.clone();
                key_ctrl.connect_key_pressed(move |_, key, _, _| {
                    match key {
                        Key::Escape => {
                            window_clone.close();
                            glib::Propagation::Stop
                        }
                        Key::F1 => {
                            hud_clone.switch_state(OverlayState::Standby);
                            glib::Propagation::Stop
                        }
                        Key::F2 => {
                            hud_clone.switch_state(OverlayState::Conversation);
                            glib::Propagation::Stop
                        }
                        Key::F3 => {
                            hud_clone.switch_state(OverlayState::TaskExecution);
                            glib::Propagation::Stop
                        }
                        Key::F4 => {
                            hud_clone.switch_state(OverlayState::SpaceshipInspection);
                            glib::Propagation::Stop
                        }
                        Key::space => {
                            hud_clone.viewport.scene.borrow_mut().select_next();
                            hud_clone.viewport.gl_area.queue_render();
                            glib::Propagation::Proceed
                        }
                        _ => glib::Propagation::Proceed,
                    }
                });
            }
            window.add_controller(key_ctrl);

            // Live Telemetry Refresh Timer (every 1000ms)
            let last_dt = Rc::new(RefCell::new(std::time::Instant::now()));
            {
                let hud_clone = hud.clone();
                let telemetry_clone = telemetry.clone();
                let last_dt = last_dt.clone();

                glib::timeout_add_local(std::time::Duration::from_millis(1000), move || {
                    let now = std::time::Instant::now();
                    let dt = now.duration_since(*last_dt.borrow()).as_secs_f32();
                    *last_dt.borrow_mut() = now;

                    let telemetry_inner = telemetry_clone.clone();
                    let hud_inner = hud_clone.clone();

                    glib::spawn_future_local(async move {
                        let mut coll = telemetry_inner.lock().await;
                        let snap = coll.snapshot();
                        hud_inner.update_telemetry_hud(&snap, dt);
                    });

                    glib::ControlFlow::Continue
                });
            }

            window.present();
        });

        app.run_with_args(&Vec::<String>::new());
    }
}
