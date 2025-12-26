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

use std::collections::HashMap;
use std::sync::mpsc;

use eframe::egui;
use uuid::Uuid;

use device_config::{DeviceConfig, DeviceState, Protocol, StateUpdate};
use device_manager::DeviceManager;
use persistence::AppConfig;
use ui::{
    AddDeviceDialogState, ConsoleEntry, ConsoleLog, DeviceCardResponse, EditDeviceDialogState,
};

/// Main application state.
struct TasmotaSupervisor {
    /// Device manager wrapping the library's `DeviceManager`
    device_manager: DeviceManager,
    /// Persisted application configuration
    app_config: AppConfig,
    /// Local device state for UI display (keyed by device UUID)
    devices: HashMap<Uuid, DeviceState>,
    /// Channel receiver for state updates from async callbacks
    update_rx: mpsc::Receiver<StateUpdate>,
    /// Console logs for HTTP devices (keyed by device UUID)
    console_logs: HashMap<Uuid, ConsoleLog>,
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
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Create channel for state updates from async callbacks
        let (update_tx, update_rx) = mpsc::channel();

        // Create device manager with channel and egui context
        let device_manager = DeviceManager::new(update_tx, cc.egui_ctx.clone());
        let app_config = AppConfig::load();

        let rt = tokio::runtime::Handle::current();

        // Add saved devices to the manager
        for config in &app_config.devices {
            if let Err(e) = rt.block_on(device_manager.add_device(config.clone())) {
                tracing::warn!(device = %config.name, error = %e, "Failed to add saved device");
            }
        }

        // Get initial device list as HashMap
        let devices: HashMap<Uuid, DeviceState> = rt
            .block_on(device_manager.devices())
            .into_iter()
            .map(|d| (d.config.id, d))
            .collect();

        Self {
            device_manager,
            app_config,
            devices,
            update_rx,
            console_logs: HashMap::new(),
            show_add_dialog: false,
            add_dialog_state: AddDeviceDialogState::new(),
            edit_dialog_state: None,
            error_message: None,
        }
    }

    /// Processes pending state updates from the channel (non-blocking).
    fn process_state_updates(&mut self) {
        // Drain all pending updates from the channel
        while let Ok(update) = self.update_rx.try_recv() {
            match update {
                StateUpdate::StateChanged { device_id, change } => {
                    if let Some(device) = self.devices.get_mut(&device_id) {
                        device.state.apply(&change);
                        tracing::trace!(
                            device_id = %device_id,
                            ?change,
                            "Applied state change to local UI state"
                        );
                    }
                }
                StateUpdate::DeviceAdded(device_id) => {
                    tracing::debug!(device_id = %device_id, "Device added event received");
                    // Device is added via add_device, this is just a notification
                }
                StateUpdate::DeviceRemoved(device_id) => {
                    tracing::debug!(device_id = %device_id, "Device removed event received");
                    self.devices.remove(&device_id);
                }
                StateUpdate::ConnectionChanged {
                    device_id,
                    status,
                    error,
                } => {
                    if let Some(device) = self.devices.get_mut(&device_id) {
                        device.status = status;
                        device.error = error;
                    }
                }
            }
        }
    }

    /// Logs an entry to the console for an HTTP device.
    fn log_to_console(&mut self, device_id: Uuid, entry: ConsoleEntry) {
        self.console_logs.entry(device_id).or_default().push(entry);
    }

    /// Handles device card interactions.
    #[allow(clippy::too_many_lines)]
    // Handler for all device interactions - splitting would reduce readability
    fn handle_device_card_response(
        &mut self,
        device_id: Uuid,
        response: &DeviceCardResponse,
        is_http: bool,
    ) {
        let rt = tokio::runtime::Handle::current();

        if response.connect_clicked {
            let dm = &self.device_manager;
            if let Err(e) = rt.block_on(dm.connect(device_id)) {
                self.error_message = Some(e);
            }
        }

        if response.disconnect_clicked {
            let dm = &self.device_manager;
            if let Err(e) = rt.block_on(dm.disconnect(device_id)) {
                self.error_message = Some(e);
            }
        }

        if response.refresh_clicked {
            let dm = &self.device_manager;
            match rt.block_on(dm.query_status(device_id)) {
                Ok(state) => {
                    // Update local device state
                    if let Some(device) = self.devices.get_mut(&device_id) {
                        device.state = state;
                    }
                }
                Err(e) => {
                    self.error_message = Some(e);
                }
            }
        }

        if response.settings_clicked {
            // Find the device config and open edit dialog
            if let Some(device) = self.devices.get(&device_id) {
                self.edit_dialog_state = Some(EditDeviceDialogState::from_config(&device.config));
                self.error_message = None;
            }
        }

        if response.delete_clicked {
            // Remove from persistent config
            self.app_config.remove_device(device_id);
            // Remove console log
            self.console_logs.remove(&device_id);
            // Remove from local state
            self.devices.remove(&device_id);

            let dm = &self.device_manager;
            rt.block_on(dm.remove_device(device_id));
        }

        // HTTP-specific: Power ON
        if response.power_on_clicked && is_http {
            let dm = &self.device_manager;
            match rt.block_on(dm.power_on(device_id)) {
                Ok(()) => {
                    self.log_to_console(
                        device_id,
                        ConsoleEntry::success("power_on()", "Power1 = ON"),
                    );
                }
                Err(e) => {
                    self.log_to_console(device_id, ConsoleEntry::error("power_on()", &e));
                }
            }
        }

        // HTTP-specific: Power OFF
        if response.power_off_clicked && is_http {
            let dm = &self.device_manager;
            match rt.block_on(dm.power_off(device_id)) {
                Ok(()) => {
                    self.log_to_console(
                        device_id,
                        ConsoleEntry::success("power_off()", "Power1 = OFF"),
                    );
                }
                Err(e) => {
                    self.log_to_console(device_id, ConsoleEntry::error("power_off()", &e));
                }
            }
        }

        // Power toggle (both HTTP and MQTT, but log for HTTP)
        if response.power_toggle_clicked {
            let dm = &self.device_manager;
            match rt.block_on(dm.toggle_power(device_id)) {
                Ok(()) => {
                    if is_http {
                        self.log_to_console(
                            device_id,
                            ConsoleEntry::success("power_toggle()", "Power toggled"),
                        );
                    }
                }
                Err(e) => {
                    if is_http {
                        self.log_to_console(device_id, ConsoleEntry::error("power_toggle()", &e));
                    } else {
                        self.error_message = Some(e);
                    }
                }
            }
        }

        // Dimmer
        if let Some(dimmer) = response.dimmer_changed {
            let dm = &self.device_manager;
            match rt.block_on(dm.set_dimmer(device_id, dimmer)) {
                Ok(()) => {
                    if is_http {
                        self.log_to_console(
                            device_id,
                            ConsoleEntry::success(
                                &format!("set_dimmer({dimmer})"),
                                &format!("Dimmer = {dimmer}"),
                            ),
                        );
                    }
                }
                Err(e) => {
                    if is_http {
                        self.log_to_console(
                            device_id,
                            ConsoleEntry::error(&format!("set_dimmer({dimmer})"), &e),
                        );
                    } else {
                        self.error_message = Some(e);
                    }
                }
            }
        }

        // HSB Color
        if let Some((hue, sat, bri)) = response.hue_changed {
            let dm = &self.device_manager;
            match rt.block_on(dm.set_hsb_color(device_id, hue, sat, bri)) {
                Ok(()) => {
                    if is_http {
                        self.log_to_console(
                            device_id,
                            ConsoleEntry::success(
                                &format!("set_hsb_color({hue}, {sat}, {bri})"),
                                &format!("HSBColor = {hue},{sat},{bri}"),
                            ),
                        );
                    }
                }
                Err(e) => {
                    if is_http {
                        self.log_to_console(
                            device_id,
                            ConsoleEntry::error(&format!("set_hsb_color({hue}, {sat}, {bri})"), &e),
                        );
                    } else {
                        self.error_message = Some(e);
                    }
                }
            }
        }

        // Color temperature
        if let Some(ct) = response.color_temp_changed {
            let dm = &self.device_manager;
            match rt.block_on(dm.set_color_temperature(device_id, ct)) {
                Ok(()) => {
                    if is_http {
                        self.log_to_console(
                            device_id,
                            ConsoleEntry::success(
                                &format!("set_color_temperature({ct})"),
                                &format!("CT = {ct}"),
                            ),
                        );
                    }
                }
                Err(e) => {
                    if is_http {
                        self.log_to_console(
                            device_id,
                            ConsoleEntry::error(&format!("set_color_temperature({ct})"), &e),
                        );
                    } else {
                        self.error_message = Some(e);
                    }
                }
            }
        }

        // Energy reset
        if response.energy_reset_clicked {
            let dm = &self.device_manager;
            match rt.block_on(dm.reset_energy_total(device_id)) {
                Ok(state) => {
                    // Update local device state with the new energy values
                    if let Some(device) = self.devices.get_mut(&device_id) {
                        device.state = state;
                    }
                    if is_http {
                        self.log_to_console(
                            device_id,
                            ConsoleEntry::success("reset_energy_total()", "Energy counter reset"),
                        );
                    }
                }
                Err(e) => {
                    if is_http {
                        self.log_to_console(
                            device_id,
                            ConsoleEntry::error("reset_energy_total()", &e),
                        );
                    } else {
                        self.error_message = Some(e);
                    }
                }
            }
        }

        // HTTP-specific: Status query
        if response.status_query_clicked && is_http {
            let dm = &self.device_manager;
            match rt.block_on(dm.query_status(device_id)) {
                Ok(state) => {
                    // Update local device state
                    if let Some(device) = self.devices.get_mut(&device_id) {
                        device.state = state.clone();
                    }

                    // Display the queried state in console
                    let power_str = state
                        .power(1)
                        .map_or("?".to_string(), |ps| ps.as_str().to_string());
                    let dimmer_str = state
                        .dimmer()
                        .map_or(String::new(), |d| format!(", Dimmer={}", d.value()));
                    let ct_str = state
                        .color_temperature()
                        .map_or(String::new(), |ct| format!(", CT={}", ct.value()));
                    self.log_to_console(
                        device_id,
                        ConsoleEntry::success(
                            "status()",
                            &format!("Power={power_str}{dimmer_str}{ct_str}"),
                        ),
                    );
                }
                Err(e) => {
                    self.log_to_console(device_id, ConsoleEntry::error("status()", &e));
                }
            }
        }

        // HTTP-specific: Console clear
        if response.console_clear_clicked {
            if let Some(log) = self.console_logs.get_mut(&device_id) {
                log.clear();
            }
        }
    }

    /// Handles add device dialog.
    fn show_add_device_dialog(&mut self, ctx: &egui::Context) {
        let rt = tokio::runtime::Handle::current();

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
                            let device_id = config.id;

                            // Add to device manager
                            match rt.block_on(self.device_manager.add_device(config.clone())) {
                                Ok(()) => {
                                    // Add to local device state
                                    let mut managed = DeviceState::new(config.clone());
                                    managed.status = device_config::ConnectionStatus::Connected;
                                    self.devices.insert(device_id, managed);
                                    // Save to persistent config only on success
                                    self.app_config.add_device(config);
                                    self.show_add_dialog = false;
                                    self.add_dialog_state = AddDeviceDialogState::new();
                                    self.error_message = None;
                                }
                                Err(e) => {
                                    self.error_message = Some(format!("Failed to add device: {e}"));
                                }
                            }
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
        let rt = tokio::runtime::Handle::current();

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

            // Update in device manager (remove and re-add)
            rt.block_on(self.device_manager.remove_device(device_id));
            self.devices.remove(&device_id);
            match rt.block_on(self.device_manager.add_device(updated_config.clone())) {
                Ok(()) => {
                    // Add to local device state with new config
                    let mut managed = DeviceState::new(updated_config.clone());
                    managed.status = device_config::ConnectionStatus::Connected;
                    self.devices.insert(device_id, managed);
                    // Update in persistent config only on success
                    self.app_config.update_device(updated_config);
                    self.edit_dialog_state = None;
                    self.error_message = None;
                }
                Err(e) => {
                    self.error_message = Some(format!("Failed to update device: {e}"));
                }
            }
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
    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Shutdown device manager to properly disconnect MQTT
        let rt = tokio::runtime::Handle::current();
        rt.block_on(self.device_manager.shutdown());
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Process any pending state updates from async callbacks (non-blocking)
        self.process_state_updates();

        // Top panel with actions
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Tasmota Supervisor");
                ui.separator();

                if ui.button("+ Add Device").clicked() {
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
                            .values()
                            .map(|device| {
                                let console_log = self.console_logs.get(&device.config.id);
                                let response = ui::device_card(ui, device, console_log);
                                ui.add_space(8.0);
                                (device.config.id, device.config.protocol, response)
                            })
                            .collect()
                    })
                    .inner;

                for (device_id, protocol, response) in &responses {
                    let is_http = *protocol == Protocol::Http;
                    self.handle_device_card_response(*device_id, response, is_http);
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
    use eframe::egui;

    /// Creates a test supervisor with mock channel and context.
    fn create_test_supervisor() -> TasmotaSupervisor {
        let (tx, rx) = mpsc::channel();
        let ctx = egui::Context::default();
        let device_manager = DeviceManager::new(tx, ctx);

        TasmotaSupervisor {
            device_manager,
            app_config: AppConfig::default(),
            devices: HashMap::new(),
            update_rx: rx,
            console_logs: HashMap::new(),
            show_add_dialog: false,
            add_dialog_state: AddDeviceDialogState::new(),
            edit_dialog_state: None,
            error_message: None,
        }
    }

    #[tokio::test]
    async fn create_http_config() {
        let mut app = create_test_supervisor();

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
        let mut app = create_test_supervisor();

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
        let mut app = create_test_supervisor();

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
