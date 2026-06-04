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

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use paho_mqtt::{AsyncClient, QoS};
use tokio::sync::{RwLock, mpsc};

use crate::credentials::Credentials;
use crate::error::ProtocolError;
use crate::protocol::TopicRouter;
use crate::protocol::response_collector::MqttMessage;

/// Global counter for generating unique client IDs.
static BROKER_CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Default timeout for MQTT command responses.
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(5);

/// Events dispatched from paho-mqtt callbacks to the Tokio event loop.
enum BrokerEvent {
    /// An incoming MQTT message was received.
    Message { topic: String, payload: String },
    /// The connection to the broker was lost.
    ConnectionLost,
    /// The connection to the broker was (re)established.
    Reconnected,
}

/// Configuration for an MQTT broker connection.
///
/// # Security
///
/// Credentials are stored in [`zeroize::Zeroizing`]-backed allocations — the heap bytes
/// are overwritten with zeros when this config is dropped. The config is retained in the
/// broker's inner state for the broker's entire lifetime, so zeroization occurs at broker
/// drop, not at connection time.
///
/// `paho-mqtt` maintains its own C-heap copy of the credentials for the connection
/// lifetime — that copy is outside Rust's control. Use TLS to protect credentials in transit.
#[derive(Debug, Clone)]
pub struct MqttBrokerConfig {
    host: String,
    port: u16,
    credentials: Option<Credentials>,
    keep_alive: Duration,
    connection_timeout: Duration,
    command_timeout: Duration,
}

impl Default for MqttBrokerConfig {
    fn default() -> Self {
        Self {
            host: String::new(),
            port: 1883,
            credentials: None,
            keep_alive: Duration::from_secs(30),
            connection_timeout: Duration::from_secs(10),
            command_timeout: DEFAULT_COMMAND_TIMEOUT,
        }
    }
}

/// A subscription to a device topic on the broker.
pub(crate) struct DeviceSubscription {
    /// Channel to send command responses (RESULT, STATUS*) to the device.
    pub response_tx: mpsc::Sender<MqttMessage>,
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
    /// The paho-mqtt async client for publishing and subscribing.
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

    /// Returns the command timeout for devices on this broker.
    #[must_use]
    pub fn command_timeout(&self) -> Duration {
        self.inner.config.command_timeout
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
    /// Returns a receiver channel for command responses (with topic suffix metadata).
    ///
    /// # Errors
    ///
    /// Returns error if the MQTT subscription fails.
    pub(crate) async fn add_device_subscription(
        &self,
        device_topic: String,
    ) -> Result<(mpsc::Receiver<MqttMessage>, Arc<TopicRouter>), ProtocolError> {
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

        // Channel capacity increased to handle multi-message responses (e.g., Status 0)
        let (response_tx, response_rx) = mpsc::channel::<MqttMessage>(20);
        let router = Arc::new(TopicRouter::new());

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
    pub(crate) async fn remove_device_subscription(&self, device_topic: &str) {
        self.inner.subscriptions.write().await.remove(device_topic);

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

        // Capture device topics for active discovery sessions
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
            let _ = discovery_tx.send(device_topic.to_string()).await;
        }

        let subscriptions = self.inner.subscriptions.read().await;
        let Some(sub) = subscriptions.get(device_topic) else {
            return;
        };

        sub.router.route(topic, &payload);

        if prefix == "stat" {
            let is_json_response = suffix == "RESULT" || suffix.starts_with("STATUS");
            if is_json_response {
                tracing::debug!(
                    topic = %topic,
                    device = %device_topic,
                    suffix = %suffix,
                    "Routing response to device"
                );
                let msg = MqttMessage::new(suffix.to_string(), payload);
                let _ = sub.response_tx.send(msg).await;
            }
        }
    }

    /// Resubscribes to all device topics after a reconnection.
    ///
    /// Called automatically when the MQTT connection is restored. Resubscribes
    /// to all registered device topics and dispatches `on_reconnected` callbacks.
    async fn handle_reconnection(&self) {
        let subscriptions = self.inner.subscriptions.read().await;

        for (device_topic, subscription) in subscriptions.iter() {
            let stat_topic = format!("stat/{device_topic}/+");
            let tele_topic = format!("tele/{device_topic}/+");

            if let Err(e) = self
                .inner
                .client
                .subscribe(&stat_topic, QoS::AtLeastOnce)
                .await
            {
                tracing::error!(topic = %stat_topic, error = %e, "Failed to resubscribe to stat topic");
            }

            if let Err(e) = self
                .inner
                .client
                .subscribe(&tele_topic, QoS::AtLeastOnce)
                .await
            {
                tracing::error!(topic = %tele_topic, error = %e, "Failed to resubscribe to tele topic");
            }

            tracing::debug!(device = %device_topic, "Resubscribed to device topics");

            subscription.router.dispatch_reconnected_all();
        }

        tracing::info!(
            device_count = subscriptions.len(),
            "Reconnection complete, all devices notified"
        );
    }

    /// Dispatches disconnection event to all registered devices.
    async fn dispatch_disconnected_all(&self) {
        let subscriptions = self.inner.subscriptions.read().await;
        for (device_topic, subscription) in subscriptions.iter() {
            tracing::debug!(device = %device_topic, "Notifying device of disconnection");
            subscription.router.dispatch_disconnected_all();
        }
    }

    /// Disconnects from the broker.
    ///
    /// Closes the connection and cleans up all subscriptions.
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

        self.inner.subscriptions.write().await.clear();

        self.inner
            .client
            .disconnect(None)
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
        self.config.credentials = Some(Credentials::new(username, password));
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

    /// Sets the timeout for waiting on command responses (default: 5 seconds).
    ///
    /// This timeout applies to all commands sent via devices created from this broker.
    /// Increase this value if you have slow-responding devices or routines with delays.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tasmor_lib::MqttBroker;
    /// use std::time::Duration;
    ///
    /// # async fn example() -> tasmor_lib::Result<()> {
    /// // Increase timeout for slow devices
    /// let broker = MqttBroker::builder()
    ///     .host("192.168.1.50")
    ///     .command_timeout(Duration::from_secs(15))
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn command_timeout(mut self, duration: Duration) -> Self {
        self.config.command_timeout = duration;
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

        let counter = BROKER_CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let client_id = format!("tasmor_{}_{}", std::process::id(), counter);

        let create_opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(format!("tcp://{}:{}", self.config.host, self.config.port))
            .client_id(client_id)
            .finalize();

        let client = paho_mqtt::AsyncClient::new(create_opts)
            .map_err(|e| ProtocolError::ConnectionFailed(e.to_string()))?;

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

        // Channels bridge paho C-thread callbacks into the Tokio event loop.
        let (event_tx, event_rx) = mpsc::unbounded_channel::<BrokerEvent>();

        // Message callback: route incoming publishes.
        {
            let tx = event_tx.clone();
            broker.inner.client.set_message_callback(move |_cli, msg| {
                if let Some(msg) = msg {
                    let _ = tx.send(BrokerEvent::Message {
                        topic: msg.topic().to_string(),
                        payload: msg.payload_str().into_owned(),
                    });
                }
            });
        }

        // Connection-lost callback: fires when the TCP connection drops.
        {
            let tx = event_tx.clone();
            broker
                .inner
                .client
                .set_connection_lost_callback(move |_cli| {
                    let _ = tx.send(BrokerEvent::ConnectionLost);
                });
        }

        // Connect options with automatic exponential-backoff reconnection.
        let conn_opts = {
            let mut b = paho_mqtt::ConnectOptionsBuilder::new();
            b.keep_alive_interval(self.config.keep_alive)
                .clean_session(true)
                .automatic_reconnect(Duration::from_millis(500), Duration::from_secs(60));
            if let Some(creds) = &self.config.credentials {
                b.user_name(creds.username()).password(creds.password());
            }
            b.finalize()
        };

        // Connect and wait with a user-defined timeout.
        let timeout = self.config.connection_timeout;
        match tokio::time::timeout(timeout, broker.inner.client.connect(conn_opts)).await {
            Ok(Ok(_)) => {
                broker.inner.connected.store(true, Ordering::Release);
                tracing::info!(
                    host = %self.config.host,
                    port = %self.config.port,
                    "Connected to MQTT broker"
                );
            }
            Ok(Err(e)) => {
                return Err(ProtocolError::ConnectionFailed(e.to_string()));
            }
            Err(_) => {
                return Err(ProtocolError::ConnectionFailed(format!(
                    "MQTT connection timeout after {}s",
                    timeout.as_secs()
                )));
            }
        }

        // Set the reconnected callback only AFTER the initial connect completes,
        // so it fires exclusively for subsequent reconnections by paho-mqtt's
        // automatic-reconnect logic.
        {
            let tx = event_tx;
            broker.inner.client.set_connected_callback(move |_cli| {
                let _ = tx.send(BrokerEvent::Reconnected);
            });
        }

        let broker_clone = broker.clone();
        tokio::spawn(async move {
            handle_broker_events(event_rx, broker_clone).await;
        });

        Ok(broker)
    }
}

/// Processes broker events forwarded from paho-mqtt's C-thread callbacks.
///
/// Runs for the lifetime of the broker, handling:
/// - Incoming messages → routed to subscribed devices
/// - Connection loss → `on_disconnected` callbacks dispatched
/// - Reconnection → topics resubscribed, `on_reconnected` callbacks dispatched
async fn handle_broker_events(
    mut event_rx: mpsc::UnboundedReceiver<BrokerEvent>,
    broker: MqttBroker,
) {
    while let Some(event) = event_rx.recv().await {
        match event {
            BrokerEvent::Reconnected => {
                broker.inner.connected.store(true, Ordering::Release);
                tracing::info!("MQTT broker reconnected, restoring subscriptions");
                broker.handle_reconnection().await;
            }
            BrokerEvent::ConnectionLost => {
                let was_connected = broker.inner.connected.swap(false, Ordering::AcqRel);
                if was_connected {
                    tracing::warn!("MQTT connection lost, waiting for reconnection");
                    broker.dispatch_disconnected_all().await;
                }
            }
            BrokerEvent::Message { topic, payload } => {
                tracing::debug!(
                    topic = %topic,
                    payload = %payload,
                    "MQTT message received"
                );
                broker.route_message(&topic, payload).await;
            }
        }
    }
    tracing::info!("MQTT broker event loop ended");
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
        assert_eq!(creds.username(), "user");
        assert_eq!(creds.password(), "pass");
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
    fn builder_with_command_timeout() {
        let builder = MqttBrokerBuilder::default().command_timeout(Duration::from_secs(15));
        assert_eq!(builder.config.command_timeout, Duration::from_secs(15));
    }

    #[test]
    fn builder_default_command_timeout() {
        let builder = MqttBrokerBuilder::default();
        assert_eq!(builder.config.command_timeout, Duration::from_secs(5));
    }

    #[test]
    fn builder_chain() {
        let builder = MqttBrokerBuilder::default()
            .host("192.168.1.50")
            .port(8883)
            .credentials("admin", "secret")
            .keep_alive(Duration::from_secs(45))
            .connection_timeout(Duration::from_secs(15))
            .command_timeout(Duration::from_secs(10));

        assert_eq!(builder.config.host, "192.168.1.50");
        assert_eq!(builder.config.port, 8883);
        assert!(builder.config.credentials.is_some());
        assert_eq!(builder.config.keep_alive, Duration::from_secs(45));
        assert_eq!(builder.config.connection_timeout, Duration::from_secs(15));
        assert_eq!(builder.config.command_timeout, Duration::from_secs(10));
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
