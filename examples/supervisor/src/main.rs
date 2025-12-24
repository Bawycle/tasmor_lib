// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Tasmota Supervisor - GUI application for monitoring and controlling Tasmota devices.
//!
//! This application provides a cross-platform GUI for managing multiple Tasmota devices
//! via HTTP and MQTT protocols. It supports various device models including smart bulbs
//! and smart plugs with energy monitoring.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod device_config;
mod device_manager;
mod device_model;
mod persistence;
mod ui;

use eframe::egui;
use uuid::Uuid;

use device_config::{DeviceConfig, DeviceState, Protocol};
use device_manager::{DeviceCommand, DeviceEvent, DeviceManager};
use persistence::AppConfig;
use ui::{AddDeviceDialogState, DeviceCardResponse, EditDeviceDialogState};

/// Main application state.
struct TasmotaSupervisor {
    /// Device manager handling async communication
    device_manager: DeviceManager,
    /// Persisted application configuration
    app_config: AppConfig,
    /// List of devices
    devices: Vec<DeviceState>,
    /// Whether the add device dialog is open
    show_add_dialog: bool,
    /// State for the add device dialog
    add_dialog_state: AddDeviceDialogState,
    /// State for the edit device dialog (None if not open)
    edit_dialog_state: Option<EditDeviceDialogState>,
    /// Error message to display
    error_message: Option<String>,
}

impl TasmotaSupervisor {
    /// Creates a new application instance.
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let device_manager = DeviceManager::new();
        let app_config = AppConfig::load();

        // Add saved devices to the manager
        for config in &app_config.devices {
            let _ = device_manager.send_command(DeviceCommand::AddDevice(config.clone()));
        }

        Self {
            device_manager,
            app_config,
            devices: Vec::new(),
            show_add_dialog: false,
            add_dialog_state: AddDeviceDialogState::new(),
            edit_dialog_state: None,
            error_message: None,
        }
    }

    /// Handles device card interactions.
    fn handle_device_card_response(&mut self, device_id: Uuid, response: &DeviceCardResponse) {
        if response.connect_clicked {
            let _ = self
                .device_manager
                .send_command(DeviceCommand::Connect(device_id));
        }

        if response.disconnect_clicked {
            let _ = self
                .device_manager
                .send_command(DeviceCommand::Disconnect(device_id));
        }

        if response.refresh_clicked {
            let _ = self
                .device_manager
                .send_command(DeviceCommand::RefreshStatus(device_id));
        }

        if response.settings_clicked {
            // Find the device config and open edit dialog
            if let Some(device) = self.devices.iter().find(|d| d.config.id == device_id) {
                self.edit_dialog_state = Some(EditDeviceDialogState::from_config(&device.config));
                self.error_message = None;
            }
        }

        if response.delete_clicked {
            // Remove from persistent config
            self.app_config.remove_device(device_id);

            let _ = self
                .device_manager
                .send_command(DeviceCommand::RemoveDevice(device_id));
        }

        if response.power_toggle_clicked {
            let _ = self
                .device_manager
                .send_command(DeviceCommand::TogglePower(device_id));
        }

        if let Some(dimmer) = response.dimmer_changed {
            let _ = self
                .device_manager
                .send_command(DeviceCommand::SetDimmer(device_id, dimmer));
        }

        if let Some((hue, sat, bri)) = response.hue_changed {
            let _ = self
                .device_manager
                .send_command(DeviceCommand::SetHsbColor(device_id, hue, sat, bri));
        }

        if let Some(ct) = response.color_temp_changed {
            let _ = self
                .device_manager
                .send_command(DeviceCommand::SetColorTemp(device_id, ct));
        }
    }

    /// Processes pending device events.
    fn process_events(&mut self, ctx: &egui::Context) {
        let rt = tokio::runtime::Handle::current();

        // Process all pending events
        while let Some(event) = rt.block_on(self.device_manager.poll_event()) {
            match event {
                DeviceEvent::DeviceAdded | DeviceEvent::StateUpdated => {
                    // Update device list
                    self.devices = rt.block_on(self.device_manager.devices());
                }

                DeviceEvent::DeviceRemoved => {
                    self.devices = rt.block_on(self.device_manager.devices());
                }

                DeviceEvent::Error(error) => {
                    self.error_message = Some(error);
                }
            }

            // Request repaint to update UI
            ctx.request_repaint();
        }
    }

    /// Handles add device dialog.
    fn show_add_device_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("Add Device")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let response = ui::add_device_dialog(ui, &mut self.add_dialog_state);

                if response.add_clicked {
                    match self.add_dialog_state.validate() {
                        Ok(()) => {
                            let config = self.create_device_config();

                            // Save to persistent config
                            self.app_config.add_device(config.clone());

                            let _ = self
                                .device_manager
                                .send_command(DeviceCommand::AddDevice(config));

                            self.show_add_dialog = false;
                            self.add_dialog_state = AddDeviceDialogState::new();
                            self.error_message = None;
                        }
                        Err(e) => {
                            self.error_message = Some(e);
                        }
                    }
                }

                if response.cancel_clicked {
                    self.show_add_dialog = false;
                    self.add_dialog_state = AddDeviceDialogState::new();
                    self.error_message = None;
                }

                // Display error if any
                if let Some(error) = &self.error_message {
                    ui.separator();
                    ui.colored_label(egui::Color32::RED, error);
                }
            });
    }

    /// Handles edit device dialog.
    fn show_edit_device_dialog(&mut self, ctx: &egui::Context) {
        // Clone the state to avoid borrow conflicts
        let Some(mut state) = self.edit_dialog_state.clone() else {
            return;
        };

        let device_id = state.device_id;
        let mut save_clicked = false;
        let mut cancel_clicked = false;
        let mut validation_error: Option<String> = None;

        egui::Window::new("Edit Device")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                let response = ui::edit_device_dialog(ui, &mut state);

                if response.save_clicked {
                    match state.validate() {
                        Ok(()) => save_clicked = true,
                        Err(e) => validation_error = Some(e),
                    }
                }

                if response.cancel_clicked {
                    cancel_clicked = true;
                }

                // Display error if any
                if let Some(error) = &self.error_message {
                    ui.separator();
                    ui.colored_label(egui::Color32::RED, error);
                }
            });

        // Handle actions after the window is closed
        if save_clicked {
            let updated_config = Self::create_updated_config(&state);

            // Update in persistent config
            self.app_config.update_device(updated_config.clone());

            // Update in device manager (remove and re-add)
            let _ = self
                .device_manager
                .send_command(DeviceCommand::RemoveDevice(device_id));
            let _ = self
                .device_manager
                .send_command(DeviceCommand::AddDevice(updated_config));

            self.edit_dialog_state = None;
            self.error_message = None;
        } else if cancel_clicked {
            self.edit_dialog_state = None;
            self.error_message = None;
        } else if let Some(error) = validation_error {
            self.error_message = Some(error);
            // Update state with any changes made in the dialog
            self.edit_dialog_state = Some(state);
        } else {
            // Update state with any changes made in the dialog
            self.edit_dialog_state = Some(state);
        }
    }

    /// Creates an updated device configuration from edit dialog state.
    fn create_updated_config(state: &EditDeviceDialogState) -> DeviceConfig {
        let mut config = if state.use_http {
            DeviceConfig {
                id: state.device_id,
                name: state.name.clone(),
                model: state.model,
                protocol: Protocol::Http,
                host: state.http_host.clone(),
                topic: None,
                username: None,
                password: None,
            }
        } else {
            DeviceConfig {
                id: state.device_id,
                name: state.name.clone(),
                model: state.model,
                protocol: Protocol::Mqtt,
                host: state.mqtt_broker.clone(),
                topic: Some(state.mqtt_topic.clone()),
                username: None,
                password: None,
            }
        };

        if state.use_auth {
            config.username = Some(state.username.clone());
            config.password = Some(state.password.clone());
        }

        config
    }

    /// Creates a device configuration from dialog state.
    fn create_device_config(&self) -> DeviceConfig {
        let state = &self.add_dialog_state;

        let mut config = if state.use_http {
            DeviceConfig::new_http(state.name.clone(), state.model, state.http_host.clone())
        } else {
            DeviceConfig::new_mqtt(
                state.name.clone(),
                state.model,
                state.mqtt_broker.clone(),
                state.mqtt_topic.clone(),
            )
        };

        if state.use_auth {
            config = config.with_credentials(state.username.clone(), state.password.clone());
        }

        config
    }
}

impl eframe::App for TasmotaSupervisor {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process device events
        self.process_events(ctx);

        // Top panel with actions
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Tasmota Supervisor");
                ui.separator();

                if ui.button("➕ Add Device").clicked() {
                    self.show_add_dialog = true;
                }

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("Devices: {}", self.devices.len()));
                });
            });
        });

        // Central panel with device list
        egui::CentralPanel::default().show(ctx, |ui| {
            if self.devices.is_empty() {
                ui.vertical_centered(|ui| {
                    ui.add_space(100.0);
                    ui.heading("No devices configured");
                    ui.label("Click 'Add Device' to get started");
                });
            } else {
                // Collect responses first to avoid borrow conflict
                let responses: Vec<_> = egui::ScrollArea::vertical()
                    .show(ui, |ui| {
                        self.devices
                            .iter()
                            .map(|device| {
                                let response = ui::device_card(ui, device);
                                ui.add_space(8.0);
                                (device.config.id, response)
                            })
                            .collect()
                    })
                    .inner;

                for (device_id, response) in &responses {
                    self.handle_device_card_response(*device_id, response);
                }
            }
        });

        // Show add device dialog if open
        if self.show_add_dialog {
            self.show_add_device_dialog(ctx);
        }

        // Show edit device dialog if open
        if self.edit_dialog_state.is_some() {
            self.show_edit_device_dialog(ctx);
        }
    }
}

fn main() -> eframe::Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();

    // Create tokio runtime
    let rt = tokio::runtime::Runtime::new().expect("Failed to create tokio runtime");
    let _guard = rt.enter();

    // Run the application
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_min_inner_size([400.0, 300.0])
            .with_title("Tasmota Supervisor"),
        ..Default::default()
    };

    eframe::run_native(
        "Tasmota Supervisor",
        native_options,
        Box::new(|cc| Ok(Box::new(TasmotaSupervisor::new(cc)))),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device_config::Protocol;

    #[tokio::test]
    async fn create_http_config() {
        let mut app = TasmotaSupervisor {
            device_manager: DeviceManager::new(),
            app_config: AppConfig::default(),
            devices: Vec::new(),
            show_add_dialog: false,
            add_dialog_state: AddDeviceDialogState::new(),
            edit_dialog_state: None,
            error_message: None,
        };

        app.add_dialog_state.name = "Test Bulb".to_string();
        app.add_dialog_state.model = device_model::DeviceModel::AthomBulb5W7W;
        app.add_dialog_state.use_http = true;
        app.add_dialog_state.http_host = "192.168.1.100".to_string();

        let config = app.create_device_config();

        assert_eq!(config.name, "Test Bulb");
        assert_eq!(config.model, device_model::DeviceModel::AthomBulb5W7W);
        assert_eq!(config.protocol, Protocol::Http);
        assert_eq!(config.host, "192.168.1.100");
    }

    #[tokio::test]
    async fn create_mqtt_config() {
        let mut app = TasmotaSupervisor {
            device_manager: DeviceManager::new(),
            app_config: AppConfig::default(),
            devices: Vec::new(),
            show_add_dialog: false,
            add_dialog_state: AddDeviceDialogState::new(),
            edit_dialog_state: None,
            error_message: None,
        };

        app.add_dialog_state.name = "Test Plug".to_string();
        app.add_dialog_state.model = device_model::DeviceModel::NousA1T;
        app.add_dialog_state.use_http = false;
        app.add_dialog_state.mqtt_broker = "mqtt://192.168.1.50:1883".to_string();
        app.add_dialog_state.mqtt_topic = "tasmota_plug".to_string();

        let config = app.create_device_config();

        assert_eq!(config.name, "Test Plug");
        assert_eq!(config.model, device_model::DeviceModel::NousA1T);
        assert_eq!(config.protocol, Protocol::Mqtt);
        assert_eq!(config.host, "mqtt://192.168.1.50:1883");
        assert_eq!(config.topic, Some("tasmota_plug".to_string()));
    }

    #[tokio::test]
    async fn create_config_with_auth() {
        let mut app = TasmotaSupervisor {
            device_manager: DeviceManager::new(),
            app_config: AppConfig::default(),
            devices: Vec::new(),
            show_add_dialog: false,
            add_dialog_state: AddDeviceDialogState::new(),
            edit_dialog_state: None,
            error_message: None,
        };

        app.add_dialog_state.name = "Test Device".to_string();
        app.add_dialog_state.use_http = true;
        app.add_dialog_state.http_host = "192.168.1.100".to_string();
        app.add_dialog_state.use_auth = true;
        app.add_dialog_state.username = "admin".to_string();
        app.add_dialog_state.password = "secret".to_string();

        let config = app.create_device_config();

        assert_eq!(config.username, Some("admin".to_string()));
        assert_eq!(config.password, Some("secret".to_string()));
    }
}
