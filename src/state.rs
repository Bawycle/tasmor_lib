// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device state management types.
//!
//! This module provides types for tracking and updating Tasmota device state.
//! The [`DeviceState`] struct maintains the current state of a device, while
//! [`StateChange`] represents individual state changes that can be applied.
//!
//! # Examples
//!
//! ```
//! use tasmor_lib::state::{DeviceState, StateChange};
//! use tasmor_lib::types::PowerState;
//!
//! let mut state = DeviceState::new();
//!
//! // Apply a power state change
//! let change = StateChange::Power { index: 1, state: PowerState::On };
//! state.apply(&change);
//!
//! assert_eq!(state.power(1), Some(PowerState::On));
//! ```

mod device_state;
mod state_change;

pub use device_state::{DeviceState, SystemInfo};
pub use state_change::StateChange;
