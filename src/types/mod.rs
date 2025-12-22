// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Value types for Tasmota device control.
//!
//! This module provides type-safe representations of values used in Tasmota
//! commands. Each type ensures values are within their valid ranges at
//! construction time, preventing runtime errors.
//!
//! # Types
//!
//! - [`PowerState`] - On/Off/Toggle states for power control
//! - [`PowerIndex`] - Relay index for multi-channel devices (1-8)
//! - [`Dimmer`] - Brightness level (0-100%)
//! - [`ColorTemp`] - Color temperature in mireds (153-500)
//! - [`HsbColor`] - HSB color (Hue 0-360, Saturation 0-100, Brightness 0-100)
//! - [`FadeSpeed`] - Transition speed (1-40)

mod color;
mod dimmer;
mod power;
mod speed;

pub use color::{ColorTemp, HsbColor};
pub use dimmer::Dimmer;
pub use power::{PowerIndex, PowerState};
pub use speed::FadeSpeed;
