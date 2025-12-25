// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! UI components for the Tasmota Supervisor application.

use egui::{Color32, RichText, Ui, Vec2, Widget};

use crate::device_config::{ConnectionStatus, DeviceState};
use crate::device_model::DeviceModel;

/// Creates a power toggle switch widget.
fn power_toggle(on: &mut bool, enabled: bool) -> impl Widget + '_ {
    move |ui: &mut Ui| -> egui::Response {
        let desired_size = Vec2::new(50.0, 24.0);
        let (rect, response) = ui.allocate_exact_size(desired_size, egui::Sense::click());

        if response.clicked() && enabled {
            *on = !*on;
        }

        if ui.is_rect_visible(rect) {
            let visuals = ui.style().interact(&response);

            // Background
            let bg_color = if !enabled {
                Color32::DARK_GRAY
            } else if *on {
                Color32::from_rgb(34, 139, 34) // Forest green
            } else {
                Color32::from_rgb(139, 69, 69) // Dark red
            };

            ui.painter().rect_filled(rect, 12.0, bg_color);
            ui.painter()
                .rect_stroke(rect, 12.0, visuals.bg_stroke, egui::StrokeKind::Outside);

            // Knob
            let knob_radius = 10.0;
            let knob_x = if *on {
                rect.right() - knob_radius - 2.0
            } else {
                rect.left() + knob_radius + 2.0
            };
            let knob_center = egui::pos2(knob_x, rect.center().y);

            ui.painter()
                .circle_filled(knob_center, knob_radius, Color32::WHITE);

            // Text label
            let text = if *on { "ON" } else { "OFF" };
            let text_color = Color32::WHITE;
            let text_pos = if *on {
                egui::pos2(rect.left() + 6.0, rect.center().y)
            } else {
                egui::pos2(rect.right() - 6.0, rect.center().y)
            };

            let anchor = if *on {
                egui::Align2::LEFT_CENTER
            } else {
                egui::Align2::RIGHT_CENTER
            };

            ui.painter().text(
                text_pos,
                anchor,
                text,
                egui::FontId::proportional(10.0),
                text_color,
            );
        }

        response
    }
}

/// Renders device information in a compact card format.
#[allow(clippy::too_many_lines)]
// UI rendering function with multiple sections - splitting would reduce readability
pub fn device_card(ui: &mut Ui, device: &DeviceState) -> DeviceCardResponse {
    let mut response = DeviceCardResponse::default();

    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // Status indicator
                let status_color = device.status().color();
                ui.label(RichText::new("●").color(status_color).size(20.0));

                ui.vertical(|ui| {
                    // Device name and model
                    ui.heading(&device.config.name);
                    ui.label(RichText::new(device.model().name()).small().weak());
                    // Show device capabilities
                    let features: Vec<&str> = device.model().capabilities().features().collect();
                    if !features.is_empty() {
                        ui.label(RichText::new(features.join(" · ")).small().weak().italics());
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Action buttons
                    match device.status() {
                        ConnectionStatus::Disconnected | ConnectionStatus::Error => {
                            if ui.button("Connect").clicked() {
                                response.connect_clicked = true;
                            }
                        }
                        ConnectionStatus::Connected => {
                            if ui.button("Disconnect").clicked() {
                                response.disconnect_clicked = true;
                            }

                            if ui.button("⟳").clicked() {
                                response.refresh_clicked = true;
                            }
                        }
                        ConnectionStatus::Connecting => {
                            ui.spinner();
                        }
                    }

                    if ui.button("⚙").clicked() {
                        response.settings_clicked = true;
                    }

                    if ui.button("🗑").clicked() {
                        response.delete_clicked = true;
                    }
                });
            });

            // Device details when connected
            if device.status() == ConnectionStatus::Connected {
                ui.separator();

                ui.horizontal(|ui| {
                    // Power toggle with state indicator
                    let power_on = device.is_power_on().unwrap_or(false);
                    let mut power_state = power_on;

                    // Custom styled toggle
                    let toggle_response = ui.add(power_toggle(
                        &mut power_state,
                        device.is_power_on().is_some(),
                    ));

                    if toggle_response.clicked() && device.is_power_on().is_some() {
                        response.power_toggle_clicked = true;
                    }

                    // Show unknown state indicator if power state is not known
                    if device.is_power_on().is_none() {
                        ui.label(RichText::new("?").color(Color32::GRAY).strong());
                    }

                    // Dimmer control for lights
                    if device.model().supports_dimming() {
                        ui.label("Brightness:");
                        let mut dimmer_value = f32::from(device.dimmer_value().unwrap_or(100));
                        let slider_response =
                            ui.add(egui::Slider::new(&mut dimmer_value, 0.0..=100.0).suffix("%"));
                        // Only send command when slider is released, not during dragging
                        if slider_response.drag_stopped() || slider_response.lost_focus() {
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            // Slider is constrained to 0-100, truncation and sign loss are safe
                            let dimmer = dimmer_value as u8;
                            response.dimmer_changed = Some(dimmer);
                        }
                    }
                });

                // Energy monitoring for plugs (on separate row for better readability)
                if device.model().supports_energy_monitoring() {
                    render_energy_section(ui, device, &mut response);
                }

                // Color controls on a separate row
                if device.model().supports_color() {
                    ui.horizontal(|ui| {
                        // Hue slider for color selection
                        ui.label("Color:");
                        let (h, s, b) = device.hsb_color_values().unwrap_or((0, 100, 100));
                        let mut hue_value = f32::from(h);
                        let hue_response =
                            ui.add(egui::Slider::new(&mut hue_value, 0.0..=360.0).suffix("°"));
                        // Only send command when slider is released
                        if hue_response.drag_stopped() || hue_response.lost_focus() {
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            // Slider is constrained to 0-360, truncation and sign loss are safe
                            let hue = hue_value as u16;
                            response.hue_changed = Some((hue, s, b));
                        }

                        // Color temperature slider (for RGBCCT lights)
                        if device
                            .model()
                            .capabilities()
                            .supports_color_temperature_control()
                        {
                            ui.label("Temp:");
                            let mut ct_value = f32::from(device.color_temp_mireds().unwrap_or(326));
                            let ct_response = ui.add(
                                egui::Slider::new(&mut ct_value, 153.0..=500.0).suffix(" mired"),
                            );
                            // Only send command when slider is released
                            if ct_response.drag_stopped() || ct_response.lost_focus() {
                                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                                // Slider is constrained to 153-500, truncation and sign loss are safe
                                let ct = ct_value as u16;
                                response.color_temp_changed = Some(ct);
                            }
                        }
                    });
                }
            }

            // Error display
            if let Some(error) = &device.error {
                ui.separator();
                ui.label(
                    RichText::new(format!("❌ {error}"))
                        .color(Color32::RED)
                        .small(),
                );
            }
        });

    response
}

/// Response from a device card interaction.
#[derive(Default)]
#[allow(clippy::struct_excessive_bools)]
// UI response struct naturally has many boolean flags for different user actions
pub struct DeviceCardResponse {
    /// Connect button was clicked
    pub connect_clicked: bool,
    /// Disconnect button was clicked
    pub disconnect_clicked: bool,
    /// Refresh button was clicked
    pub refresh_clicked: bool,
    /// Settings button was clicked
    pub settings_clicked: bool,
    /// Delete button was clicked
    pub delete_clicked: bool,
    /// Power toggle button was clicked
    pub power_toggle_clicked: bool,
    /// Dimmer slider changed
    pub dimmer_changed: Option<u8>,
    /// HSB color hue changed (hue, saturation, brightness)
    pub hue_changed: Option<(u16, u8, u8)>,
    /// Color temperature changed (in mireds)
    pub color_temp_changed: Option<u16>,
    /// Energy reset button was clicked
    pub energy_reset_clicked: bool,
}

/// Renders the add device dialog.
pub fn add_device_dialog(ui: &mut Ui, state: &mut AddDeviceDialogState) -> AddDeviceDialogResponse {
    let mut response = AddDeviceDialogResponse::default();

    ui.heading("Add Device");
    ui.separator();

    // Device name
    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut state.name);
    });

    // Device model
    ui.horizontal(|ui| {
        ui.label("Model:");
        egui::ComboBox::from_id_salt("device_model")
            .selected_text(state.model.name())
            .show_ui(ui, |ui| {
                for model in DeviceModel::all() {
                    ui.selectable_value(&mut state.model, *model, model.name());
                }
            });
    });

    ui.separator();

    // Protocol selection
    ui.horizontal(|ui| {
        ui.label("Protocol:");
        ui.radio_value(&mut state.use_http, true, "HTTP");
        ui.radio_value(&mut state.use_http, false, "MQTT");
    });

    if state.use_http {
        // HTTP configuration
        ui.horizontal(|ui| {
            ui.label("Host:");
            ui.text_edit_singleline(&mut state.http_host);
        });
    } else {
        // MQTT configuration
        ui.horizontal(|ui| {
            ui.label("Broker:");
            ui.text_edit_singleline(&mut state.mqtt_broker);
        });

        ui.horizontal(|ui| {
            ui.label("Topic:");
            ui.text_edit_singleline(&mut state.mqtt_topic);
        });
    }

    ui.separator();

    // Optional authentication
    ui.checkbox(&mut state.use_auth, "Use Authentication");

    if state.use_auth {
        ui.horizontal(|ui| {
            ui.label("Username:");
            ui.text_edit_singleline(&mut state.username);
        });

        ui.horizontal(|ui| {
            ui.label("Password:");
            ui.add(egui::TextEdit::singleline(&mut state.password).password(true));
        });
    }

    ui.separator();

    // Action buttons
    ui.horizontal(|ui| {
        if ui.button("Add").clicked() {
            response.add_clicked = true;
        }

        if ui.button("Cancel").clicked() {
            response.cancel_clicked = true;
        }
    });

    response
}

/// State for the add device dialog.
#[derive(Default)]
pub struct AddDeviceDialogState {
    pub name: String,
    pub model: DeviceModel,
    pub use_http: bool,
    pub http_host: String,
    pub mqtt_broker: String,
    pub mqtt_topic: String,
    pub use_auth: bool,
    pub username: String,
    pub password: String,
}

impl AddDeviceDialogState {
    /// Creates a new dialog state with default values.
    #[must_use]
    pub fn new() -> Self {
        Self {
            name: String::new(),
            model: DeviceModel::AthomBulb5W7W,
            use_http: true,
            http_host: String::new(),
            mqtt_broker: "mqtt://192.168.1.50:1883".to_string(),
            mqtt_topic: String::new(),
            use_auth: false,
            username: String::new(),
            password: String::new(),
        }
    }

    /// Validates the dialog input.
    ///
    /// # Errors
    ///
    /// Returns an error message if validation fails.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Device name is required".to_string());
        }

        if self.use_http {
            if self.http_host.is_empty() {
                return Err("HTTP host is required".to_string());
            }
        } else {
            if self.mqtt_broker.is_empty() {
                return Err("MQTT broker is required".to_string());
            }
            if self.mqtt_topic.is_empty() {
                return Err("MQTT topic is required".to_string());
            }
        }

        if self.use_auth && (self.username.is_empty() || self.password.is_empty()) {
            return Err(
                "Username and password are required when authentication is enabled".to_string(),
            );
        }

        Ok(())
    }
}

/// Response from the add device dialog.
#[derive(Default)]
pub struct AddDeviceDialogResponse {
    /// Add button was clicked
    pub add_clicked: bool,
    /// Cancel button was clicked
    pub cancel_clicked: bool,
}

/// Renders the edit device dialog.
pub fn edit_device_dialog(
    ui: &mut Ui,
    state: &mut EditDeviceDialogState,
) -> EditDeviceDialogResponse {
    let mut response = EditDeviceDialogResponse::default();

    ui.heading("Edit Device");
    ui.separator();

    // Device name
    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut state.name);
    });

    // Device model (read-only display)
    ui.horizontal(|ui| {
        ui.label("Model:");
        ui.label(state.model.name());
    });

    // Protocol (read-only display)
    ui.horizontal(|ui| {
        ui.label("Protocol:");
        ui.label(if state.use_http { "HTTP" } else { "MQTT" });
    });

    ui.separator();

    if state.use_http {
        // HTTP configuration
        ui.horizontal(|ui| {
            ui.label("Host:");
            ui.text_edit_singleline(&mut state.http_host);
        });
    } else {
        // MQTT configuration
        ui.horizontal(|ui| {
            ui.label("Broker:");
            ui.text_edit_singleline(&mut state.mqtt_broker);
        });

        ui.horizontal(|ui| {
            ui.label("Topic:");
            ui.text_edit_singleline(&mut state.mqtt_topic);
        });
    }

    ui.separator();

    // Optional authentication
    ui.checkbox(&mut state.use_auth, "Use Authentication");

    if state.use_auth {
        ui.horizontal(|ui| {
            ui.label("Username:");
            ui.text_edit_singleline(&mut state.username);
        });

        ui.horizontal(|ui| {
            ui.label("Password:");
            ui.add(egui::TextEdit::singleline(&mut state.password).password(true));
        });
    }

    ui.separator();

    // Action buttons
    ui.horizontal(|ui| {
        if ui.button("Save").clicked() {
            response.save_clicked = true;
        }

        if ui.button("Cancel").clicked() {
            response.cancel_clicked = true;
        }
    });

    response
}

/// State for the edit device dialog.
#[derive(Clone)]
pub struct EditDeviceDialogState {
    /// Device ID being edited
    pub device_id: uuid::Uuid,
    /// Device name
    pub name: String,
    /// Device model (not editable)
    pub model: DeviceModel,
    /// Whether using HTTP protocol
    pub use_http: bool,
    /// HTTP host
    pub http_host: String,
    /// MQTT broker URL
    pub mqtt_broker: String,
    /// MQTT topic
    pub mqtt_topic: String,
    /// Whether authentication is enabled
    pub use_auth: bool,
    /// Username for authentication
    pub username: String,
    /// Password for authentication
    pub password: String,
}

impl EditDeviceDialogState {
    /// Creates a new edit dialog state from an existing device configuration.
    #[must_use]
    pub fn from_config(config: &crate::device_config::DeviceConfig) -> Self {
        let use_http = config.protocol == crate::device_config::Protocol::Http;
        Self {
            device_id: config.id,
            name: config.name.clone(),
            model: config.model,
            use_http,
            http_host: if use_http {
                config.host.clone()
            } else {
                String::new()
            },
            mqtt_broker: if use_http {
                String::new()
            } else {
                config.host.clone()
            },
            mqtt_topic: config.topic.clone().unwrap_or_default(),
            use_auth: config.username.is_some(),
            username: config.username.clone().unwrap_or_default(),
            password: config.password.clone().unwrap_or_default(),
        }
    }

    /// Validates the dialog input.
    ///
    /// # Errors
    ///
    /// Returns an error message if validation fails.
    pub fn validate(&self) -> Result<(), String> {
        if self.name.is_empty() {
            return Err("Device name is required".to_string());
        }

        if self.use_http {
            if self.http_host.is_empty() {
                return Err("HTTP host is required".to_string());
            }
        } else {
            if self.mqtt_broker.is_empty() {
                return Err("MQTT broker is required".to_string());
            }
            if self.mqtt_topic.is_empty() {
                return Err("MQTT topic is required".to_string());
            }
        }

        if self.use_auth && (self.username.is_empty() || self.password.is_empty()) {
            return Err(
                "Username and password are required when authentication is enabled".to_string(),
            );
        }

        Ok(())
    }
}

/// Response from the edit device dialog.
#[derive(Default)]
pub struct EditDeviceDialogResponse {
    /// Save button was clicked
    pub save_clicked: bool,
    /// Cancel button was clicked
    pub cancel_clicked: bool,
}

/// Renders the energy monitoring section for devices that support it.
fn render_energy_section(ui: &mut Ui, device: &DeviceState, response: &mut DeviceCardResponse) {
    ui.horizontal(|ui| {
        // Main power readings
        if let Some(power) = device.power_consumption_watts() {
            ui.label(format!("⚡ {power:.0} W"));
        }

        if let Some(voltage) = device.voltage() {
            ui.label(format!("| {voltage:.0} V"));
        }

        if let Some(current) = device.current() {
            ui.label(format!("| {current:.2} A"));
        }

        if let Some(pf) = device.power_factor() {
            ui.label(format!("| PF: {pf:.2}"));
        }
    });

    // Secondary readings (apparent/reactive power and consumption)
    let has_secondary = device.apparent_power().is_some()
        || device.reactive_power().is_some()
        || device.energy_today().is_some()
        || device.energy_yesterday().is_some()
        || device.energy_total().is_some();

    if has_secondary {
        ui.horizontal(|ui| {
            if let Some(apparent) = device.apparent_power() {
                ui.label(format!("{apparent:.0} VA"));
            }

            if let Some(reactive) = device.reactive_power() {
                ui.label(format!("| {reactive:.0} VAr"));
            }

            if let Some(today) = device.energy_today() {
                ui.label(format!("| Today: {today:.2} kWh"));
            }

            if let Some(yesterday) = device.energy_yesterday() {
                ui.label(format!("| Yesterday: {yesterday:.2} kWh"));
            }
        });

        // Total energy on its own line (with start time for context)
        if let Some(total) = device.energy_total() {
            ui.horizontal(|ui| {
                let total_text = if let Some(start_time) = device.total_start_time() {
                    // Use chrono's format for date and time display
                    let formatted_datetime = start_time.naive().format("%Y-%m-%d %H:%M");
                    format!("Total: {total:.1} kWh (since {formatted_datetime})")
                } else {
                    format!("Total: {total:.1} kWh")
                };
                ui.label(total_text);

                // Reset button
                if ui
                    .small_button("Reset")
                    .on_hover_text("Reset total energy counter")
                    .clicked()
                {
                    response.energy_reset_clicked = true;
                }
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_state_validation() {
        let mut state = AddDeviceDialogState::new();

        // Empty name should fail
        assert!(state.validate().is_err());

        // Valid HTTP configuration
        state.name = "Test Device".to_string();
        state.http_host = "192.168.1.100".to_string();
        assert!(state.validate().is_ok());

        // Invalid MQTT configuration (empty broker)
        state.use_http = false;
        assert!(state.validate().is_err());

        // Valid MQTT configuration
        state.mqtt_broker = "mqtt://broker:1883".to_string();
        state.mqtt_topic = "tasmota_device".to_string();
        assert!(state.validate().is_ok());

        // Authentication required but not provided
        state.use_auth = true;
        assert!(state.validate().is_err());

        // Valid authentication
        state.username = "user".to_string();
        state.password = "pass".to_string();
        assert!(state.validate().is_ok());
    }
}
