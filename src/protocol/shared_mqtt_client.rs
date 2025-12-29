// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Shared MQTT client that uses a broker's connection.
//!
//! This client shares the MQTT connection from an [`MqttBroker`] instead of
//! creating its own connection. This is more efficient when managing multiple
//! devices on the same broker.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use rumqttc::QoS;
use tokio::sync::{Mutex, mpsc};

use crate::command::Command;
use crate::error::ProtocolError;
use crate::protocol::{CommandResponse, Protocol};
use crate::subscription::CallbackRegistry;

use super::mqtt_broker::MqttBroker;
use super::topic_router::TopicRouter;

/// MQTT client that shares a broker's connection.
///
/// This client uses the connection from an [`MqttBroker`](super::MqttBroker),
/// which is efficient when managing multiple devices on the same broker.
///
/// This client is created via [`MqttBroker::device()`](super::MqttBroker::device).
///
/// # Disconnection
///
/// When you're done with a device, call [`disconnect()`](Self::disconnect) to cleanly
/// unsubscribe from MQTT topics. If `disconnect()` is not called, the `Drop`
/// implementation will attempt a best-effort cleanup.
///
/// ```no_run
/// # async fn example() -> tasmor_lib::Result<()> {
/// use tasmor_lib::MqttBroker;
///
/// let broker = MqttBroker::builder().host("192.168.1.50").build().await?;
/// let (device, _) = broker.device("tasmota").build().await?;
///
/// device.power_on().await?;
/// device.disconnect().await;  // Clean shutdown
/// # Ok(())
/// # }
/// ```
pub struct SharedMqttClient {
    /// The shared MQTT async client for publishing.
    client: rumqttc::AsyncClient,
    /// The device topic (e.g., `tasmota_bulb`).
    topic: String,
    /// Channel for receiving command responses.
    response_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    /// Router for dispatching messages to callbacks.
    router: Arc<TopicRouter>,
    /// Reference to the broker for cleanup.
    broker: MqttBroker,
    /// Whether this client has been disconnected.
    disconnected: AtomicBool,
}

impl SharedMqttClient {
    /// Creates a new shared MQTT client.
    ///
    /// This is called internally by `MqttBroker` when creating a device.
    pub(crate) fn new(
        client: rumqttc::AsyncClient,
        topic: String,
        response_rx: mpsc::Receiver<String>,
        router: Arc<TopicRouter>,
        broker: MqttBroker,
    ) -> Self {
        Self {
            client,
            topic,
            response_rx: Arc::new(Mutex::new(response_rx)),
            router,
            broker,
            disconnected: AtomicBool::new(false),
        }
    }

    /// Returns the device topic.
    #[must_use]
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Disconnects and cleans up MQTT subscriptions.
    ///
    /// This only unsubscribes this device from its topics; the shared broker
    /// connection remains open for other devices.
    ///
    /// This method is idempotent - calling it multiple times is safe.
    pub async fn disconnect(&self) {
        if self.disconnected.swap(true, Ordering::SeqCst) {
            return; // Already disconnected
        }
        self.broker.remove_device_subscription(&self.topic).await;
        tracing::debug!(topic = %self.topic, "Device disconnected");
    }

    /// Returns whether this client has been disconnected.
    #[must_use]
    pub fn is_disconnected(&self) -> bool {
        self.disconnected.load(Ordering::SeqCst)
    }

    /// Registers a callback registry for receiving state updates.
    pub fn register_callbacks(&self, callbacks: &Arc<CallbackRegistry>) {
        self.router.register(&self.topic, callbacks);
    }

    /// Publishes a message to the command topic.
    async fn publish_command(&self, command: &str, payload: &str) -> Result<(), ProtocolError> {
        let topic = format!("cmnd/{}/{command}", self.topic);

        tracing::debug!(topic = %topic, payload = %payload, "Publishing shared MQTT command");

        self.client
            .publish(&topic, QoS::AtLeastOnce, false, payload)
            .await
            .map_err(ProtocolError::Mqtt)
    }

    /// Drains stale messages from the response channel.
    async fn drain_stale_responses(&self) {
        let mut rx = self.response_rx.lock().await;
        let mut count = 0;
        while rx.try_recv().is_ok() {
            count += 1;
        }
        if count > 0 {
            tracing::debug!(count, "Drained stale MQTT responses");
        }
    }

    /// Waits for a response with timeout.
    async fn wait_response(&self, timeout: Duration) -> Result<String, ProtocolError> {
        let mut rx = self.response_rx.lock().await;

        #[allow(clippy::cast_possible_truncation)]
        let timeout_ms = timeout.as_millis() as u64;

        tokio::time::timeout(timeout, rx.recv())
            .await
            .map_err(|_| ProtocolError::Timeout(timeout_ms))?
            .ok_or_else(|| ProtocolError::ConnectionFailed("Response channel closed".to_string()))
    }
}

impl Protocol for SharedMqttClient {
    async fn send_command<C: Command + Sync>(
        &self,
        command: &C,
    ) -> Result<CommandResponse, ProtocolError> {
        let cmd_name = command.mqtt_topic_suffix();
        let payload = command.mqtt_payload();

        self.drain_stale_responses().await;
        self.publish_command(&cmd_name, &payload).await?;

        let body = self.wait_response(Duration::from_secs(5)).await?;
        Ok(CommandResponse::new(body))
    }

    async fn send_raw(&self, command: &str) -> Result<CommandResponse, ProtocolError> {
        let parts: Vec<&str> = command.splitn(2, ' ').collect();
        let (cmd_name, payload) = match parts.as_slice() {
            [name] => (*name, ""),
            [name, payload] => (*name, *payload),
            _ => {
                return Err(ProtocolError::InvalidAddress(
                    "Invalid command format".to_string(),
                ));
            }
        };

        self.drain_stale_responses().await;
        self.publish_command(cmd_name, payload).await?;

        let body = self.wait_response(Duration::from_secs(5)).await?;
        Ok(CommandResponse::new(body))
    }
}

impl Drop for SharedMqttClient {
    fn drop(&mut self) {
        if self.disconnected.load(Ordering::SeqCst) {
            return; // Already disconnected via disconnect()
        }

        let topic = self.topic.clone();
        let broker = self.broker.clone();

        // Attempt async cleanup if we're in a tokio runtime
        if let Ok(handle) = tokio::runtime::Handle::try_current() {
            handle.spawn(async move {
                broker.remove_device_subscription(&topic).await;
                tracing::debug!(topic = %topic, "Device cleanup via Drop");
            });
        } else {
            tracing::warn!(
                topic = %self.topic,
                "No tokio runtime available for async cleanup in Drop"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_client_implements_protocol() {
        fn assert_protocol<T: Protocol>() {}
        assert_protocol::<SharedMqttClient>();
    }
}
