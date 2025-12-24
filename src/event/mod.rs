// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Event system for device state changes.
//!
//! This module provides a pub/sub event system for notifying subscribers about
//! device state changes. The [`EventBus`] uses tokio's broadcast channel to
//! allow multiple subscribers to receive events.
//!
//! # Examples
//!
//! ```
//! use tasmor_lib::event::{DeviceId, DeviceEvent, EventBus};
//!
//! let bus = EventBus::new();
//!
//! // Subscribe to events
//! let mut rx = bus.subscribe();
//!
//! // Publish an event
//! let device_id = DeviceId::new();
//! bus.publish(DeviceEvent::DeviceAdded { device_id });
//! ```

mod device_event;
mod device_id;
mod event_bus;

pub use device_event::DeviceEvent;
pub use device_id::DeviceId;
pub use event_bus::EventBus;
