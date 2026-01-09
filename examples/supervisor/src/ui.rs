// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! UI components for the Tasmota Supervisor application.

use std::collections::VecDeque;

use chrono::Local;
use egui::{Color32, RichText, Ui, Vec2, Widget};

use crate::device_config::{ConnectionStatus, DeviceState, Protocol};
use crate::device_model::DeviceModel;

// ============================================================================
// Console Log for HTTP Devices
// ============================================================================

/// Maximum number of entries to keep in the console log.
const CONSOLE_MAX_ENTRIES: usize = 50;

/// A single entry in the HTTP console log.
#[derive(Clone)]
pub struct ConsoleEntry {
    /// Timestamp of the entry (short format: HH:MM:SS)
    pub timestamp: String,
    /// The request that was sent (e.g., "`power_on()`")
    pub request: String,
    /// The result of the request
    pub result: ConsoleResult,
}

/// Result of an HTTP request.
#[derive(Clone)]
pub enum ConsoleResult {
    /// Successful response with details
    Success(String),
    /// Error response
    Error(String),
    /// Request is pending (waiting for response)
    #[allow(dead_code)] // Reserved for future async request tracking
    Pending,
}

impl ConsoleEntry {
    /// Creates a new pending console entry for a request.
    #[must_use]
    #[allow(dead_code)] // Reserved for future async request tracking
    pub fn new_request(request: &str) -> Self {
        Self {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            request: request.to_string(),
            result: ConsoleResult::Pending,
        }
    }

    /// Creates a new successful console entry.
    #[must_use]
    pub fn success(request: &str, response: &str) -> Self {
        Self {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            request: request.to_string(),
            result: ConsoleResult::Success(response.to_string()),
        }
    }

    /// Creates a new error console entry.
    #[must_use]
    pub fn error(request: &str, error: &str) -> Self {
        Self {
            timestamp: Local::now().format("%H:%M:%S").to_string(),
            request: request.to_string(),
            result: ConsoleResult::Error(error.to_string()),
        }
    }
}

/// Console log for an HTTP device.
#[derive(Clone, Default)]
pub struct ConsoleLog {
    entries: VecDeque<ConsoleEntry>,
}

impl ConsoleLog {
    /// Creates a new empty console log.
    #[must_use]
    pub fn new() -> Self {
        Self {
            entries: VecDeque::new(),
        }
    }

    /// Adds an entry to the log, removing old entries if necessary.
    pub fn push(&mut self, entry: ConsoleEntry) {
        self.entries.push_back(entry);
        while self.entries.len() > CONSOLE_MAX_ENTRIES {
            self.entries.pop_front();
        }
    }

    /// Clears all entries from the log.
    pub fn clear(&mut self) {
        self.entries.clear();
    }

    /// Returns an iterator over the entries.
    pub fn iter(&self) -> impl Iterator<Item = &ConsoleEntry> {
        self.entries.iter()
    }

    /// Returns true if the log is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

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
///
/// Dispatches to protocol-specific rendering based on the device's protocol.
/// For HTTP devices, requires a console log for request/response display.
pub fn device_card(
    ui: &mut Ui,
    device: &DeviceState,
    console_log: Option<&ConsoleLog>,
) -> DeviceCardResponse {
    match device.config.protocol {
        Protocol::Http => http_device_card(ui, device, console_log.unwrap_or(&ConsoleLog::new())),
        Protocol::Mqtt => mqtt_device_card(ui, device),
    }
}

/// Renders an HTTP device card with console-style request/response UI.
///
/// HTTP devices show controls that send requests and a console log
/// displaying the request/response history.
#[allow(clippy::too_many_lines)]
// UI rendering function with multiple sections - splitting would reduce readability
fn http_device_card(
    ui: &mut Ui,
    device: &DeviceState,
    console_log: &ConsoleLog,
) -> DeviceCardResponse {
    let mut response = DeviceCardResponse::default();

    egui::Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(ui.visuals().widgets.noninteractive.bg_stroke)
        .corner_radius(4.0)
        .inner_margin(12.0)
        .show(ui, |ui| {
            // Header row
            ui.horizontal(|ui| {
                // Protocol badge
                ui.label(
                    RichText::new("HTTP")
                        .small()
                        .color(Color32::WHITE)
                        .background_color(Color32::from_rgb(255, 140, 0)),
                );

                ui.vertical(|ui| {
                    ui.heading(&device.config.name);
                    ui.label(RichText::new(device.model().name()).small().weak());
                    let features: Vec<&str> = device.model().capabilities().features().collect();
                    if !features.is_empty() {
                        ui.label(RichText::new(features.join(" · ")).small().weak().italics());
                    }
                });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    // Connection controls
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

            ui.separator();

            // Commands section
            ui.label(RichText::new("Commands").strong());

            // Power controls
            ui.horizontal(|ui| {
                ui.label("Power:");
                if ui.button("ON").clicked() {
                    response.power_on_clicked = true;
                }
                if ui.button("OFF").clicked() {
                    response.power_off_clicked = true;
                }
                if ui.button("Toggle").clicked() {
                    response.power_toggle_clicked = true;
                }

                ui.separator();

                if ui.button("Status").clicked() {
                    response.status_query_clicked = true;
                }
            });

            // Dimmer control (if supported)
            if device.model().supports_dimming() {
                ui.horizontal(|ui| {
                    ui.label("Dimmer:");
                    // Read current value from device state, default to 50%
                    let mut dimmer_value = f32::from(device.dimmer_value().unwrap_or(50));
                    let slider_response =
                        ui.add(egui::Slider::new(&mut dimmer_value, 0.0..=100.0).suffix("%"));
                    // Send command when slider is released (no Send button needed)
                    if slider_response.drag_stopped() || slider_response.lost_focus() {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let dimmer = dimmer_value as u8;
                        response.dimmer_changed = Some(dimmer);
                    }
                });
            }

            // Color controls (if supported)
            if device.model().supports_color() {
                ui.horizontal(|ui| {
                    ui.label("Color:");
                    // Get current color from device state, convert to RGB
                    let (h, s, b) = device.hsb_color_values().unwrap_or((0, 100, 100));
                    let hsb = tasmor_lib::types::HsbColor::new(h, s, b).unwrap_or_default();
                    let rgb = hsb.to_rgb();
                    let mut color = [rgb.red(), rgb.green(), rgb.blue()];

                    // Color picker button
                    let picker_response = ui.color_edit_button_srgb(&mut color);
                    if picker_response.changed() {
                        response.rgb_color_changed =
                            Some(format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]));
                    }

                    // Hue slider as alternative
                    ui.label("H:");
                    let mut hue = f32::from(h);
                    let hue_response =
                        ui.add(egui::Slider::new(&mut hue, 0.0..=360.0).show_value(false));
                    if hue_response.drag_stopped() || hue_response.lost_focus() {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let hue_val = hue as u16;
                        response.hue_changed = Some((hue_val, s, b));
                    }
                });

                if device
                    .model()
                    .capabilities()
                    .supports_color_temperature_control()
                {
                    ui.horizontal(|ui| {
                        ui.label("Color Temp:");
                        // Read current CT from device state, default to 326 (neutral)
                        let mut ct_value = device.color_temp_mireds().map_or(326.0, f32::from);
                        let ct_response = ui
                            .add(egui::Slider::new(&mut ct_value, 153.0..=500.0).suffix(" mired"));
                        // Send command when slider is released
                        if ct_response.drag_stopped() || ct_response.lost_focus() {
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            let ct = ct_value as u16;
                            response.color_temp_changed = Some(ct);
                        }
                    });
                }

                // Scheme selector row
                ui.horizontal(|ui| {
                    ui.label("Scheme:");
                    let current_scheme = device.scheme_value().unwrap_or(0);
                    let scheme_names = ["Single", "Wakeup", "Cycle Up", "Cycle Down", "Random"];
                    egui::ComboBox::from_id_salt(("http_scheme", device.config.id))
                        .selected_text(*scheme_names.get(current_scheme as usize).unwrap_or(&"?"))
                        .show_ui(ui, |ui| {
                            for (idx, name) in scheme_names.iter().enumerate() {
                                #[allow(clippy::cast_possible_truncation)]
                                let idx_u8 = idx as u8;
                                if ui
                                    .selectable_label(current_scheme == idx_u8, *name)
                                    .clicked()
                                {
                                    response.scheme_changed = Some(idx_u8);
                                }
                            }
                        });

                    // Wakeup duration (only shown when scheme is Wakeup)
                    if current_scheme == 1 {
                        ui.label("Duration:");
                        let mut duration_secs =
                            f32::from(device.wakeup_duration_seconds().unwrap_or(60));
                        let duration_response =
                            ui.add(egui::Slider::new(&mut duration_secs, 1.0..=3000.0).suffix("s"));
                        if duration_response.drag_stopped() || duration_response.lost_focus() {
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            let secs = duration_secs as u16;
                            response.wakeup_duration_changed = Some(secs);
                        }
                    }
                });

                // Fade controls row
                ui.horizontal(|ui| {
                    let fade_on = device.fade_enabled().unwrap_or(false);
                    ui.label("Fade:");
                    if ui.selectable_label(fade_on, "On").clicked() {
                        response.fade_toggle_clicked = true;
                    }
                    if ui.selectable_label(!fade_on, "Off").clicked() {
                        response.fade_duration_changed = Some(0);
                    }

                    ui.label("Duration:");
                    let mut duration_value = f32::from(device.fade_duration_value().unwrap_or(10));
                    let duration_response =
                        ui.add(egui::Slider::new(&mut duration_value, 1.0..=40.0));
                    if duration_response.drag_stopped() || duration_response.lost_focus() {
                        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                        let duration = duration_value as u8;
                        response.fade_duration_changed = Some(duration);
                    }
                });
            }

            // Energy controls (if supported)
            if device.model().supports_energy_monitoring() {
                ui.horizontal(|ui| {
                    ui.label("Energy:");
                    if ui.button("Query").clicked() {
                        response.status_query_clicked = true;
                    }
                    if ui.button("Reset Total").clicked() {
                        response.energy_reset_clicked = true;
                    }
                });
            }

            ui.separator();

            // Console section
            ui.horizontal(|ui| {
                ui.label(RichText::new("Console").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Clear").clicked() {
                        response.console_clear_clicked = true;
                    }
                });
            });

            // Console log area
            egui::Frame::new()
                .fill(Color32::from_rgb(30, 30, 30))
                .corner_radius(4.0)
                .inner_margin(8.0)
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .max_height(150.0)
                        .stick_to_bottom(true)
                        .show(ui, |ui| {
                            ui.set_min_width(ui.available_width());

                            if console_log.is_empty() {
                                ui.label(
                                    RichText::new("No requests yet. Use the controls above.")
                                        .weak()
                                        .italics()
                                        .color(Color32::GRAY),
                                );
                            } else {
                                for entry in console_log.iter() {
                                    render_console_entry(ui, entry);
                                }
                            }
                        });
                });

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

/// Renders a single console entry.
fn render_console_entry(ui: &mut Ui, entry: &ConsoleEntry) {
    let timestamp_color = Color32::from_rgb(128, 128, 128);
    let request_color = Color32::from_rgb(100, 180, 255);

    ui.horizontal_wrapped(|ui| {
        // Timestamp
        ui.label(
            RichText::new(&entry.timestamp)
                .small()
                .color(timestamp_color),
        );

        // Request
        ui.label(RichText::new(format!("> {}", entry.request)).color(request_color));
    });

    // Result on next line with indentation
    match &entry.result {
        ConsoleResult::Success(msg) => {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(55.0); // Align with request
                ui.label(
                    RichText::new(format!("✓ {msg}"))
                        .small()
                        .color(Color32::from_rgb(100, 200, 100)),
                );
            });
        }
        ConsoleResult::Error(msg) => {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(55.0);
                ui.label(
                    RichText::new(format!("✗ {msg}"))
                        .small()
                        .color(Color32::from_rgb(255, 100, 100)),
                );
            });
        }
        ConsoleResult::Pending => {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(55.0);
                ui.label(
                    RichText::new("⏳ pending...")
                        .small()
                        .color(Color32::YELLOW),
                );
            });
        }
    }

    ui.add_space(4.0);
}

/// Renders an MQTT device card with real-time state display.
///
/// MQTT devices show live state updates and controls that reflect
/// the current device state.
#[allow(clippy::too_many_lines)]
// UI rendering function with multiple sections - splitting would reduce readability
fn mqtt_device_card(ui: &mut Ui, device: &DeviceState) -> DeviceCardResponse {
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

                // Protocol badge for MQTT
                if device.status() == ConnectionStatus::Connected {
                    ui.label(
                        RichText::new("MQTT")
                            .small()
                            .color(Color32::WHITE)
                            .background_color(Color32::from_rgb(34, 139, 34)),
                    );
                }

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
                        ui.label("Color:");
                        // Get current color from device state, convert to RGB
                        let (h, s, b) = device.hsb_color_values().unwrap_or((0, 100, 100));
                        let hsb = tasmor_lib::types::HsbColor::new(h, s, b).unwrap_or_default();
                        let rgb = hsb.to_rgb();
                        let mut color = [rgb.red(), rgb.green(), rgb.blue()];

                        // Color picker button
                        let picker_response = ui.color_edit_button_srgb(&mut color);
                        if picker_response.changed() {
                            response.rgb_color_changed =
                                Some(format!("#{:02X}{:02X}{:02X}", color[0], color[1], color[2]));
                        }

                        // Hue slider as alternative
                        ui.label("H:");
                        let mut hue_value = f32::from(h);
                        let hue_response = ui
                            .add(egui::Slider::new(&mut hue_value, 0.0..=360.0).show_value(false));
                        if hue_response.drag_stopped() || hue_response.lost_focus() {
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            let hue = hue_value as u16;
                            response.hue_changed = Some((hue, s, b));
                        }

                        // Color temperature slider (for RGBCCT lights)
                        if device
                            .model()
                            .capabilities()
                            .supports_color_temperature_control()
                        {
                            ui.label("CT:");
                            let mut ct_value = f32::from(device.color_temp_mireds().unwrap_or(326));
                            let ct_response = ui.add(
                                egui::Slider::new(&mut ct_value, 153.0..=500.0).show_value(false),
                            );
                            if ct_response.drag_stopped() || ct_response.lost_focus() {
                                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                                let ct = ct_value as u16;
                                response.color_temp_changed = Some(ct);
                            }
                        }
                    });

                    // Scheme selector row
                    ui.horizontal(|ui| {
                        ui.label("Scheme:");
                        let current_scheme = device.scheme_value().unwrap_or(0);
                        let scheme_names = ["Single", "Wakeup", "Cycle Up", "Cycle Down", "Random"];
                        egui::ComboBox::from_id_salt(("mqtt_scheme", device.config.id))
                            .selected_text(
                                *scheme_names.get(current_scheme as usize).unwrap_or(&"?"),
                            )
                            .show_ui(ui, |ui| {
                                for (idx, name) in scheme_names.iter().enumerate() {
                                    #[allow(clippy::cast_possible_truncation)]
                                    let idx_u8 = idx as u8;
                                    if ui
                                        .selectable_label(current_scheme == idx_u8, *name)
                                        .clicked()
                                    {
                                        response.scheme_changed = Some(idx_u8);
                                    }
                                }
                            });

                        // Wakeup duration (only shown when scheme is Wakeup)
                        if current_scheme == 1 {
                            ui.label("Duration:");
                            let mut duration_secs =
                                f32::from(device.wakeup_duration_seconds().unwrap_or(60));
                            let duration_response = ui.add(
                                egui::Slider::new(&mut duration_secs, 1.0..=3000.0).suffix("s"),
                            );
                            if duration_response.drag_stopped() || duration_response.lost_focus() {
                                #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                                let secs = duration_secs as u16;
                                response.wakeup_duration_changed = Some(secs);
                            }
                        }
                    });

                    // Fade controls row
                    ui.horizontal(|ui| {
                        let fade_on = device.fade_enabled().unwrap_or(false);
                        ui.label("Fade:");
                        if ui.selectable_label(fade_on, "On").clicked() {
                            response.fade_toggle_clicked = true;
                        }
                        if ui.selectable_label(!fade_on, "Off").clicked() {
                            response.fade_duration_changed = Some(0);
                        }

                        ui.label("Duration:");
                        let mut duration_value =
                            f32::from(device.fade_duration_value().unwrap_or(10));
                        let duration_response =
                            ui.add(egui::Slider::new(&mut duration_value, 1.0..=40.0));
                        if duration_response.drag_stopped() || duration_response.lost_focus() {
                            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
                            let duration = duration_value as u8;
                            response.fade_duration_changed = Some(duration);
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
    /// Power ON button was clicked (HTTP only)
    pub power_on_clicked: bool,
    /// Power OFF button was clicked (HTTP only)
    pub power_off_clicked: bool,
    /// Dimmer slider changed
    pub dimmer_changed: Option<u8>,
    /// HSB color hue changed (hue, saturation, brightness)
    pub hue_changed: Option<(u16, u8, u8)>,
    /// Color temperature changed (in mireds)
    pub color_temp_changed: Option<u16>,
    /// Energy reset button was clicked
    pub energy_reset_clicked: bool,
    /// Status query button was clicked (HTTP only)
    pub status_query_clicked: bool,
    /// Console clear button was clicked (HTTP only)
    pub console_clear_clicked: bool,
    /// Scheme changed (0-4)
    pub scheme_changed: Option<u8>,
    /// Wakeup duration changed (in seconds)
    pub wakeup_duration_changed: Option<u16>,
    /// RGB color changed (hex string like "#FF5733")
    pub rgb_color_changed: Option<String>,
    /// Fade toggle button was clicked
    pub fade_toggle_clicked: bool,
    /// Fade duration changed (1-40)
    pub fade_duration_changed: Option<u8>,
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
