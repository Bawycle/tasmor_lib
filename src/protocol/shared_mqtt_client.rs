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
use std::time::Duration;

use rumqttc::QoS;
use tokio::sync::{Mutex, mpsc};

use crate::command::Command;
use crate::error::ProtocolError;
use crate::protocol::{CommandResponse, Protocol};
use crate::subscription::CallbackRegistry;

use super::topic_router::TopicRouter;

/// MQTT client that shares a broker's connection.
///
/// Unlike [`MqttClient`](super::MqttClient), this client does not create its own
/// MQTT connection. Instead, it uses the connection from an [`MqttBroker`](super::MqttBroker),
/// which is more efficient when managing multiple devices.
///
/// This client is created via [`MqttBroker::device()`](super::MqttBroker::device).
#[derive(Debug)]
pub struct SharedMqttClient {
    /// The shared MQTT async client for publishing.
    client: rumqttc::AsyncClient,
    /// The device topic (e.g., `tasmota_bulb`).
    topic: String,
    /// Channel for receiving command responses.
    response_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    /// Router for dispatching messages to callbacks.
    router: Arc<TopicRouter>,
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
    ) -> Self {
        Self {
            client,
            topic,
            response_rx: Arc::new(Mutex::new(response_rx)),
            router,
        }
    }

    /// Returns the device topic.
    #[must_use]
    pub fn topic(&self) -> &str {
        &self.topic
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_client_implements_protocol() {
        fn assert_protocol<T: Protocol>() {}
        assert_protocol::<SharedMqttClient>();
    }
}
