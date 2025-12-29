// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! MQTT device auto-discovery for Tasmota devices.
//!
//! This module provides functionality to automatically discover Tasmota devices
//! on an MQTT broker by listening for their telemetry messages.
//!
//! # Discovery Mechanism
//!
//! The discovery process works by subscribing to wildcard MQTT topics:
//!
//! - `tele/+/LWT` - Last Will Testament messages (device online/offline status)
//! - `tele/+/STATE` - Periodic state messages from devices
//!
//! When a device publishes to these topics, its topic name is extracted and
//! used to create a fully configured [`Device`] instance.
//!
//! # Examples
//!
//! ## Discovery via `MqttBroker` (Recommended)
//!
//! ```no_run
//! use tasmor_lib::MqttBroker;
//! use std::time::Duration;
//!
//! # async fn example() -> tasmor_lib::Result<()> {
//! // Connect to broker
//! let broker = MqttBroker::builder()
//!     .host("192.168.1.50")
//!     .port(1883)
//!     .credentials("user", "password")
//!     .build()
//!     .await?;
//!
//! // Discover devices (10 second timeout)
//! let devices = broker.discover_devices(Duration::from_secs(10)).await?;
//!
//! println!("Found {} devices:", devices.len());
//! for (device, state) in &devices {
//!     println!("  - Power: {:?}", state.power(1));
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ## Standalone Discovery (Convenience)
//!
//! For one-off discovery, creating a broker connection for you:
//!
//! ```no_run
//! use tasmor_lib::discovery::{discover_devices, DiscoveryOptions};
//! use std::time::Duration;
//!
//! # async fn example() -> tasmor_lib::Result<()> {
//! let options = DiscoveryOptions::new()
//!     .with_timeout(Duration::from_secs(10))
//!     .with_credentials("user", "pass");
//!
//! let (broker, devices) = discover_devices("192.168.1.50", Some(options)).await?;
//!
//! for (device, state) in &devices {
//!     device.power_toggle().await?;
//! }
//!
//! // Disconnect when done
//! broker.disconnect().await?;
//! # Ok(())
//! # }
//! ```

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use rumqttc::QoS;
use tokio::sync::RwLock;

use crate::device::Device;
use crate::error::{Error, ProtocolError};
use crate::protocol::{MqttBroker, SharedMqttClient};
use crate::state::DeviceState;

/// Default discovery timeout.
const DEFAULT_DISCOVERY_TIMEOUT: Duration = Duration::from_secs(5);

/// Options for standalone MQTT device discovery.
///
/// Use this when calling [`discover_devices`] without an existing broker connection.
/// When using [`MqttBroker::discover_devices`], only the timeout is needed.
///
/// # Examples
///
/// ```
/// use tasmor_lib::discovery::DiscoveryOptions;
/// use std::time::Duration;
///
/// let options = DiscoveryOptions::new()
///     .with_timeout(Duration::from_secs(10))
///     .with_credentials("user", "pass");
/// ```
#[derive(Debug, Clone, Default)]
pub struct DiscoveryOptions {
    /// How long to listen for device announcements.
    timeout: Option<Duration>,
    /// MQTT broker credentials (username, password).
    credentials: Option<(String, String)>,
    /// MQTT broker port (default: 1883).
    port: Option<u16>,
}

impl DiscoveryOptions {
    /// Creates a new `DiscoveryOptions` with default settings.
    ///
    /// Default timeout is 5 seconds.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the discovery timeout.
    ///
    /// This is how long the discovery process will listen for device
    /// announcements before returning the discovered devices.
    ///
    /// # Arguments
    ///
    /// * `timeout` - The discovery timeout duration
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Sets the MQTT broker credentials.
    ///
    /// # Arguments
    ///
    /// * `username` - MQTT broker username
    /// * `password` - MQTT broker password
    #[must_use]
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.credentials = Some((username.into(), password.into()));
        self
    }

    /// Sets the MQTT broker port.
    ///
    /// Default is 1883.
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = Some(port);
        self
    }

    /// Returns the discovery timeout.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.timeout.unwrap_or(DEFAULT_DISCOVERY_TIMEOUT)
    }

    /// Returns the credentials if set.
    #[must_use]
    pub fn credentials(&self) -> Option<(&str, &str)> {
        self.credentials
            .as_ref()
            .map(|(u, p)| (u.as_str(), p.as_str()))
    }

    /// Returns the port if set.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port.unwrap_or(1883)
    }
}

impl MqttBroker {
    /// Discovers Tasmota devices on this broker.
    ///
    /// This method listens for Tasmota device announcements and returns
    /// ready-to-use [`Device`] instances with their initial state.
    ///
    /// # Discovery Process
    ///
    /// 1. Subscribes to wildcard topics (`tele/+/LWT`, `tele/+/STATE`)
    /// 2. Collects device topics from incoming messages during the timeout period
    /// 3. Creates a [`Device`] instance for each discovered topic
    /// 4. Queries each device for capabilities and initial state
    ///
    /// # Arguments
    ///
    /// * `timeout` - How long to listen for device announcements
    ///
    /// # Returns
    ///
    /// A vector of `(Device, DeviceState)` tuples for each discovered device.
    /// Devices that fail to respond to status queries are skipped.
    ///
    /// # Errors
    ///
    /// Returns error if subscription to discovery topics fails.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tasmor_lib::MqttBroker;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> tasmor_lib::Result<()> {
    /// let broker = MqttBroker::builder()
    ///     .host("192.168.1.50")
    ///     .credentials("user", "pass")
    ///     .build()
    ///     .await?;
    ///
    /// let devices = broker.discover_devices(Duration::from_secs(10)).await?;
    ///
    /// for (device, state) in devices {
    ///     println!("Found device with power state: {:?}", state.power(1));
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover_devices(
        &self,
        timeout: Duration,
    ) -> Result<Vec<(Device<SharedMqttClient>, DeviceState)>, Error> {
        tracing::info!(
            host = %self.host(),
            port = %self.port(),
            timeout_secs = timeout.as_secs(),
            "Starting MQTT device discovery"
        );

        // Subscribe to wildcard topics for discovery
        // tele/+/LWT - Last Will Testament (online/offline status)
        // tele/+/STATE - Periodic state updates
        // stat/+/STATUS - Response to Status command
        self.client()
            .subscribe("tele/+/LWT", QoS::AtMostOnce)
            .await
            .map_err(ProtocolError::Mqtt)?;

        self.client()
            .subscribe("tele/+/STATE", QoS::AtMostOnce)
            .await
            .map_err(ProtocolError::Mqtt)?;

        self.client()
            .subscribe("stat/+/STATUS", QoS::AtMostOnce)
            .await
            .map_err(ProtocolError::Mqtt)?;

        tracing::debug!("Subscribed to discovery topics");

        // Trigger all devices to respond by sending Status command to default group topic
        // Tasmota devices have "tasmotas" as default GroupTopic1
        self.client()
            .publish("cmnd/tasmotas/Status", QoS::AtMostOnce, false, "0")
            .await
            .map_err(ProtocolError::Mqtt)?;

        tracing::debug!("Sent broadcast Status command to trigger device responses");

        // Collect device topics during the timeout period
        let topics = self.collect_device_topics(timeout).await;

        // Unsubscribe from discovery topics
        let _ = self.client().unsubscribe("tele/+/LWT").await;
        let _ = self.client().unsubscribe("tele/+/STATE").await;
        let _ = self.client().unsubscribe("stat/+/STATUS").await;

        tracing::info!(count = topics.len(), "Discovered device topics");

        if topics.is_empty() {
            return Ok(Vec::new());
        }

        // Create devices for each discovered topic
        let mut devices = Vec::with_capacity(topics.len());

        for topic in topics {
            tracing::debug!(topic = %topic, "Creating device for discovered topic");

            match self.create_device_for_topic(&topic).await {
                Ok(device_and_state) => {
                    tracing::info!(topic = %topic, "Successfully created device");
                    devices.push(device_and_state);
                }
                Err(e) => {
                    tracing::warn!(topic = %topic, error = %e, "Failed to create device, skipping");
                }
            }
        }

        tracing::info!(
            discovered = devices.len(),
            "MQTT device discovery completed"
        );

        Ok(devices)
    }

    /// Collects device topics by monitoring MQTT messages.
    async fn collect_device_topics(&self, timeout: Duration) -> HashSet<String> {
        // Start discovery mode - the broker will send discovered topics to this channel
        let mut discovery_rx = self.start_discovery().await;
        let discovered_topics: Arc<RwLock<HashSet<String>>> = Arc::new(RwLock::new(HashSet::new()));

        // Spawn a task to collect topics from the channel
        let topics_clone = discovered_topics.clone();
        let collector = tokio::spawn(async move {
            while let Some(topic) = discovery_rx.recv().await {
                topics_clone.write().await.insert(topic);
            }
        });

        // Wait for the timeout period
        tokio::time::sleep(timeout).await;

        // Stop discovery mode and wait for collector to finish
        self.stop_discovery().await;
        collector.abort();

        discovered_topics.read().await.clone()
    }

    /// Creates a Device for a discovered topic using the shared broker connection.
    async fn create_device_for_topic(
        &self,
        topic: &str,
    ) -> Result<(Device<SharedMqttClient>, DeviceState), Error> {
        // Use the broker's shared connection via device()
        self.device(topic).build().await
    }
}

/// Discovers Tasmota devices on an MQTT broker.
///
/// This is a convenience function that connects to a broker, discovers devices,
/// and returns both the broker connection and the discovered devices.
///
/// The returned devices share the broker's MQTT connection, so you must keep
/// the broker alive while using the devices. When you're done, call
/// [`MqttBroker::disconnect`] to clean up.
///
/// # Arguments
///
/// * `host` - The MQTT broker host (e.g., `192.168.1.50`)
/// * `options` - Optional discovery configuration (timeout, credentials, port)
///
/// # Returns
///
/// A tuple of `(MqttBroker, Vec<(Device, DeviceState)>)`. The broker must be kept
/// alive while using the devices.
///
/// # Errors
///
/// Returns error if connection to the broker fails.
///
/// # Examples
///
/// ```no_run
/// use tasmor_lib::discovery::{discover_devices, DiscoveryOptions};
/// use std::time::Duration;
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// // Discovery with custom options
/// let options = DiscoveryOptions::new()
///     .with_timeout(Duration::from_secs(10))
///     .with_credentials("user", "pass");
///
/// let (broker, devices) = discover_devices("192.168.1.50", Some(options)).await?;
///
/// for (device, state) in &devices {
///     println!("Found device with power state: {:?}", state.power(1));
/// }
///
/// // Use devices...
/// // When done, disconnect the broker
/// broker.disconnect().await?;
/// # Ok(())
/// # }
/// ```
pub async fn discover_devices(
    host: &str,
    options: Option<DiscoveryOptions>,
) -> Result<(MqttBroker, Vec<(Device<SharedMqttClient>, DeviceState)>), Error> {
    let options = options.unwrap_or_default();

    // Build broker connection
    let mut builder = MqttBroker::builder().host(host).port(options.port());

    if let Some((username, password)) = options.credentials() {
        builder = builder.credentials(username, password);
    }

    let broker = builder.build().await?;

    // Perform discovery
    let devices = broker.discover_devices(options.timeout()).await?;

    Ok((broker, devices))
}

/// Extracts the device topic from an MQTT topic string.
///
/// # Examples
///
/// - `tele/tasmota_bulb/LWT` → `Some("tasmota_bulb")`
/// - `tele/my_device/STATE` → `Some("my_device")`
#[allow(dead_code)]
fn extract_device_topic(mqtt_topic: &str) -> Option<&str> {
    let parts: Vec<&str> = mqtt_topic.split('/').collect();
    if parts.len() >= 3 && parts[0] == "tele" {
        Some(parts[1])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovery_options_default() {
        let options = DiscoveryOptions::default();
        assert_eq!(options.timeout(), Duration::from_secs(5));
        assert!(options.credentials().is_none());
        assert_eq!(options.port(), 1883);
    }

    #[test]
    fn discovery_options_new() {
        let options = DiscoveryOptions::new();
        assert_eq!(options.timeout(), Duration::from_secs(5));
    }

    #[test]
    fn discovery_options_with_timeout() {
        let options = DiscoveryOptions::new().with_timeout(Duration::from_secs(10));
        assert_eq!(options.timeout(), Duration::from_secs(10));
    }

    #[test]
    fn discovery_options_with_credentials() {
        let options = DiscoveryOptions::new().with_credentials("user", "pass");
        assert_eq!(options.credentials(), Some(("user", "pass")));
    }

    #[test]
    fn discovery_options_with_port() {
        let options = DiscoveryOptions::new().with_port(8883);
        assert_eq!(options.port(), 8883);
    }

    #[test]
    fn discovery_options_chained() {
        let options = DiscoveryOptions::new()
            .with_timeout(Duration::from_secs(15))
            .with_credentials("mqtt_user", "mqtt_pass")
            .with_port(1884);

        assert_eq!(options.timeout(), Duration::from_secs(15));
        assert_eq!(options.credentials(), Some(("mqtt_user", "mqtt_pass")));
        assert_eq!(options.port(), 1884);
    }

    #[test]
    fn extract_device_topic_lwt() {
        assert_eq!(
            extract_device_topic("tele/tasmota_bulb/LWT"),
            Some("tasmota_bulb")
        );
    }

    #[test]
    fn extract_device_topic_state() {
        assert_eq!(
            extract_device_topic("tele/my_device/STATE"),
            Some("my_device")
        );
    }

    #[test]
    fn extract_device_topic_invalid() {
        assert_eq!(extract_device_topic("stat/device/RESULT"), None);
        assert_eq!(extract_device_topic("invalid"), None);
        assert_eq!(extract_device_topic("only/two"), None);
    }
}
