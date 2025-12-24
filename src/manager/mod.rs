// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device manager for coordinating multiple Tasmota devices.
//!
//! This module provides a high-level API for managing multiple Tasmota devices
//! with connection pooling, state management, and event distribution.
//!
//! # Overview
//!
//! The [`DeviceManager`] is the central component for applications that need to
//! control multiple Tasmota devices. It provides:
//!
//! - **Centralized device management**: Add, remove, connect, and disconnect devices
//! - **Connection pooling**: MQTT connections are shared between devices on the same broker
//! - **State tracking**: Device state is automatically updated and can be queried or watched
//! - **Event system**: Subscribe to device events via broadcast channels
//! - **Auto-reconnection**: Configurable automatic reconnection on connection loss
//!
//! # Examples
//!
//! ## Basic Usage
//!
//! ```no_run
//! use tasmor_lib::manager::{DeviceManager, DeviceConfig};
//! use tasmor_lib::Dimmer;
//!
//! #[tokio::main]
//! async fn main() -> tasmor_lib::Result<()> {
//!     let manager = DeviceManager::new();
//!
//!     // Add a device
//!     let config = DeviceConfig::mqtt("mqtt://192.168.1.50:1883", "living_room_light")
//!         .with_friendly_name("Living Room Light");
//!     let device_id = manager.add_device(config).await;
//!
//!     // Connect to the device
//!     manager.connect(device_id).await?;
//!
//!     // Control the device
//!     manager.power_on(device_id).await?;
//!     manager.set_dimmer(device_id, Dimmer::new(75)?).await?;
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Event Subscription
//!
//! ```no_run
//! use tasmor_lib::manager::DeviceManager;
//! use tasmor_lib::event::DeviceEvent;
//!
//! # fn example() {
//! let manager = DeviceManager::new();
//! let mut events = manager.subscribe();
//!
//! tokio::spawn(async move {
//!     while let Ok(event) = events.recv().await {
//!         match event {
//!             DeviceEvent::StateChanged { device_id, change, new_state } => {
//!                 println!("Device {:?} state changed: {:?}", device_id, change);
//!             }
//!             DeviceEvent::ConnectionChanged { device_id, connected, .. } => {
//!                 println!("Device {:?} connected: {}", device_id, connected);
//!             }
//!             _ => {}
//!         }
//!     }
//! });
//! # }
//! ```
//!
//! ## Watching Device State
//!
//! ```no_run
//! use tasmor_lib::manager::{DeviceManager, DeviceConfig};
//!
//! # async fn example() {
//! let manager = DeviceManager::new();
//! let config = DeviceConfig::mqtt("mqtt://broker:1883", "device");
//! let device_id = manager.add_device(config).await;
//!
//! // Get a watch receiver for the device
//! if let Some(mut state_rx) = manager.watch_device(device_id).await {
//!     tokio::spawn(async move {
//!         while state_rx.changed().await.is_ok() {
//!             let state = state_rx.borrow();
//!             println!("Current dimmer: {:?}", state.dimmer());
//!         }
//!     });
//! }
//! # }
//! ```

mod device_config;
mod device_manager;
mod managed_device;

pub use device_config::{ConnectionConfig, DeviceConfig, ReconnectionPolicy};
pub use device_manager::DeviceManager;
pub use managed_device::ConnectionState;
