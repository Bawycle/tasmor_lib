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
use crate::protocol::TopicRouter;

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
    /// Channel to send command responses (RESULT, STATUS*) to the device.
    pub response_tx: mpsc::Sender<String>,
    /// Router for dispatching messages to callbacks.
    pub router: Arc<TopicRouter>,
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
    /// Channel for sending discovered device topics during discovery.
    discovery_tx: RwLock<Option<mpsc::Sender<String>>>,
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

    /// Returns the credentials if configured.
    #[must_use]
    #[allow(dead_code)] // May be useful for future discovery/configuration features
    pub(crate) fn credentials(&self) -> Option<(&str, &str)> {
        self.inner
            .config
            .credentials
            .as_ref()
            .map(|(u, p)| (u.as_str(), p.as_str()))
    }

    /// Returns the MQTT client for internal use.
    pub(crate) fn client(&self) -> &AsyncClient {
        &self.inner.client
    }

    /// Creates a builder for a device that shares this broker's MQTT connection.
    ///
    /// This is the recommended way to create multiple devices on the same broker,
    /// as they will all share a single MQTT connection instead of each creating
    /// their own.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tasmor_lib::MqttBroker;
    ///
    /// # async fn example() -> tasmor_lib::Result<()> {
    /// let broker = MqttBroker::builder()
    ///     .host("192.168.1.50")
    ///     .credentials("user", "pass")
    ///     .build()
    ///     .await?;
    ///
    /// // All devices share the same connection
    /// let (bulb, _) = broker.device("tasmota_bulb").build().await?;
    /// let (plug, _) = broker.device("tasmota_plug").build().await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn device(&self, topic: impl Into<String>) -> crate::device::BrokerDeviceBuilder<'_> {
        crate::device::BrokerDeviceBuilder::new(self, topic)
    }

    /// Adds a subscription for a device topic.
    ///
    /// Subscribes to:
    /// - `stat/<topic>/+` for command responses
    /// - `tele/<topic>/+` for telemetry
    ///
    /// Returns a receiver channel for command responses.
    ///
    /// # Errors
    ///
    /// Returns error if the MQTT subscription fails.
    pub(crate) async fn add_device_subscription(
        &self,
        device_topic: String,
    ) -> Result<(mpsc::Receiver<String>, Arc<TopicRouter>), ProtocolError> {
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

        // Create channels and router for this device
        let (response_tx, response_rx) = mpsc::channel::<String>(10);
        let router = Arc::new(TopicRouter::new());

        // Register the subscription
        let subscription = DeviceSubscription {
            response_tx,
            router: Arc::clone(&router),
        };
        self.inner
            .subscriptions
            .write()
            .await
            .insert(device_topic, subscription);

        Ok((response_rx, router))
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
    async fn route_message(&self, topic: &str, payload: String) {
        // Parse topic: stat/<device_topic>/<command> or tele/<device_topic>/<type>
        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() < 3 {
            return;
        }

        let prefix = parts[0];
        let device_topic = parts[1];
        let suffix = parts[2];

        if prefix != "stat" && prefix != "tele" {
            return;
        }

        // Check for discovery mode - capture device topics from discovery messages
        // tele/+/LWT, tele/+/STATE, or stat/+/STATUS
        let is_discovery_topic = (prefix == "tele" && (suffix == "LWT" || suffix == "STATE"))
            || (prefix == "stat" && suffix == "STATUS");

        if is_discovery_topic
            && let Some(discovery_tx) = self.inner.discovery_tx.read().await.as_ref()
        {
            tracing::debug!(
                topic = %topic,
                device = %device_topic,
                "Discovered device topic"
            );
            // Ignore send errors - discovery may have stopped
            let _ = discovery_tx.send(device_topic.to_string()).await;
        }

        // Route to registered device subscriptions
        let subscriptions = self.inner.subscriptions.read().await;
        let Some(sub) = subscriptions.get(device_topic) else {
            return;
        };

        // Route to callbacks via the topic router
        sub.router.route(topic, &payload);

        // For stat/ messages, also send to response channel if it's a command response
        if prefix == "stat" {
            // RESULT and STATUS* are JSON responses that go to the response channel
            let is_json_response = suffix == "RESULT" || suffix.starts_with("STATUS");
            if is_json_response {
                tracing::debug!(
                    topic = %topic,
                    device = %device_topic,
                    "Routing response to device"
                );
                // Ignore send errors - the device may have been dropped
                let _ = sub.response_tx.send(payload).await;
            }
        }
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

    /// Starts discovery mode and returns a receiver for discovered device topics.
    ///
    /// While in discovery mode, any message received on `tele/+/LWT` or `tele/+/STATE`
    /// topics will have its device topic sent to the returned receiver.
    pub(crate) async fn start_discovery(&self) -> mpsc::Receiver<String> {
        let (tx, rx) = mpsc::channel::<String>(100);
        *self.inner.discovery_tx.write().await = Some(tx);
        rx
    }

    /// Stops discovery mode.
    pub(crate) async fn stop_discovery(&self) {
        *self.inner.discovery_tx.write().await = None;
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
            discovery_tx: RwLock::new(None),
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
                    broker.route_message(&publish.topic, payload).await;
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
}
