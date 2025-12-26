// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Subscription system for device state changes.
//!
//! This module provides a callback-based subscription system for receiving
//! notifications when device state changes. It is designed for MQTT devices
//! which maintain persistent connections and receive real-time updates.
//!
//! # Overview
//!
//! The subscription system consists of:
//!
//! - [`SubscriptionId`] - A unique identifier for a subscription, used to unsubscribe
//! - [`CallbackRegistry`] - Internal registry that manages callbacks and dispatches events
//! - [`Subscribable`] - Trait for types that support event subscriptions
//!
//! # Usage
//!
//! Subscriptions are typically created through methods on MQTT devices:
//!
//! ```ignore
//! // Subscribe to power state changes
//! let sub_id = device.on_power_changed(|index, state| {
//!     println!("Power {index} changed to {state:?}");
//! });
//!
//! // Later, unsubscribe
//! device.unsubscribe(sub_id);
//! ```
//!
//! # HTTP vs MQTT
//!
//! - **HTTP devices**: Do not support subscriptions (stateless protocol)
//! - **MQTT devices**: Full subscription support via the [`Subscribable`] trait
//!
//! Attempting to call subscription methods on HTTP devices results in a compile-time error.

mod callback;
mod subscribable;

pub use callback::{CallbackRegistry, EnergyData, SubscriptionId};
pub use subscribable::Subscribable;
