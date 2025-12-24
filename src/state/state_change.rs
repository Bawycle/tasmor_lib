// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! State change representation.

use crate::types::{ColorTemp, Dimmer, HsbColor, PowerState};

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
#[derive(Debug, Clone, PartialEq)]
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
    ColorTemp(ColorTemp),

    /// Energy monitoring data updated.
    ///
    /// Contains instantaneous power, voltage, and current readings.
    Energy {
        /// Power consumption in Watts.
        power: f32,
        /// Voltage in Volts.
        voltage: f32,
        /// Current in Amperes.
        current: f32,
    },

    /// Total energy consumption updated.
    EnergyTotal(f32),

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
    pub fn color_temp(ct: ColorTemp) -> Self {
        Self::ColorTemp(ct)
    }

    /// Creates an energy reading change.
    #[must_use]
    pub fn energy(power: f32, voltage: f32, current: f32) -> Self {
        Self::Energy {
            power,
            voltage,
            current,
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

    /// Returns `true` if this is a light-related change (dimmer, color, CT).
    #[must_use]
    pub fn is_light(&self) -> bool {
        matches!(
            self,
            Self::Dimmer(_) | Self::HsbColor(_) | Self::ColorTemp(_)
        )
    }

    /// Returns `true` if this is an energy-related change.
    #[must_use]
    pub fn is_energy(&self) -> bool {
        matches!(self, Self::Energy { .. } | Self::EnergyTotal(_))
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
        assert!(StateChange::EnergyTotal(1.5).is_energy());
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
