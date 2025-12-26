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
//! # Types Overview
//!
//! | Type | Range | Description |
//! |------|-------|-------------|
//! | [`PowerState`] | On/Off/Toggle/Blink | Relay power state |
//! | [`PowerIndex`] | 0-8 | Relay index (0 = all relays) |
//! | [`Dimmer`] | 0-100 | Brightness percentage |
//! | [`ColorTemperature`] | 153-500 mireds | White color temperature |
//! | [`HsbColor`] | H:0-360, S:0-100, B:0-100 | Color in HSB format |
//! | [`RgbColor`] | R:0-255, G:0-255, B:0-255 | Color in RGB format |
//! | [`Scheme`] | 0-4 | Light effect (Single/Wakeup/Cycle/Random) |
//! | [`WakeupDuration`] | 1-3000 seconds | Duration for wakeup effect |
//! | [`FadeSpeed`] | 1-40 | Transition speed (1=fastest) |
//! | [`TasmotaDateTime`] | ISO 8601 | Datetime from telemetry |
//!
//! # Construction Patterns
//!
//! All types with constraints use the newtype pattern with validation:
//!
//! ```
//! use tasmor_lib::types::{Dimmer, ColorTemperature, HsbColor};
//!
//! // Validated construction - returns Result
//! let dimmer = Dimmer::new(75)?;           // Ok(Dimmer(75))
//! let invalid = Dimmer::new(150);          // Err(ValueError)
//!
//! // Clamped construction - always succeeds
//! let clamped = Dimmer::clamped(150);      // Dimmer(100)
//!
//! // Preset values for common use cases
//! let warm = ColorTemperature::WARM;       // 500 mireds (2000K)
//! let cool = ColorTemperature::COOL;       // 153 mireds (6500K)
//! let red = HsbColor::red();               // Pure red
//! # Ok::<(), tasmor_lib::ValueError>(())
//! ```
//!
//! # Type Conversions
//!
//! ```
//! use tasmor_lib::types::{PowerState, Dimmer, ColorTemperature};
//!
//! // PowerState from bool
//! let on: PowerState = true.into();
//! let off: PowerState = false.into();
//!
//! // ColorTemperature to Kelvin
//! let ct = ColorTemperature::new(326)?;
//! assert_eq!(ct.to_kelvin(), 3067);  // ~3000K neutral white
//!
//! // Dimmer as fraction
//! let dim = Dimmer::new(50)?;
//! assert_eq!(dim.as_fraction(), 0.5);
//! # Ok::<(), tasmor_lib::ValueError>(())
//! ```

mod color;
mod datetime;
mod dimmer;
mod power;
mod rgb_color;
mod scheme;
mod speed;
mod wakeup_duration;

pub use color::{ColorTemperature, HsbColor};
pub use datetime::{DateTimeParseError, TasmotaDateTime};
pub use dimmer::Dimmer;
pub use power::{PowerIndex, PowerState};
pub use rgb_color::RgbColor;
pub use scheme::Scheme;
pub use speed::FadeSpeed;
pub use wakeup_duration::WakeupDuration;
