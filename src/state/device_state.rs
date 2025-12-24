// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device state tracking.

use crate::types::{ColorTemp, Dimmer, HsbColor, PowerState};

use super::StateChange;

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
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceState {
    /// Power state for each relay (indexed 0-7 for POWER1-POWER8).
    power: [Option<PowerState>; 8],
    /// Dimmer level (0-100).
    dimmer: Option<Dimmer>,
    /// HSB color (hue, saturation, brightness).
    hsb_color: Option<HsbColor>,
    /// Color temperature in mireds (153-500).
    color_temp: Option<ColorTemp>,
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
    /// Timestamp when total energy counting started (ISO 8601 format).
    total_start_time: Option<String>,
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
    pub fn color_temp(&self) -> Option<ColorTemp> {
        self.color_temp
    }

    /// Sets the color temperature.
    pub fn set_color_temp(&mut self, ct: ColorTemp) {
        self.color_temp = Some(ct);
    }

    /// Clears the color temperature.
    pub fn clear_color_temp(&mut self) {
        self.color_temp = None;
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
    /// Returns the timestamp in ISO 8601 format (e.g., "2024-01-15T10:30:00").
    #[must_use]
    pub fn total_start_time(&self) -> Option<&str> {
        self.total_start_time.as_deref()
    }

    /// Sets the timestamp when total energy counting started.
    pub fn set_total_start_time(&mut self, time: String) {
        self.total_start_time = Some(time);
    }

    // ========== State Changes ==========

    /// Applies a state change and returns whether the state actually changed.
    ///
    /// # Returns
    ///
    /// Returns `true` if the state was modified, `false` if it was already
    /// at the target value.
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
            StateChange::ColorTemp(ct) => {
                if self.color_temp == Some(*ct) {
                    false
                } else {
                    self.color_temp = Some(*ct);
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

                // Handle string field separately
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
        assert!(state.color_temp().is_none());
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

        // Color temp should also be set
        assert!(state.color_temp().is_some());
        assert_eq!(state.color_temp().unwrap().value(), 153);
    }
}
