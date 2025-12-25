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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use tokio::sync::{RwLock, mpsc, oneshot};

use crate::error::ProtocolError;

/// Global counter for generating unique client IDs.
static BROKER_CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Configuration for an MQTT broker connection.
#[derive(Debug, Clone)]
pub struct MqttBrokerConfig {
    host: String,
    port: u16,
    credentials: Option<(String, String)>,
    keep_alive: Duration,
    connection_timeout: Duration,
}

impl Default for MqttBrokerConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 1883,
            credentials: None,
            keep_alive: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(10),
        }
    }
}

/// A subscription to a device topic on the broker.
pub(crate) struct DeviceSubscription {
    /// Channel to send messages to this subscriber.
    pub message_tx: mpsc::Sender<BrokerMessage>,
}

/// A message received from the broker.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Will be used when devices connect through the broker
pub struct BrokerMessage {
    /// The full MQTT topic (e.g., `stat/tasmota_bedroom/POWER`).
    pub topic: String,
    /// The message payload.
    pub payload: String,
}

/// An MQTT broker connection that can be shared across multiple devices.
///
/// This represents a persistent connection to an MQTT broker. It handles
/// connection management, message routing, and device subscriptions.
///
/// `MqttBroker` is cheaply cloneable (via `Arc`) and can be passed to
/// multiple devices that communicate through the same broker.
#[derive(Clone)]
pub struct MqttBroker {
    inner: Arc<MqttBrokerInner>,
}

struct MqttBrokerInner {
    /// The MQTT async client for publishing.
    client: AsyncClient,
    /// Active device subscriptions by device topic.
    subscriptions: RwLock<HashMap<String, DeviceSubscription>>,
    /// Configuration used for this connection.
    config: MqttBrokerConfig,
    /// Connection status.
    connected: AtomicBool,
}

impl MqttBroker {
    /// Creates a new builder for configuring an MQTT broker connection.
    #[must_use]
    pub fn builder() -> MqttBrokerBuilder {
        MqttBrokerBuilder::default()
    }

    /// Returns whether the broker is currently connected.
    #[must_use]
    pub fn is_connected(&self) -> bool {
        self.inner.connected.load(Ordering::Acquire)
    }

    /// Returns the host address of the broker.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.inner.config.host
    }

    /// Returns the port of the broker.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.inner.config.port
    }

    /// Returns whether authentication is configured.
    #[must_use]
    pub fn has_credentials(&self) -> bool {
        self.inner.config.credentials.is_some()
    }

    /// Returns the MQTT client for internal use.
    #[allow(dead_code)] // Will be used when devices connect through the broker
    pub(crate) fn client(&self) -> &AsyncClient {
        &self.inner.client
    }

    /// Adds a subscription for a device topic.
    ///
    /// Subscribes to:
    /// - `stat/<topic>/+` for command responses
    /// - `tele/<topic>/+` for telemetry
    ///
    /// # Errors
    ///
    /// Returns error if the MQTT subscription fails.
    #[allow(dead_code)] // Will be used when devices connect through the broker
    pub(crate) async fn add_device_subscription(
        &self,
        device_topic: String,
        message_tx: mpsc::Sender<BrokerMessage>,
    ) -> Result<(), ProtocolError> {
        // Subscribe to stat/<topic>/+ for command responses
        let stat_topic = format!("stat/{device_topic}/+");
        self.inner
            .client
            .subscribe(&stat_topic, QoS::AtLeastOnce)
            .await
            .map_err(ProtocolError::Mqtt)?;

        // Subscribe to tele/<topic>/+ for telemetry
        let tele_topic = format!("tele/{device_topic}/+");
        self.inner
            .client
            .subscribe(&tele_topic, QoS::AtLeastOnce)
            .await
            .map_err(ProtocolError::Mqtt)?;

        tracing::debug!(
            stat = %stat_topic,
            tele = %tele_topic,
            "Subscribed to device topics"
        );

        // Register the subscription
        let subscription = DeviceSubscription { message_tx };
        self.inner
            .subscriptions
            .write()
            .await
            .insert(device_topic, subscription);

        Ok(())
    }

    /// Removes a subscription for a device topic.
    #[allow(dead_code)] // Will be used when devices disconnect from the broker
    pub(crate) async fn remove_device_subscription(&self, device_topic: &str) {
        // Remove from tracking
        self.inner.subscriptions.write().await.remove(device_topic);

        // Unsubscribe from MQTT topics
        let stat_topic = format!("stat/{device_topic}/+");
        let tele_topic = format!("tele/{device_topic}/+");

        if let Err(e) = self.inner.client.unsubscribe(&stat_topic).await {
            tracing::warn!(topic = %stat_topic, error = %e, "Failed to unsubscribe from stat topic");
        }

        if let Err(e) = self.inner.client.unsubscribe(&tele_topic).await {
            tracing::warn!(topic = %tele_topic, error = %e, "Failed to unsubscribe from tele topic");
        }

        tracing::debug!(
            stat = %stat_topic,
            tele = %tele_topic,
            "Unsubscribed from device topics"
        );
    }

    /// Routes an incoming message to the appropriate device subscriber.
    async fn route_message(&self, topic: &str, payload: String) -> Result<(), ProtocolError> {
        // Parse topic: stat/<device_topic>/<command> or tele/<device_topic>/<type>
        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() >= 3 && (parts[0] == "stat" || parts[0] == "tele") {
            let device_topic = parts[1];

            let subscriptions = self.inner.subscriptions.read().await;
            if let Some(sub) = subscriptions.get(device_topic) {
                let message = BrokerMessage {
                    topic: topic.to_string(),
                    payload,
                };
                tracing::debug!(
                    topic = %topic,
                    device = %device_topic,
                    "Routing message to device"
                );
                sub.message_tx.send(message).await.map_err(|e| {
                    ProtocolError::ChannelClosed(format!(
                        "Failed to send message to device {device_topic}: {e}"
                    ))
                })?;
            }
        }
        Ok(())
    }

    /// Disconnects from the broker.
    ///
    /// This will close the connection and clean up all subscriptions.
    ///
    /// # Errors
    ///
    /// Returns error if the disconnect operation fails.
    pub async fn disconnect(&self) -> Result<(), ProtocolError> {
        tracing::info!(
            host = %self.inner.config.host,
            port = %self.inner.config.port,
            "Disconnecting from MQTT broker"
        );

        // Clear all subscriptions
        self.inner.subscriptions.write().await.clear();

        // Disconnect the client
        self.inner
            .client
            .disconnect()
            .await
            .map_err(ProtocolError::Mqtt)?;

        self.inner.connected.store(false, Ordering::Release);
        Ok(())
    }

    /// Returns the number of active device subscriptions.
    pub async fn subscription_count(&self) -> usize {
        self.inner.subscriptions.read().await.len()
    }
}

impl std::fmt::Debug for MqttBroker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttBroker")
            .field("host", &self.inner.config.host)
            .field("port", &self.inner.config.port)
            .field("connected", &self.is_connected())
            .finish()
    }
}

/// Builder for creating an MQTT broker connection.
///
/// # Examples
///
/// ```no_run
/// use tasmor_lib::protocol::MqttBroker;
/// use std::time::Duration;
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// let broker = MqttBroker::builder()
///     .host("192.168.1.50")
///     .port(1883)
///     .credentials("user", "password")
///     .keep_alive(Duration::from_secs(60))
///     .connection_timeout(Duration::from_secs(5))
///     .build()
///     .await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Default)]
pub struct MqttBrokerBuilder {
    config: MqttBrokerConfig,
}

impl MqttBrokerBuilder {
    /// Sets the broker host address.
    #[must_use]
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.config.host = host.into();
        self
    }

    /// Sets the broker port (default: 1883).
    #[must_use]
    pub fn port(mut self, port: u16) -> Self {
        self.config.port = port;
        self
    }

    /// Sets authentication credentials.
    #[must_use]
    pub fn credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.config.credentials = Some((username.into(), password.into()));
        self
    }

    /// Sets the keep-alive interval (default: 30 seconds).
    #[must_use]
    pub fn keep_alive(mut self, duration: Duration) -> Self {
        self.config.keep_alive = duration;
        self
    }

    /// Sets the connection timeout (default: 10 seconds).
    #[must_use]
    pub fn connection_timeout(mut self, duration: Duration) -> Self {
        self.config.connection_timeout = duration;
        self
    }

    /// Builds and connects to the MQTT broker.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Host is not set
    /// - Connection fails
    /// - Connection times out
    pub async fn build(self) -> Result<MqttBroker, ProtocolError> {
        if self.config.host.is_empty() {
            return Err(ProtocolError::InvalidAddress(
                "MQTT broker host is required".to_string(),
            ));
        }

        // Generate unique client ID
        let counter = BROKER_CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let client_id = format!("tasmor_{}_{}", std::process::id(), counter);

        let mut mqtt_options = MqttOptions::new(&client_id, &self.config.host, self.config.port);
        mqtt_options.set_keep_alive(self.config.keep_alive);
        mqtt_options.set_clean_session(true);

        if let Some((ref username, ref password)) = self.config.credentials {
            mqtt_options.set_credentials(username, password);
        }

        let (client, event_loop) = AsyncClient::new(mqtt_options, 10);

        let inner = MqttBrokerInner {
            client,
            subscriptions: RwLock::new(HashMap::new()),
            config: self.config.clone(),
            connected: AtomicBool::new(false),
        };

        let broker = MqttBroker {
            inner: Arc::new(inner),
        };

        // Clone for event loop
        let broker_clone = broker.clone();

        // Channel to signal when ConnAck is received
        let (connack_tx, connack_rx) = oneshot::channel();

        // Spawn event loop handler
        tokio::spawn(async move {
            handle_broker_events(event_loop, broker_clone, Some(connack_tx)).await;
        });

        // Wait for ConnAck with timeout
        let timeout = self.config.connection_timeout;
        match tokio::time::timeout(timeout, connack_rx).await {
            Ok(Ok(())) => {
                broker.inner.connected.store(true, Ordering::Release);
                tracing::info!(
                    host = %self.config.host,
                    port = %self.config.port,
                    "Connected to MQTT broker"
                );
            }
            Ok(Err(_)) => {
                return Err(ProtocolError::ConnectionFailed(
                    "MQTT event loop terminated unexpectedly".to_string(),
                ));
            }
            Err(_) => {
                return Err(ProtocolError::ConnectionFailed(format!(
                    "MQTT connection timeout after {}s",
                    timeout.as_secs()
                )));
            }
        }

        Ok(broker)
    }
}

/// Handles MQTT events for the broker connection.
async fn handle_broker_events(
    mut event_loop: EventLoop,
    broker: MqttBroker,
    connack_tx: Option<oneshot::Sender<()>>,
) {
    use rumqttc::{Event, Packet};

    let mut connack_tx = connack_tx;

    loop {
        match event_loop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(connack))) => {
                tracing::debug!(?connack, "MQTT broker connected");
                broker.inner.connected.store(true, Ordering::Release);
                if let Some(tx) = connack_tx.take() {
                    let _ = tx.send(());
                }
            }
            Ok(Event::Incoming(Packet::SubAck(suback))) => {
                tracing::debug!(?suback, "MQTT subscription acknowledged");
            }
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                if let Ok(payload) = String::from_utf8(publish.payload.to_vec()) {
                    tracing::debug!(
                        topic = %publish.topic,
                        payload = %payload,
                        "MQTT message received"
                    );
                    if let Err(e) = broker.route_message(&publish.topic, payload).await {
                        tracing::warn!(
                            topic = %publish.topic,
                            error = %e,
                            "Failed to route MQTT message"
                        );
                    }
                }
            }
            Ok(Event::Incoming(Packet::Disconnect)) => {
                tracing::info!("MQTT broker disconnected");
                broker.inner.connected.store(false, Ordering::Release);
                break;
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(error = %e, "MQTT broker event loop error");
                broker.inner.connected.store(false, Ordering::Release);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_default_values() {
        let builder = MqttBrokerBuilder::default();
        assert_eq!(builder.config.port, 1883);
        assert!(builder.config.host.is_empty());
        assert!(builder.config.credentials.is_none());
        assert_eq!(builder.config.keep_alive, Duration::from_secs(30));
        assert_eq!(builder.config.connection_timeout, Duration::from_secs(10));
    }

    #[test]
    fn builder_with_host() {
        let builder = MqttBrokerBuilder::default().host("192.168.1.50");
        assert_eq!(builder.config.host, "192.168.1.50");
    }

    #[test]
    fn builder_with_port() {
        let builder = MqttBrokerBuilder::default().port(8883);
        assert_eq!(builder.config.port, 8883);
    }

    #[test]
    fn builder_with_credentials() {
        let builder = MqttBrokerBuilder::default().credentials("user", "pass");
        let creds = builder.config.credentials.unwrap();
        assert_eq!(creds.0, "user");
        assert_eq!(creds.1, "pass");
    }

    #[test]
    fn builder_with_keep_alive() {
        let builder = MqttBrokerBuilder::default().keep_alive(Duration::from_secs(60));
        assert_eq!(builder.config.keep_alive, Duration::from_secs(60));
    }

    #[test]
    fn builder_with_connection_timeout() {
        let builder = MqttBrokerBuilder::default().connection_timeout(Duration::from_secs(5));
        assert_eq!(builder.config.connection_timeout, Duration::from_secs(5));
    }

    #[test]
    fn builder_chain() {
        let builder = MqttBrokerBuilder::default()
            .host("192.168.1.50")
            .port(8883)
            .credentials("admin", "secret")
            .keep_alive(Duration::from_secs(45))
            .connection_timeout(Duration::from_secs(15));

        assert_eq!(builder.config.host, "192.168.1.50");
        assert_eq!(builder.config.port, 8883);
        assert!(builder.config.credentials.is_some());
        assert_eq!(builder.config.keep_alive, Duration::from_secs(45));
        assert_eq!(builder.config.connection_timeout, Duration::from_secs(15));
    }

    #[tokio::test]
    async fn builder_missing_host_fails() {
        let result = MqttBrokerBuilder::default().build().await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, ProtocolError::InvalidAddress(_)));
    }

    #[test]
    fn config_default() {
        let config = MqttBrokerConfig::default();
        assert!(config.host.is_empty());
        assert_eq!(config.port, 1883);
        assert!(config.credentials.is_none());
    }

    #[test]
    fn broker_message_debug() {
        let msg = BrokerMessage {
            topic: "stat/device/POWER".to_string(),
            payload: "ON".to_string(),
        };
        let debug = format!("{msg:?}");
        assert!(debug.contains("BrokerMessage"));
        assert!(debug.contains("POWER"));
        assert!(debug.contains("ON"));
    }
}
