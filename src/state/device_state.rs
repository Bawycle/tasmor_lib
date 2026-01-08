// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device state tracking.
//!
//! This module provides [`DeviceState`], a comprehensive representation of a
//! Tasmota device's current state. It tracks:
//!
//! - **Power state** for up to 8 relays (POWER1-POWER8)
//! - **Light settings**: dimmer level, HSB color, color temperature
//! - **Energy readings**: voltage, current, power consumption, energy totals
//! - **System info**: uptime, Wi-Fi signal strength, free memory (read-only)
//!
//! # Design Philosophy
//!
//! All fields are `Option` types because device state may not be known until
//! the device reports it via telemetry or command response. This allows
//! partial state updates without losing existing information.
//!
//! # Usage with Events
//!
//! `DeviceState` works together with [`StateChange`](super::StateChange) to
//! provide an event-driven state management system:
//!
//! ```
//! use tasmor_lib::state::{DeviceState, StateChange};
//! use tasmor_lib::types::{PowerState, Dimmer};
//!
//! let mut state = DeviceState::new();
//!
//! // Apply changes from telemetry
//! let changes = vec![
//!     StateChange::power(1, PowerState::On),
//!     StateChange::dimmer(Dimmer::new(80).unwrap()),
//! ];
//!
//! for change in &changes {
//!     state.apply(change);
//! }
//!
//! // Query current state
//! assert_eq!(state.power(1), Some(PowerState::On));
//! assert_eq!(state.dimmer().map(|d| d.value()), Some(80));
//! ```

use crate::types::{
    ColorTemperature, Dimmer, FadeSpeed, HsbColor, PowerState, Scheme, TasmotaDateTime,
    WakeupDuration,
};

use super::StateChange;

/// System information from device telemetry.
///
/// Contains read-only diagnostic data like uptime and network status.
/// These values change frequently and do **not** trigger callbacks when updated.
///
/// # Data Sources
///
/// - **MQTT telemetry**: `uptime_sec` and `wifi_rssi` from `tele/<topic>/STATE`
/// - **HTTP status**: All fields from `Status 0` command
///
/// # Examples
///
/// ```
/// use tasmor_lib::state::SystemInfo;
///
/// let info = SystemInfo::new()
///     .with_uptime_sec(172800)
///     .with_wifi_rssi(-60)
///     .with_heap(25000);
///
/// assert_eq!(info.uptime_seconds(), Some(172800));
/// ```
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct SystemInfo {
    /// Device uptime in seconds.
    uptime_sec: Option<u64>,
    /// Wi-Fi signal strength in dBm (typically -100 to 0, where 0 is best).
    wifi_rssi: Option<i8>,
    /// Free heap memory in kilobytes.
    heap: Option<u32>,
}

impl SystemInfo {
    /// Creates a new empty system info.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the uptime in seconds.
    #[must_use]
    pub fn with_uptime_sec(mut self, seconds: u64) -> Self {
        self.uptime_sec = Some(seconds);
        self
    }

    /// Sets the Wi-Fi RSSI in dBm.
    #[must_use]
    pub fn with_wifi_rssi(mut self, rssi: i8) -> Self {
        self.wifi_rssi = Some(rssi);
        self
    }

    /// Sets the free heap memory in kilobytes.
    #[must_use]
    pub fn with_heap(mut self, heap_kb: u32) -> Self {
        self.heap = Some(heap_kb);
        self
    }

    /// Returns the device uptime in seconds.
    #[must_use]
    pub fn uptime_seconds(&self) -> Option<u64> {
        self.uptime_sec
    }

    /// Returns the Wi-Fi signal strength in dBm.
    ///
    /// Typical values range from -100 (weak) to 0 (strongest).
    /// A signal of -50 dBm or better is considered excellent.
    #[must_use]
    pub fn wifi_rssi(&self) -> Option<i8> {
        self.wifi_rssi
    }

    /// Returns the free heap memory in kilobytes.
    #[must_use]
    pub fn heap(&self) -> Option<u32> {
        self.heap
    }

    /// Updates fields from another `SystemInfo`, preserving existing values
    /// when the new value is `None`.
    pub fn merge(&mut self, other: &SystemInfo) {
        if other.uptime_sec.is_some() {
            self.uptime_sec = other.uptime_sec;
        }
        if other.wifi_rssi.is_some() {
            self.wifi_rssi = other.wifi_rssi;
        }
        if other.heap.is_some() {
            self.heap = other.heap;
        }
    }

    /// Returns `true` if all fields are `None`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.uptime_sec.is_none() && self.wifi_rssi.is_none() && self.heap.is_none()
    }
}

/// Tracked state of a Tasmota device.
///
/// This struct maintains the current state of a device, including power states,
/// dimmer level, color settings, and energy readings. All fields are optional
/// because state may not be known until the device reports it.
///
/// # Maximum Relays
///
/// Tasmota supports up to 8 relays (POWER1-POWER8). The state tracks each
/// relay independently.
///
/// # Examples
///
/// ```
/// use tasmor_lib::state::DeviceState;
/// use tasmor_lib::types::PowerState;
///
/// let mut state = DeviceState::new();
/// state.set_power(1, PowerState::On);
/// assert_eq!(state.power(1), Some(PowerState::On));
/// ```
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeviceState {
    /// Power state for each relay (indexed 0-7 for POWER1-POWER8).
    power: [Option<PowerState>; 8],
    /// Dimmer level (0-100).
    dimmer: Option<Dimmer>,
    /// HSB color (hue, saturation, brightness).
    hsb_color: Option<HsbColor>,
    /// Color temperature in mireds (153-500).
    color_temperature: Option<ColorTemperature>,
    /// Light scheme/effect (0-4).
    scheme: Option<Scheme>,
    /// Wakeup duration in seconds (1-3000).
    wakeup_duration: Option<WakeupDuration>,
    /// Whether fade transitions are enabled.
    fade_enabled: Option<bool>,
    /// Fade transition speed (1-40).
    fade_speed: Option<FadeSpeed>,
    /// Current power consumption in Watts.
    power_consumption: Option<f32>,
    /// Current voltage in Volts.
    voltage: Option<f32>,
    /// Current in Amperes.
    current: Option<f32>,
    /// Apparent power in VA.
    apparent_power: Option<f32>,
    /// Reactive power in `VAr`.
    reactive_power: Option<f32>,
    /// Power factor (0-1).
    power_factor: Option<f32>,
    /// Energy consumed today in kWh.
    energy_today: Option<f32>,
    /// Energy consumed yesterday in kWh.
    energy_yesterday: Option<f32>,
    /// Energy total in kWh.
    energy_total: Option<f32>,
    /// Timestamp when total energy counting started.
    total_start_time: Option<TasmotaDateTime>,
    /// System diagnostic information (uptime, Wi-Fi, memory).
    ///
    /// This is read-only data that does **not** trigger callbacks.
    system_info: Option<SystemInfo>,
}

impl DeviceState {
    /// Creates a new empty device state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    // ========== Power State ==========

    /// Gets the power state for a specific relay.
    ///
    /// # Arguments
    ///
    /// * `index` - The relay index (1-8)
    ///
    /// # Returns
    ///
    /// Returns `None` if the index is out of range (0 or >8) or if the
    /// power state is unknown.
    #[must_use]
    pub fn power(&self, index: u8) -> Option<PowerState> {
        if index == 0 || index > 8 {
            return None;
        }
        self.power[usize::from(index - 1)]
    }

    /// Sets the power state for a specific relay.
    ///
    /// # Arguments
    ///
    /// * `index` - The relay index (1-8)
    /// * `state` - The power state to set
    ///
    /// Does nothing if index is 0 or greater than 8.
    pub fn set_power(&mut self, index: u8, state: PowerState) {
        if index > 0 && index <= 8 {
            self.power[usize::from(index - 1)] = Some(state);
        }
    }

    /// Clears the power state for a specific relay.
    pub fn clear_power(&mut self, index: u8) {
        if index > 0 && index <= 8 {
            self.power[usize::from(index - 1)] = None;
        }
    }

    /// Returns all known power states as (index, state) pairs.
    #[must_use]
    pub fn all_power_states(&self) -> Vec<(u8, PowerState)> {
        self.power
            .iter()
            .enumerate()
            .filter_map(|(i, state)| {
                state.map(|s| {
                    // Safe: i is 0-7, so i+1 fits in u8
                    #[allow(clippy::cast_possible_truncation)]
                    let index = (i + 1) as u8;
                    (index, s)
                })
            })
            .collect()
    }

    /// Returns `true` if any relay is on.
    #[must_use]
    pub fn is_any_on(&self) -> bool {
        self.power.iter().any(|s| matches!(s, Some(PowerState::On)))
    }

    // ========== Dimmer ==========

    /// Gets the dimmer level.
    #[must_use]
    pub fn dimmer(&self) -> Option<Dimmer> {
        self.dimmer
    }

    /// Sets the dimmer level.
    pub fn set_dimmer(&mut self, value: Dimmer) {
        self.dimmer = Some(value);
    }

    /// Clears the dimmer level.
    pub fn clear_dimmer(&mut self) {
        self.dimmer = None;
    }

    // ========== HSB Color ==========

    /// Gets the HSB color.
    #[must_use]
    pub fn hsb_color(&self) -> Option<HsbColor> {
        self.hsb_color
    }

    /// Sets the HSB color.
    pub fn set_hsb_color(&mut self, color: HsbColor) {
        self.hsb_color = Some(color);
    }

    /// Clears the HSB color.
    pub fn clear_hsb_color(&mut self) {
        self.hsb_color = None;
    }

    // ========== Color Temperature ==========

    /// Gets the color temperature.
    #[must_use]
    pub fn color_temperature(&self) -> Option<ColorTemperature> {
        self.color_temperature
    }

    /// Sets the color temperature.
    pub fn set_color_temperature(&mut self, ct: ColorTemperature) {
        self.color_temperature = Some(ct);
    }

    /// Clears the color temperature.
    pub fn clear_color_temperature(&mut self) {
        self.color_temperature = None;
    }

    // ========== Scheme ==========

    /// Gets the light scheme/effect.
    #[must_use]
    pub fn scheme(&self) -> Option<Scheme> {
        self.scheme
    }

    /// Sets the light scheme/effect.
    pub fn set_scheme(&mut self, scheme: Scheme) {
        self.scheme = Some(scheme);
    }

    /// Clears the scheme.
    pub fn clear_scheme(&mut self) {
        self.scheme = None;
    }

    // ========== Wakeup Duration ==========

    /// Gets the wakeup duration.
    #[must_use]
    pub fn wakeup_duration(&self) -> Option<WakeupDuration> {
        self.wakeup_duration
    }

    /// Sets the wakeup duration.
    pub fn set_wakeup_duration(&mut self, duration: WakeupDuration) {
        self.wakeup_duration = Some(duration);
    }

    /// Clears the wakeup duration.
    pub fn clear_wakeup_duration(&mut self) {
        self.wakeup_duration = None;
    }

    // ========== Fade Settings ==========

    /// Gets whether fade transitions are enabled.
    #[must_use]
    pub fn fade_enabled(&self) -> Option<bool> {
        self.fade_enabled
    }

    /// Sets whether fade transitions are enabled.
    pub fn set_fade_enabled(&mut self, enabled: bool) {
        self.fade_enabled = Some(enabled);
    }

    /// Clears the fade enabled state.
    pub fn clear_fade_enabled(&mut self) {
        self.fade_enabled = None;
    }

    /// Gets the fade transition speed.
    #[must_use]
    pub fn fade_speed(&self) -> Option<FadeSpeed> {
        self.fade_speed
    }

    /// Sets the fade transition speed.
    pub fn set_fade_speed(&mut self, speed: FadeSpeed) {
        self.fade_speed = Some(speed);
    }

    /// Clears the fade speed.
    pub fn clear_fade_speed(&mut self) {
        self.fade_speed = None;
    }

    // ========== Energy Monitoring ==========

    /// Gets the current power consumption in Watts.
    #[must_use]
    pub fn power_consumption(&self) -> Option<f32> {
        self.power_consumption
    }

    /// Sets the power consumption.
    pub fn set_power_consumption(&mut self, watts: f32) {
        self.power_consumption = Some(watts);
    }

    /// Gets the current voltage in Volts.
    #[must_use]
    pub fn voltage(&self) -> Option<f32> {
        self.voltage
    }

    /// Sets the voltage.
    pub fn set_voltage(&mut self, volts: f32) {
        self.voltage = Some(volts);
    }

    /// Gets the current in Amperes.
    #[must_use]
    pub fn current(&self) -> Option<f32> {
        self.current
    }

    /// Sets the current.
    pub fn set_current(&mut self, amps: f32) {
        self.current = Some(amps);
    }

    /// Gets the total energy consumption in kWh.
    #[must_use]
    pub fn energy_total(&self) -> Option<f32> {
        self.energy_total
    }

    /// Sets the total energy.
    pub fn set_energy_total(&mut self, kwh: f32) {
        self.energy_total = Some(kwh);
    }

    /// Gets the apparent power in VA.
    #[must_use]
    pub fn apparent_power(&self) -> Option<f32> {
        self.apparent_power
    }

    /// Sets the apparent power.
    pub fn set_apparent_power(&mut self, va: f32) {
        self.apparent_power = Some(va);
    }

    /// Gets the reactive power in `VAr`.
    #[must_use]
    pub fn reactive_power(&self) -> Option<f32> {
        self.reactive_power
    }

    /// Sets the reactive power.
    pub fn set_reactive_power(&mut self, var: f32) {
        self.reactive_power = Some(var);
    }

    /// Gets the power factor (0-1).
    #[must_use]
    pub fn power_factor(&self) -> Option<f32> {
        self.power_factor
    }

    /// Sets the power factor.
    pub fn set_power_factor(&mut self, factor: f32) {
        self.power_factor = Some(factor);
    }

    /// Gets the energy consumed today in kWh.
    #[must_use]
    pub fn energy_today(&self) -> Option<f32> {
        self.energy_today
    }

    /// Sets the energy consumed today.
    pub fn set_energy_today(&mut self, kwh: f32) {
        self.energy_today = Some(kwh);
    }

    /// Gets the energy consumed yesterday in kWh.
    #[must_use]
    pub fn energy_yesterday(&self) -> Option<f32> {
        self.energy_yesterday
    }

    /// Sets the energy consumed yesterday.
    pub fn set_energy_yesterday(&mut self, kwh: f32) {
        self.energy_yesterday = Some(kwh);
    }

    /// Gets the timestamp when total energy counting started.
    ///
    /// Returns a [`TasmotaDateTime`] which provides both:
    /// - `naive()` - the datetime without timezone (always available)
    /// - `to_datetime()` - the timezone-aware datetime (if timezone was known)
    #[must_use]
    pub fn total_start_time(&self) -> Option<&TasmotaDateTime> {
        self.total_start_time.as_ref()
    }

    /// Sets the timestamp when total energy counting started.
    pub fn set_total_start_time(&mut self, time: TasmotaDateTime) {
        self.total_start_time = Some(time);
    }

    // ========== System Info ==========

    /// Gets the system diagnostic information.
    ///
    /// System info includes uptime, Wi-Fi signal strength, and free memory.
    /// This data does **not** trigger callbacks when updated.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::state::{DeviceState, SystemInfo};
    ///
    /// let mut state = DeviceState::new();
    /// state.set_system_info(SystemInfo::new().with_uptime_sec(172800));
    ///
    /// if let Some(info) = state.system_info() {
    ///     println!("Uptime: {} seconds", info.uptime_seconds().unwrap_or(0));
    /// }
    /// ```
    #[must_use]
    pub fn system_info(&self) -> Option<&SystemInfo> {
        self.system_info.as_ref()
    }

    /// Sets the system diagnostic information.
    pub fn set_system_info(&mut self, info: SystemInfo) {
        self.system_info = Some(info);
    }

    /// Updates system information, merging with existing data.
    ///
    /// This preserves existing values when the new `SystemInfo` has `None` fields.
    pub fn update_system_info(&mut self, info: &SystemInfo) {
        if let Some(existing) = &mut self.system_info {
            existing.merge(info);
        } else {
            self.system_info = Some(info.clone());
        }
    }

    /// Returns the device uptime in seconds.
    ///
    /// This is a convenience method equivalent to
    /// `state.system_info().and_then(|i| i.uptime_seconds())`.
    #[must_use]
    pub fn uptime_seconds(&self) -> Option<u64> {
        self.system_info
            .as_ref()
            .and_then(SystemInfo::uptime_seconds)
    }

    // ========== State Changes ==========

    /// Applies a state change and returns whether the state actually changed.
    ///
    /// # Returns
    ///
    /// Returns `true` if the state was modified, `false` if it was already
    /// at the target value.
    #[allow(clippy::too_many_lines)]
    // Match arms for each StateChange variant are straightforward and splitting
    // would reduce readability without improving maintainability
    pub fn apply(&mut self, change: &StateChange) -> bool {
        match change {
            StateChange::Power { index, state } => {
                let current = self.power(*index);
                if current == Some(*state) {
                    false
                } else {
                    self.set_power(*index, *state);
                    true
                }
            }
            StateChange::Dimmer(value) => {
                if self.dimmer == Some(*value) {
                    false
                } else {
                    self.dimmer = Some(*value);
                    true
                }
            }
            StateChange::HsbColor(color) => {
                if self.hsb_color == Some(*color) {
                    false
                } else {
                    self.hsb_color = Some(*color);
                    true
                }
            }
            StateChange::ColorTemperature(ct) => {
                if self.color_temperature == Some(*ct) {
                    false
                } else {
                    self.color_temperature = Some(*ct);
                    true
                }
            }
            StateChange::Scheme(scheme) => {
                if self.scheme == Some(*scheme) {
                    false
                } else {
                    self.scheme = Some(*scheme);
                    true
                }
            }
            StateChange::WakeupDuration(duration) => {
                if self.wakeup_duration == Some(*duration) {
                    false
                } else {
                    self.wakeup_duration = Some(*duration);
                    true
                }
            }
            StateChange::FadeEnabled(enabled) => {
                if self.fade_enabled == Some(*enabled) {
                    false
                } else {
                    self.fade_enabled = Some(*enabled);
                    true
                }
            }
            StateChange::FadeSpeed(speed) => {
                if self.fade_speed == Some(*speed) {
                    false
                } else {
                    self.fade_speed = Some(*speed);
                    true
                }
            }
            StateChange::Energy {
                power,
                voltage,
                current,
                apparent_power,
                reactive_power,
                power_factor,
                energy_today,
                energy_yesterday,
                energy_total,
                total_start_time,
            } => {
                let mut changed = false;

                // Helper macro to update optional numeric fields
                macro_rules! update_if_some {
                    ($field:ident, $value:expr) => {
                        if let Some(v) = $value {
                            if self.$field != Some(*v) {
                                self.$field = Some(*v);
                                changed = true;
                            }
                        }
                    };
                }

                update_if_some!(power_consumption, power);
                update_if_some!(voltage, voltage);
                update_if_some!(current, current);
                update_if_some!(apparent_power, apparent_power);
                update_if_some!(reactive_power, reactive_power);
                update_if_some!(power_factor, power_factor);
                update_if_some!(energy_today, energy_today);
                update_if_some!(energy_yesterday, energy_yesterday);
                update_if_some!(energy_total, energy_total);

                // Handle datetime field separately (not a Copy type)
                if let Some(time) = total_start_time
                    && self.total_start_time.as_ref() != Some(time)
                {
                    self.total_start_time = Some(time.clone());
                    changed = true;
                }

                changed
            }
            StateChange::Batch(changes) => {
                let mut any_changed = false;
                for c in changes {
                    if self.apply(c) {
                        any_changed = true;
                    }
                }
                any_changed
            }
        }
    }

    /// Clears all state, resetting to unknown.
    pub fn clear(&mut self) {
        *self = Self::new();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_state_is_empty() {
        let state = DeviceState::new();
        assert!(state.power(1).is_none());
        assert!(state.dimmer().is_none());
        assert!(state.hsb_color().is_none());
        assert!(state.color_temperature().is_none());
        assert!(state.power_consumption().is_none());
    }

    #[test]
    fn power_state_management() {
        let mut state = DeviceState::new();

        state.set_power(1, PowerState::On);
        assert_eq!(state.power(1), Some(PowerState::On));
        assert!(state.power(2).is_none());

        state.set_power(2, PowerState::Off);
        assert_eq!(state.power(2), Some(PowerState::Off));

        state.clear_power(1);
        assert!(state.power(1).is_none());
    }

    #[test]
    fn power_index_bounds() {
        let mut state = DeviceState::new();

        // Index 0 is invalid
        state.set_power(0, PowerState::On);
        assert!(state.power(0).is_none());

        // Index 9 is out of range
        state.set_power(9, PowerState::On);
        assert!(state.power(9).is_none());

        // Index 8 is valid
        state.set_power(8, PowerState::On);
        assert_eq!(state.power(8), Some(PowerState::On));
    }

    #[test]
    fn all_power_states() {
        let mut state = DeviceState::new();
        state.set_power(1, PowerState::On);
        state.set_power(3, PowerState::Off);
        state.set_power(5, PowerState::On);

        let states = state.all_power_states();
        assert_eq!(states.len(), 3);
        assert!(states.contains(&(1, PowerState::On)));
        assert!(states.contains(&(3, PowerState::Off)));
        assert!(states.contains(&(5, PowerState::On)));
    }

    #[test]
    fn is_any_on() {
        let mut state = DeviceState::new();
        assert!(!state.is_any_on());

        state.set_power(1, PowerState::Off);
        assert!(!state.is_any_on());

        state.set_power(2, PowerState::On);
        assert!(state.is_any_on());
    }

    #[test]
    fn apply_power_change() {
        let mut state = DeviceState::new();

        let change = StateChange::Power {
            index: 1,
            state: PowerState::On,
        };
        assert!(state.apply(&change));
        assert_eq!(state.power(1), Some(PowerState::On));

        // Applying same state returns false
        assert!(!state.apply(&change));
    }

    #[test]
    fn apply_dimmer_change() {
        let mut state = DeviceState::new();
        let dimmer = Dimmer::new(75).unwrap();

        let change = StateChange::Dimmer(dimmer);
        assert!(state.apply(&change));
        assert_eq!(state.dimmer(), Some(dimmer));
    }

    #[test]
    fn apply_batch_changes() {
        let mut state = DeviceState::new();

        let changes = StateChange::Batch(vec![
            StateChange::Power {
                index: 1,
                state: PowerState::On,
            },
            StateChange::Dimmer(Dimmer::new(50).unwrap()),
        ]);

        assert!(state.apply(&changes));
        assert_eq!(state.power(1), Some(PowerState::On));
        assert_eq!(state.dimmer(), Some(Dimmer::new(50).unwrap()));
    }

    #[test]
    fn clear_resets_state() {
        let mut state = DeviceState::new();
        state.set_power(1, PowerState::On);
        state.set_dimmer(Dimmer::new(75).unwrap());

        state.clear();

        assert!(state.power(1).is_none());
        assert!(state.dimmer().is_none());
    }

    #[test]
    fn apply_batch_with_hsb_color() {
        use crate::types::HsbColor;

        let mut state = DeviceState::new();
        let hsb = HsbColor::new(360, 100, 100).unwrap();

        let changes = StateChange::Batch(vec![
            StateChange::Power {
                index: 1,
                state: PowerState::Off,
            },
            StateChange::Dimmer(Dimmer::new(100).unwrap()),
            StateChange::HsbColor(hsb),
        ]);

        assert!(state.apply(&changes));
        assert_eq!(state.power(1), Some(PowerState::Off));
        assert_eq!(state.dimmer(), Some(Dimmer::new(100).unwrap()));

        // Verify HsbColor was applied
        let applied_hsb = state.hsb_color().expect("HsbColor should be set");
        assert_eq!(applied_hsb.hue(), 360);
        assert_eq!(applied_hsb.saturation(), 100);
        assert_eq!(applied_hsb.brightness(), 100);
    }

    #[test]
    fn apply_state_from_tasmota_telemetry() {
        use crate::telemetry::TelemetryState;

        // Real Tasmota RESULT JSON from logs
        let json = r#"{
            "Time":"2025-12-24T14:24:03",
            "Uptime":"1T23:46:58",
            "UptimeSec":172018,
            "Heap":25,
            "SleepMode":"Dynamic",
            "Sleep":50,
            "LoadAvg":19,
            "MqttCount":1,
            "POWER":"OFF",
            "Dimmer":100,
            "Color":"FF00000000",
            "HSBColor":"360,100,100",
            "White":0,
            "CT":153,
            "Channel":[100,0,0,0,0],
            "Scheme":0,
            "Fade":"ON",
            "Speed":2,
            "LedTable":"ON",
            "Wifi":{"AP":1}
        }"#;

        // Parse telemetry
        let telemetry: TelemetryState = serde_json::from_str(json).unwrap();
        let changes = telemetry.to_state_changes();

        // Apply to DeviceState
        let mut state = DeviceState::new();
        for change in changes {
            state.apply(&change);
        }

        // Verify all fields are correctly set
        assert_eq!(state.power(1), Some(PowerState::Off));
        assert_eq!(state.dimmer(), Some(Dimmer::new(100).unwrap()));

        // This is the key assertion - HSBColor must be set
        let hsb = state
            .hsb_color()
            .expect("HSBColor should be set from telemetry");
        assert_eq!(hsb.hue(), 360);
        assert_eq!(hsb.saturation(), 100);
        assert_eq!(hsb.brightness(), 100);

        // Color temperature should also be set
        assert!(state.color_temperature().is_some());
        assert_eq!(state.color_temperature().unwrap().value(), 153);

        // Fade should be enabled
        assert_eq!(state.fade_enabled(), Some(true));

        // Fade speed should be set
        assert_eq!(state.fade_speed().map(|s| s.value()), Some(2));
    }

    #[test]
    fn fade_getters_setters() {
        let mut state = DeviceState::new();

        // Initially None
        assert!(state.fade_enabled().is_none());
        assert!(state.fade_speed().is_none());

        // Set fade enabled
        state.set_fade_enabled(true);
        assert_eq!(state.fade_enabled(), Some(true));

        state.set_fade_enabled(false);
        assert_eq!(state.fade_enabled(), Some(false));

        // Set fade speed
        let speed = FadeSpeed::new(15).unwrap();
        state.set_fade_speed(speed);
        assert_eq!(state.fade_speed(), Some(speed));

        // Clear
        state.clear_fade_enabled();
        state.clear_fade_speed();
        assert!(state.fade_enabled().is_none());
        assert!(state.fade_speed().is_none());
    }

    #[test]
    fn apply_fade_changes() {
        let mut state = DeviceState::new();

        // Apply fade enabled
        let change = StateChange::FadeEnabled(true);
        assert!(state.apply(&change));
        assert_eq!(state.fade_enabled(), Some(true));

        // Applying same state returns false
        assert!(!state.apply(&change));

        // Apply fade speed
        let speed = FadeSpeed::new(20).unwrap();
        let change = StateChange::FadeSpeed(speed);
        assert!(state.apply(&change));
        assert_eq!(state.fade_speed(), Some(speed));
    }

    // ========== SystemInfo Tests ==========

    #[test]
    fn system_info_new_is_empty() {
        let info = SystemInfo::new();
        assert!(info.is_empty());
        assert!(info.uptime_seconds().is_none());
        assert!(info.wifi_rssi().is_none());
        assert!(info.heap().is_none());
    }

    #[test]
    fn system_info_builder_pattern() {
        let info = SystemInfo::new()
            .with_uptime_sec(172800)
            .with_wifi_rssi(-55)
            .with_heap(25000);

        assert!(!info.is_empty());
        assert_eq!(info.uptime_seconds(), Some(172800));
        assert_eq!(info.uptime_seconds(), Some(172800));
        assert_eq!(info.wifi_rssi(), Some(-55));
        assert_eq!(info.heap(), Some(25000));
    }

    #[test]
    fn system_info_merge_preserves_existing() {
        let mut info = SystemInfo::new().with_uptime_sec(100).with_wifi_rssi(-50);

        // Merge with partial update (only heap)
        let update = SystemInfo::new().with_heap(30000);
        info.merge(&update);

        // Original values preserved, new value added
        assert_eq!(info.uptime_seconds(), Some(100));
        assert_eq!(info.wifi_rssi(), Some(-50));
        assert_eq!(info.heap(), Some(30000));
    }

    #[test]
    fn system_info_merge_updates_values() {
        let mut info = SystemInfo::new().with_uptime_sec(100).with_wifi_rssi(-50);

        // Merge with overlapping update
        let update = SystemInfo::new().with_uptime_sec(200).with_heap(30000);
        info.merge(&update);

        // Updated values
        assert_eq!(info.uptime_seconds(), Some(200));
        assert_eq!(info.wifi_rssi(), Some(-50)); // Preserved
        assert_eq!(info.heap(), Some(30000));
    }

    #[test]
    fn device_state_system_info_getters_setters() {
        let mut state = DeviceState::new();

        // Initially None
        assert!(state.system_info().is_none());
        assert!(state.uptime_seconds().is_none());

        // Set system info
        let info = SystemInfo::new().with_uptime_sec(172800);
        state.set_system_info(info);

        assert!(state.system_info().is_some());
        assert_eq!(state.uptime_seconds(), Some(172800));
    }

    #[test]
    fn device_state_update_system_info() {
        let mut state = DeviceState::new();

        // Update on empty state
        let info1 = SystemInfo::new().with_uptime_sec(100);
        state.update_system_info(&info1);
        assert_eq!(state.uptime_seconds(), Some(100));

        // Update with merge
        let info2 = SystemInfo::new().with_wifi_rssi(-55);
        state.update_system_info(&info2);

        let sys_info = state.system_info().unwrap();
        assert_eq!(sys_info.uptime_seconds(), Some(100)); // Preserved
        assert_eq!(sys_info.wifi_rssi(), Some(-55)); // Added
    }

    #[test]
    fn device_state_clear_clears_system_info() {
        let mut state = DeviceState::new();
        state.set_system_info(SystemInfo::new().with_uptime_sec(172800));

        state.clear();

        assert!(state.system_info().is_none());
    }

    #[test]
    fn system_info_serialization() {
        let info = SystemInfo::new()
            .with_uptime_sec(172800)
            .with_wifi_rssi(-55)
            .with_heap(25000);

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: SystemInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info, deserialized);
    }

    #[test]
    fn device_state_with_system_info_serialization() {
        let mut state = DeviceState::new();
        state.set_power(1, PowerState::On);
        state.set_system_info(
            SystemInfo::new()
                .with_uptime_sec(172800)
                .with_wifi_rssi(-55),
        );

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: DeviceState = serde_json::from_str(&json).unwrap();

        assert_eq!(state, deserialized);
        assert_eq!(deserialized.uptime_seconds(), Some(172800));
    }
}
