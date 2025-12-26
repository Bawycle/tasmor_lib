// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! State change representation.
//!
//! State changes are the fundamental building blocks for updating device state.
//! They represent discrete changes that can be applied to a [`DeviceState`](super::DeviceState),
//! either from command responses or telemetry updates.
//!
//! # Change Types
//!
//! - [`StateChange::Power`] - Relay state changes (on/off)
//! - [`StateChange::Dimmer`] - Brightness level changes
//! - [`StateChange::HsbColor`] - RGB color changes in HSB format
//! - [`StateChange::ColorTemperature`] - White color temperature changes
//! - [`StateChange::Scheme`] - Light scheme/effect changes
//! - [`StateChange::WakeupDuration`] - Wakeup effect duration changes
//! - [`StateChange::Energy`] - Energy monitoring updates
//! - [`StateChange::Batch`] - Multiple changes grouped together
//!
//! # Examples
//!
//! ## Creating individual changes
//!
//! ```
//! use tasmor_lib::state::StateChange;
//! use tasmor_lib::types::{PowerState, Dimmer, HsbColor, ColorTemperature};
//!
//! // Power state change
//! let power_on = StateChange::power(1, PowerState::On);
//!
//! // Light control changes
//! let dim = StateChange::dimmer(Dimmer::new(75).unwrap());
//! let color = StateChange::hsb_color(HsbColor::red());
//! let warm = StateChange::color_temperature(ColorTemperature::WARM);
//! ```
//!
//! ## Applying changes to device state
//!
//! ```
//! use tasmor_lib::state::{DeviceState, StateChange};
//! use tasmor_lib::types::PowerState;
//!
//! let mut state = DeviceState::new();
//!
//! // Apply returns true if state actually changed
//! let changed = state.apply(&StateChange::power_on());
//! assert!(changed);
//!
//! // Applying same change again returns false
//! let changed = state.apply(&StateChange::power_on());
//! assert!(!changed);
//! ```

use crate::types::{
    ColorTemperature, Dimmer, HsbColor, PowerState, Scheme, TasmotaDateTime, WakeupDuration,
};

/// Represents a change in device state.
///
/// State changes are used to update [`DeviceState`](super::DeviceState) and
/// to emit events when the device state changes. Each variant represents
/// a specific type of state change.
///
/// # Examples
///
/// ```
/// use tasmor_lib::state::StateChange;
/// use tasmor_lib::types::PowerState;
///
/// let change = StateChange::Power { index: 1, state: PowerState::On };
/// ```
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum StateChange {
    /// Power state changed for a specific relay.
    Power {
        /// The relay index (1-8).
        index: u8,
        /// The new power state.
        state: PowerState,
    },

    /// Dimmer level changed.
    Dimmer(Dimmer),

    /// HSB color changed.
    HsbColor(HsbColor),

    /// Color temperature changed.
    ColorTemperature(ColorTemperature),

    /// Light scheme/effect changed.
    Scheme(Scheme),

    /// Wakeup duration changed.
    WakeupDuration(WakeupDuration),

    /// Energy monitoring data updated.
    ///
    /// Contains all energy-related readings. All fields are optional
    /// to allow partial updates from different telemetry sources.
    Energy {
        /// Power consumption in Watts.
        power: Option<f32>,
        /// Voltage in Volts.
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
        /// Total energy consumed in kWh.
        energy_total: Option<f32>,
        /// Timestamp when total energy counting started.
        ///
        /// Contains both naive datetime and timezone-aware datetime if available.
        total_start_time: Option<TasmotaDateTime>,
    },

    /// Multiple changes at once.
    ///
    /// Used when a status refresh returns multiple values.
    Batch(Vec<StateChange>),
}

impl StateChange {
    /// Creates a power state change.
    #[must_use]
    pub fn power(index: u8, state: PowerState) -> Self {
        Self::Power { index, state }
    }

    /// Creates a power-on change for relay 1.
    #[must_use]
    pub fn power_on() -> Self {
        Self::Power {
            index: 1,
            state: PowerState::On,
        }
    }

    /// Creates a power-off change for relay 1.
    #[must_use]
    pub fn power_off() -> Self {
        Self::Power {
            index: 1,
            state: PowerState::Off,
        }
    }

    /// Creates a dimmer change.
    #[must_use]
    pub fn dimmer(value: Dimmer) -> Self {
        Self::Dimmer(value)
    }

    /// Creates an HSB color change.
    #[must_use]
    pub fn hsb_color(color: HsbColor) -> Self {
        Self::HsbColor(color)
    }

    /// Creates a color temperature change.
    #[must_use]
    pub fn color_temperature(ct: ColorTemperature) -> Self {
        Self::ColorTemperature(ct)
    }

    /// Creates a scheme change.
    #[must_use]
    pub fn scheme(scheme: Scheme) -> Self {
        Self::Scheme(scheme)
    }

    /// Creates a wakeup duration change.
    #[must_use]
    pub fn wakeup_duration(duration: WakeupDuration) -> Self {
        Self::WakeupDuration(duration)
    }

    /// Creates an energy reading change with basic power data.
    #[must_use]
    pub fn energy(power: f32, voltage: f32, current: f32) -> Self {
        Self::Energy {
            power: Some(power),
            voltage: Some(voltage),
            current: Some(current),
            apparent_power: None,
            reactive_power: None,
            power_factor: None,
            energy_today: None,
            energy_yesterday: None,
            energy_total: None,
            total_start_time: None,
        }
    }

    /// Creates an energy reading change with all fields.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn energy_full(
        power: Option<f32>,
        voltage: Option<f32>,
        current: Option<f32>,
        apparent_power: Option<f32>,
        reactive_power: Option<f32>,
        power_factor: Option<f32>,
        energy_today: Option<f32>,
        energy_yesterday: Option<f32>,
        energy_total: Option<f32>,
        total_start_time: Option<TasmotaDateTime>,
    ) -> Self {
        Self::Energy {
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
        }
    }

    /// Creates a batch of changes.
    #[must_use]
    pub fn batch(changes: Vec<StateChange>) -> Self {
        Self::Batch(changes)
    }

    /// Returns `true` if this is a power state change.
    #[must_use]
    pub fn is_power(&self) -> bool {
        matches!(self, Self::Power { .. })
    }

    /// Returns `true` if this is a light-related change (dimmer, color, CT, scheme).
    #[must_use]
    pub fn is_light(&self) -> bool {
        matches!(
            self,
            Self::Dimmer(_)
                | Self::HsbColor(_)
                | Self::ColorTemperature(_)
                | Self::Scheme(_)
                | Self::WakeupDuration(_)
        )
    }

    /// Returns `true` if this is a scheme change.
    #[must_use]
    pub fn is_scheme(&self) -> bool {
        matches!(self, Self::Scheme(_))
    }

    /// Returns `true` if this is an energy-related change.
    #[must_use]
    pub fn is_energy(&self) -> bool {
        matches!(self, Self::Energy { .. })
    }

    /// Returns `true` if this is a batch of changes.
    #[must_use]
    pub fn is_batch(&self) -> bool {
        matches!(self, Self::Batch(_))
    }

    /// Returns the number of individual changes.
    ///
    /// For batch changes, returns the total count of nested changes.
    #[must_use]
    pub fn change_count(&self) -> usize {
        match self {
            Self::Batch(changes) => changes.iter().map(Self::change_count).sum(),
            _ => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn power_change_constructors() {
        let change = StateChange::power(1, PowerState::On);
        assert!(matches!(
            change,
            StateChange::Power {
                index: 1,
                state: PowerState::On
            }
        ));

        let on = StateChange::power_on();
        assert!(matches!(
            on,
            StateChange::Power {
                index: 1,
                state: PowerState::On
            }
        ));

        let off = StateChange::power_off();
        assert!(matches!(
            off,
            StateChange::Power {
                index: 1,
                state: PowerState::Off
            }
        ));
    }

    #[test]
    fn is_power() {
        assert!(StateChange::power_on().is_power());
        assert!(!StateChange::Dimmer(Dimmer::MAX).is_power());
    }

    #[test]
    fn is_light() {
        assert!(StateChange::Dimmer(Dimmer::MAX).is_light());
        assert!(!StateChange::power_on().is_light());
    }

    #[test]
    fn is_energy() {
        assert!(StateChange::energy(100.0, 230.0, 0.5).is_energy());
        assert!(!StateChange::power_on().is_energy());
    }

    #[test]
    fn change_count() {
        assert_eq!(StateChange::power_on().change_count(), 1);

        let batch = StateChange::batch(vec![
            StateChange::power_on(),
            StateChange::Dimmer(Dimmer::MAX),
        ]);
        assert_eq!(batch.change_count(), 2);

        // Nested batch
        let nested = StateChange::batch(vec![batch, StateChange::power_off()]);
        assert_eq!(nested.change_count(), 3);
    }
}
