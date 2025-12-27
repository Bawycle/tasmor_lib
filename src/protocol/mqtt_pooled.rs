// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Pooled MQTT client that shares broker connections.

use std::sync::Arc;
use std::time::Duration;

use rumqttc::QoS;
use tokio::sync::{Mutex, mpsc};

use crate::command::Command;
use crate::error::ProtocolError;
use crate::protocol::{CommandResponse, Protocol};

use super::broker_pool::{BrokerPool, SharedConnection};

/// MQTT client that uses connection pooling.
///
/// Unlike [`MqttClient`](super::MqttClient), this client shares the underlying
/// MQTT connection with other devices on the same broker. This is more efficient
/// when managing multiple Tasmota devices on the same MQTT broker.
///
/// # Examples
///
/// ```no_run
/// use tasmor_lib::protocol::PooledMqttClient;
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// // Create multiple clients that share the same connection
/// let client1 = PooledMqttClient::connect("mqtt://broker:1883", "device1", None).await?;
/// let client2 = PooledMqttClient::connect("mqtt://broker:1883", "device2", None).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct PooledMqttClient {
    /// The shared connection.
    connection: Arc<SharedConnection>,
    /// The device topic (e.g., `tasmota_bulb`).
    topic: String,
    /// Channel for receiving responses (can be taken for async processing).
    response_rx: Arc<Mutex<Option<mpsc::Receiver<String>>>>,
}

impl PooledMqttClient {
    /// Connects to an MQTT broker using connection pooling.
    ///
    /// If a connection to this broker already exists, it will be reused.
    ///
    /// # Arguments
    ///
    /// * `broker_url` - The MQTT broker URL (e.g., `mqtt://192.168.1.50:1883`)
    /// * `device_topic` - The Tasmota device topic (e.g., `tasmota_switch`)
    /// * `credentials` - Optional (username, password) for broker authentication
    ///
    /// # Errors
    ///
    /// Returns error if connection fails.
    pub async fn connect(
        broker_url: impl Into<String>,
        device_topic: impl Into<String>,
        credentials: Option<(&str, &str)>,
    ) -> Result<Self, ProtocolError> {
        let broker_url = broker_url.into();
        let device_topic = device_topic.into();

        // Get or create pooled connection
        let pool = BrokerPool::global();
        let connection = pool.get_connection(&broker_url, credentials).await?;

        // Create response channel
        let (response_tx, response_rx) = mpsc::channel::<String>(10);

        // Add subscription for this device
        connection
            .add_subscription(device_topic.clone(), response_tx)
            .await?;

        Ok(Self {
            connection,
            topic: device_topic,
            response_rx: Arc::new(Mutex::new(Some(response_rx))),
        })
    }

    /// Takes ownership of the message receiver for async processing.
    ///
    /// This allows a background task to consume all incoming messages
    /// (both command responses and telemetry). After calling this method,
    /// `send_command` will no longer wait for responses.
    ///
    /// # Returns
    ///
    /// Returns the receiver if it hasn't been taken yet, `None` otherwise.
    pub async fn take_message_receiver(&self) -> Option<mpsc::Receiver<String>> {
        self.response_rx.lock().await.take()
    }

    /// Returns the device topic.
    #[must_use]
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Publishes a message to the command topic.
    async fn publish_command(&self, command: &str, payload: &str) -> Result<(), ProtocolError> {
        let topic = format!("cmnd/{}/{command}", self.topic);

        tracing::debug!(topic = %topic, payload = %payload, "Publishing pooled MQTT command");

        self.connection
            .client()
            .publish(&topic, QoS::AtLeastOnce, false, payload)
            .await
            .map_err(ProtocolError::Mqtt)
    }

    /// Waits for a response with timeout.
    ///
    /// If the message receiver has been taken via `take_message_receiver`,
    /// this will return an error immediately since responses are handled
    /// asynchronously by the consumer.
    async fn wait_response(&self, timeout: Duration) -> Result<String, ProtocolError> {
        let mut guard = self.response_rx.lock().await;

        let rx = guard.as_mut().ok_or_else(|| {
            ProtocolError::ChannelClosed(
                "Message receiver taken - responses handled asynchronously".to_string(),
            )
        })?;

        // Safe: timeout in practical use will never exceed u64::MAX milliseconds
        #[allow(clippy::cast_possible_truncation)]
        let timeout_ms = timeout.as_millis() as u64;

        tokio::time::timeout(timeout, rx.recv())
            .await
            .map_err(|_| ProtocolError::Timeout(timeout_ms))?
            .ok_or_else(|| ProtocolError::ConnectionFailed("Response channel closed".to_string()))
    }
}

impl Drop for PooledMqttClient {
    fn drop(&mut self) {
        // Remove subscription when client is dropped
        let topic = self.topic.clone();
        let connection = self.connection.clone();

        // Spawn a task to remove the subscription
        // This is safe because we're just cleaning up
        tokio::spawn(async move {
            connection.remove_subscription(&topic).await;
        });
    }
}

impl Protocol for PooledMqttClient {
    async fn send_command<C: Command + Sync>(
        &self,
        command: &C,
    ) -> Result<CommandResponse, ProtocolError> {
        let cmd_name = command.mqtt_topic_suffix();
        let payload = command.mqtt_payload();

        self.publish_command(&cmd_name, &payload).await?;

        // Try to wait for response, but if receiver is taken, return empty response
        // The actual response will be handled by the message consumer task
        match self.wait_response(Duration::from_secs(5)).await {
            Ok(body) => Ok(CommandResponse::new(body)),
            Err(ProtocolError::ChannelClosed(_)) => {
                // Receiver taken - response will be handled asynchronously
                tracing::debug!(command = %cmd_name, "Command sent, response handled async");
                Ok(CommandResponse::new("{}".to_string()))
            }
            Err(e) => Err(e),
        }
    }

    async fn send_raw(&self, command: &str) -> Result<CommandResponse, ProtocolError> {
        // Parse raw command into name and payload
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

        self.publish_command(cmd_name, payload).await?;

        // Try to wait for response, but if receiver is taken, return empty response
        match self.wait_response(Duration::from_secs(5)).await {
            Ok(body) => Ok(CommandResponse::new(body)),
            Err(ProtocolError::ChannelClosed(_)) => {
                tracing::debug!(command = %cmd_name, "Raw command sent, response handled async");
                Ok(CommandResponse::new("{}".to_string()))
            }
            Err(e) => Err(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Most tests require a real MQTT broker, so they're in integration tests.
    // Here we just test basic construction and trait implementation.

    #[test]
    fn pooled_client_implements_protocol() {
        fn assert_protocol<T: Protocol>() {}
        assert_protocol::<PooledMqttClient>();
    }
}
