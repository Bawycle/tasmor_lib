// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! MQTT broker connection for Tasmota devices.
//!
//! This module provides an explicit MQTT broker connection that can be shared
//! across multiple Tasmota devices. Unlike HTTP which is stateless, MQTT
//! maintains a persistent connection and supports real-time event notifications.
//!
//! # Examples
//!
//! ```no_run
//! use tasmor_lib::protocol::MqttBroker;
//!
//! # async fn example() -> tasmor_lib::Result<()> {
//! // Create a broker connection
//! let broker = MqttBroker::builder()
//!     .host("192.168.1.50")
//!     .port(1883)
//!     .credentials("user", "password")
//!     .build()
//!     .await?;
//!
//! // The broker can be cloned and shared between devices
//! let broker_clone = broker.clone();
//!
//! // Check connection status
//! if broker.is_connected() {
//!     println!("Connected to MQTT broker");
//! }
//!
//! // Disconnect when done
//! broker.disconnect().await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Reconnection Behavior
//!
//! The broker handles connection loss and reconnection automatically:
//!
//! 1. **Connection Lost**: When the MQTT connection is lost, the
//!    [`on_disconnected`](crate::subscription::Subscribable::on_disconnected)
//!    callback is triggered for all devices.
//!
//! 2. **Automatic Reconnection**: The underlying MQTT client (paho-mqtt)
//!    automatically attempts to reconnect to the broker.
//!
//! 3. **Topic Resubscription**: When the connection is restored, all device
//!    topic subscriptions (`stat/<topic>/+` and `tele/<topic>/+`) are
//!    automatically restored.
//!
//! 4. **Reconnection Notification**: The
//!    [`on_reconnected`](crate::subscription::Subscribable::on_reconnected)
//!    callback is triggered for all devices after topics are resubscribed.
//!
//! **Important**: The library does not retain device state. After a reconnection,
//! the application should call [`query_state()`](crate::Device::query_state)
//! to refresh the device state, as it may have changed during the disconnection.
//!
//! ## Example: Handling Reconnection
//!
//! ```no_run
//! use tasmor_lib::MqttBroker;
//! use tasmor_lib::subscription::Subscribable;
//!
//! # async fn example() -> tasmor_lib::Result<()> {
//! let broker = MqttBroker::builder()
//!     .host("192.168.1.50")
//!     .build()
//!     .await?;
//!
//! let (device, _) = broker.device("tasmota_device").build().await?;
//!
//! // Handle disconnection
//! device.on_disconnected(|| {
//!     println!("Connection lost!");
//! });
//!
//! // Handle reconnection
//! device.on_reconnected(|| {
//!     println!("Reconnected! Consider calling query_state()");
//! });
//! # Ok(())
//! # }
//! ```

mod broker;
mod builder;
mod config;
mod events;

use std::time::Duration;

/// Default timeout for MQTT command responses.
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

pub use broker::MqttBroker;
pub use builder::MqttBrokerBuilder;
pub use config::MqttBrokerConfig;
