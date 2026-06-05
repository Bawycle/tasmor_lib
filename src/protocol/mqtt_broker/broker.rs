// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use paho_mqtt::{AsyncClient, QoS};
use tokio::sync::{RwLock, mpsc};

use crate::error::ProtocolError;
use crate::protocol::TopicRouter;
use crate::protocol::response_collector::MqttMessage;

use super::builder::MqttBrokerBuilder;
use super::config::MqttBrokerConfig;

/// A subscription to a device topic on the broker.
pub(crate) struct DeviceSubscription {
    /// Channel to send command responses (RESULT, STATUS*) to the device.
    pub response_tx: mpsc::Sender<MqttMessage>,
    /// Router for dispatching messages to callbacks.
    pub router: Arc<TopicRouter>,
}

pub(super) struct MqttBrokerInner {
    /// The paho-mqtt async client for publishing and subscribing.
    pub(super) client: AsyncClient,
    /// Active device subscriptions by device topic.
    pub(super) subscriptions: RwLock<HashMap<String, DeviceSubscription>>,
    /// Configuration used for this connection.
    pub(super) config: MqttBrokerConfig,
    /// Connection status.
    pub(super) connected: AtomicBool,
    /// Channel for sending discovered device topics during discovery.
    pub(super) discovery_tx: RwLock<Option<mpsc::Sender<String>>>,
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
    pub(super) inner: Arc<MqttBrokerInner>,
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
            .map_err(ProtocolError::from)?;

        // Subscribe to tele/<topic>/+ for telemetry
        let tele_topic = format!("tele/{device_topic}/+");
        self.inner
            .client
            .subscribe(&tele_topic, QoS::AtLeastOnce)
            .await
            .map_err(ProtocolError::from)?;

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
    pub(super) async fn route_message(&self, topic: &str, payload: String) {
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
    pub(super) async fn handle_reconnection(&self) {
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
    pub(super) async fn dispatch_disconnected_all(&self) {
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
            .map_err(ProtocolError::from)?;

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

    /// Sets the connected flag.
    pub(super) fn set_connected(&self, val: bool) {
        self.inner.connected.store(val, Ordering::Release);
    }

    /// Atomically swaps the connected flag, returning the previous value.
    pub(super) fn swap_connected(&self, val: bool) -> bool {
        self.inner.connected.swap(val, Ordering::AcqRel)
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
