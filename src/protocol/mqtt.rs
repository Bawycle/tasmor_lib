// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! MQTT protocol implementation for Tasmota devices.

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Global counter for generating unique client IDs.
static CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use tokio::sync::{Mutex, mpsc};

use crate::command::Command;
use crate::error::ProtocolError;
use crate::protocol::{CommandResponse, Protocol, TopicRouter};
use crate::subscription::CallbackRegistry;

/// MQTT client for communicating with Tasmota devices.
///
/// Uses the Tasmota MQTT topic structure:
/// - Commands: `cmnd/<topic>/<command>`
/// - Responses: `stat/<topic>/RESULT` or `stat/<topic>/<COMMAND>`
/// - Telemetry: `tele/<topic>/<data>`
///
/// # Examples
///
/// ```no_run
/// use tasmor_lib::protocol::{MqttClient, Protocol};
/// use tasmor_lib::command::PowerCommand;
/// use tasmor_lib::types::PowerIndex;
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// let client = MqttClient::connect("mqtt://broker:1883", "tasmota_switch").await?;
/// let response = client.send_command(&PowerCommand::query(PowerIndex::one())).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct MqttClient {
    client: AsyncClient,
    topic: String,
    response_rx: Arc<Mutex<mpsc::Receiver<String>>>,
    router: Arc<TopicRouter>,
}

impl MqttClient {
    /// Connects to an MQTT broker for a specific Tasmota device topic.
    ///
    /// # Arguments
    ///
    /// * `broker_url` - The MQTT broker URL (e.g., `mqtt://192.168.1.50:1883`)
    /// * `device_topic` - The Tasmota device topic (e.g., `tasmota_switch`)
    ///
    /// # Errors
    ///
    /// Returns error if connection fails.
    pub async fn connect(
        broker_url: impl Into<String>,
        device_topic: impl Into<String>,
    ) -> Result<Self, ProtocolError> {
        let broker_url = broker_url.into();
        let device_topic = device_topic.into();

        // Parse broker URL
        let (host, port) = parse_mqtt_url(&broker_url)?;

        // Generate a unique client ID (PID + counter to avoid conflicts)
        let counter = CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let client_id = format!("tasmor_{}_{}", std::process::id(), counter);

        let mut mqtt_options = MqttOptions::new(&client_id, host, port);
        mqtt_options.set_keep_alive(Duration::from_secs(30));
        mqtt_options.set_clean_session(true);

        let (client, event_loop) = AsyncClient::new(mqtt_options, 10);

        // Create channel for receiving responses
        let (response_tx, response_rx) = mpsc::channel::<String>(10);

        // Create router for dispatching messages to callbacks
        let router = Arc::new(TopicRouter::new());

        // Subscribe to response and telemetry topics
        let stat_topic = format!("stat/{device_topic}/+");
        let tele_topic = format!("tele/{device_topic}/+");
        client
            .subscribe(&stat_topic, QoS::AtLeastOnce)
            .await
            .map_err(ProtocolError::Mqtt)?;
        client
            .subscribe(&tele_topic, QoS::AtLeastOnce)
            .await
            .map_err(ProtocolError::Mqtt)?;

        // Spawn event loop handler with router
        let topic_clone = device_topic.clone();
        let router_clone = Arc::clone(&router);
        tokio::spawn(async move {
            handle_mqtt_events(event_loop, topic_clone, response_tx, router_clone).await;
        });

        // Give time for connection establishment and subscription acknowledgment
        // This delay ensures the broker has processed our CONNECT and SUBSCRIBE
        // before we start sending commands. 500ms is conservative but necessary
        // for reliable operation with real brokers.
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(Self {
            client,
            topic: device_topic,
            response_rx: Arc::new(Mutex::new(response_rx)),
            router,
        })
    }

    /// Returns the device topic.
    #[must_use]
    pub fn topic(&self) -> &str {
        &self.topic
    }

    /// Registers a callback registry for receiving state updates.
    ///
    /// This should be called after creating the device to enable
    /// callback dispatch for incoming MQTT messages.
    pub fn register_callbacks(&self, callbacks: &Arc<CallbackRegistry>) {
        self.router.register(&self.topic, callbacks);
    }

    /// Publishes a message to the command topic.
    async fn publish_command(&self, command: &str, payload: &str) -> Result<(), ProtocolError> {
        let self_topic = &self.topic;
        let topic = format!("cmnd/{self_topic}/{command}");

        tracing::debug!(topic = %topic, payload = %payload, "Publishing MQTT command");

        self.client
            .publish(&topic, QoS::AtLeastOnce, false, payload)
            .await
            .map_err(ProtocolError::Mqtt)
    }

    /// Drains stale messages from the response channel.
    ///
    /// This is necessary because Tasmota may send multiple RESULT messages
    /// during command execution (especially for Backlog commands with delays).
    /// Without draining, subsequent commands might receive stale responses.
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

        // Safe: timeout in practical use will never exceed u64::MAX milliseconds
        #[allow(clippy::cast_possible_truncation)]
        let timeout_ms = timeout.as_millis() as u64;

        tokio::time::timeout(timeout, rx.recv())
            .await
            .map_err(|_| ProtocolError::Timeout(timeout_ms))?
            .ok_or_else(|| ProtocolError::ConnectionFailed("Response channel closed".to_string()))
    }
}

impl Protocol for MqttClient {
    async fn send_command<C: Command + Sync>(
        &self,
        command: &C,
    ) -> Result<CommandResponse, ProtocolError> {
        let cmd_name = command.mqtt_topic_suffix();
        let payload = command.mqtt_payload();

        // Drain any stale responses before sending new command
        self.drain_stale_responses().await;

        self.publish_command(&cmd_name, &payload).await?;

        // Wait for response
        let body = self.wait_response(Duration::from_secs(5)).await?;

        Ok(CommandResponse::new(body))
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

        // Drain any stale responses before sending new command
        self.drain_stale_responses().await;

        self.publish_command(cmd_name, payload).await?;

        let body = self.wait_response(Duration::from_secs(5)).await?;

        Ok(CommandResponse::new(body))
    }
}

/// Parses an MQTT URL into host and port.
fn parse_mqtt_url(url: &str) -> Result<(String, u16), ProtocolError> {
    let url = url
        .strip_prefix("mqtt://")
        .or_else(|| url.strip_prefix("tcp://"))
        .unwrap_or(url);

    let (host, port) = if let Some((h, p)) = url.rsplit_once(':') {
        let port = p
            .parse()
            .map_err(|_| ProtocolError::InvalidAddress(format!("Invalid port: {p}")))?;
        (h.to_string(), port)
    } else {
        (url.to_string(), 1883)
    };

    Ok((host, port))
}

/// Handles MQTT events in the background.
async fn handle_mqtt_events(
    mut event_loop: EventLoop,
    topic: String,
    response_tx: mpsc::Sender<String>,
    router: Arc<TopicRouter>,
) {
    use rumqttc::{Event, Packet};

    loop {
        match event_loop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(connack))) => {
                tracing::debug!(?connack, "MQTT connected");
            }
            Ok(Event::Incoming(Packet::SubAck(suback))) => {
                tracing::debug!(?suback, "MQTT subscription acknowledged");
            }
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                let Ok(payload) = String::from_utf8(publish.payload.to_vec()) else {
                    continue;
                };

                // Route the message to callbacks
                router.route(&publish.topic, &payload);

                // Also handle command responses for the response channel
                // Tasmota sends responses on:
                // - stat/<topic>/RESULT (JSON) for most commands
                // - stat/<topic>/STATUS<n> (JSON) for Status commands
                // - stat/<topic>/POWER[n] (plain text) for power commands
                //
                // NOTE: We intentionally do NOT send POWER responses to the response channel.
                // POWER responses arrive asynchronously and can interfere with other commands
                // (e.g., when waiting for STATUS10, a POWER response might arrive first).
                // POWER state changes are handled via the topic router callbacks instead.
                let stat_prefix = format!("stat/{topic}/");
                if publish.topic.starts_with(&stat_prefix) {
                    let suffix = &publish.topic[stat_prefix.len()..];

                    // Check for JSON responses (RESULT or STATUS*)
                    let is_json_response = suffix == "RESULT" || suffix.starts_with("STATUS");
                    if is_json_response {
                        tracing::debug!(
                            topic = %publish.topic,
                            payload = %payload,
                            "Received MQTT response"
                        );
                        let _ = response_tx.send(payload).await;
                    }
                    // Log POWER responses but don't send to response channel
                    // (they are already routed to callbacks via the topic router)
                    else if suffix == "POWER" || suffix.starts_with("POWER") {
                        tracing::debug!(
                            topic = %publish.topic,
                            payload = %payload,
                            "Received MQTT power response (handled via callbacks)"
                        );
                    }
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(error = %e, "MQTT event loop error");
                break;
            }
        }
    }
}

/// Builder for creating an MQTT client with custom configuration.
#[derive(Debug, Default)]
pub struct MqttClientBuilder {
    broker: Option<String>,
    device_topic: Option<String>,
    username: Option<String>,
    password: Option<String>,
    client_id: Option<String>,
    keep_alive: Option<Duration>,
}

impl MqttClientBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the MQTT broker URL.
    #[must_use]
    pub fn broker(mut self, broker: impl Into<String>) -> Self {
        self.broker = Some(broker.into());
        self
    }

    /// Sets the Tasmota device topic.
    #[must_use]
    pub fn device_topic(mut self, topic: impl Into<String>) -> Self {
        self.device_topic = Some(topic.into());
        self
    }

    /// Sets authentication credentials for the MQTT broker.
    ///
    /// # Arguments
    ///
    /// * `username` - MQTT broker username
    /// * `password` - MQTT broker password
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tasmor_lib::protocol::MqttClientBuilder;
    ///
    /// # async fn example() -> tasmor_lib::Result<()> {
    /// let client = MqttClientBuilder::new()
    ///     .broker("mqtt://192.168.1.50:1883")
    ///     .device_topic("tasmota_switch")
    ///     .credentials("mqtt_user", "mqtt_password")
    ///     .build()
    ///     .await?;
    /// # Ok(())
    /// # }
    /// ```
    #[must_use]
    pub fn credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// Sets a custom client ID.
    #[must_use]
    pub fn client_id(mut self, id: impl Into<String>) -> Self {
        self.client_id = Some(id.into());
        self
    }

    /// Sets the keep-alive interval.
    #[must_use]
    pub fn keep_alive(mut self, duration: Duration) -> Self {
        self.keep_alive = Some(duration);
        self
    }

    /// Builds and connects the MQTT client.
    ///
    /// # Errors
    ///
    /// Returns error if required fields are missing or connection fails.
    pub async fn build(self) -> Result<MqttClient, ProtocolError> {
        let broker = self
            .broker
            .ok_or_else(|| ProtocolError::InvalidAddress("broker is required".to_string()))?;

        let device_topic = self
            .device_topic
            .ok_or_else(|| ProtocolError::InvalidAddress("device_topic is required".to_string()))?;

        // Parse broker URL
        let (host, port) = parse_mqtt_url(&broker)?;

        // Generate or use provided client ID (PID + counter to avoid conflicts)
        let client_id = self.client_id.unwrap_or_else(|| {
            let counter = CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
            format!("tasmor_{}_{}", std::process::id(), counter)
        });

        let mut mqtt_options = MqttOptions::new(&client_id, host, port);
        mqtt_options.set_keep_alive(self.keep_alive.unwrap_or(Duration::from_secs(30)));
        mqtt_options.set_clean_session(true);

        // Set credentials if provided
        if let (Some(username), Some(password)) = (self.username, self.password) {
            mqtt_options.set_credentials(username, password);
        }

        let (client, event_loop) = AsyncClient::new(mqtt_options, 10);

        // Create channel for receiving responses
        let (response_tx, response_rx) = mpsc::channel::<String>(10);

        // Create router for dispatching messages to callbacks
        let router = Arc::new(TopicRouter::new());

        // Subscribe to response and telemetry topics
        let stat_topic = format!("stat/{device_topic}/+");
        let tele_topic = format!("tele/{device_topic}/+");
        client
            .subscribe(&stat_topic, QoS::AtLeastOnce)
            .await
            .map_err(ProtocolError::Mqtt)?;
        client
            .subscribe(&tele_topic, QoS::AtLeastOnce)
            .await
            .map_err(ProtocolError::Mqtt)?;

        // Spawn event loop handler with router
        let topic_clone = device_topic.clone();
        let router_clone = Arc::clone(&router);
        tokio::spawn(async move {
            handle_mqtt_events(event_loop, topic_clone, response_tx, router_clone).await;
        });

        // Give time for connection establishment and subscription acknowledgment
        // This delay ensures the broker has processed our CONNECT and SUBSCRIBE
        // before we start sending commands. 500ms is conservative but necessary
        // for reliable operation with real brokers.
        tokio::time::sleep(Duration::from_millis(500)).await;

        Ok(MqttClient {
            client,
            topic: device_topic,
            response_rx: Arc::new(Mutex::new(response_rx)),
            router,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_mqtt_url_with_port() {
        let (host, port) = parse_mqtt_url("mqtt://192.168.1.50:1883").unwrap();
        assert_eq!(host, "192.168.1.50");
        assert_eq!(port, 1883);
    }

    #[test]
    fn parse_mqtt_url_default_port() {
        let (host, port) = parse_mqtt_url("192.168.1.50").unwrap();
        assert_eq!(host, "192.168.1.50");
        assert_eq!(port, 1883);
    }

    #[test]
    fn parse_mqtt_url_tcp_scheme() {
        let (host, port) = parse_mqtt_url("tcp://broker.local:8883").unwrap();
        assert_eq!(host, "broker.local");
        assert_eq!(port, 8883);
    }

    #[test]
    fn mqtt_client_builder_with_credentials() {
        let builder = MqttClientBuilder::new()
            .broker("mqtt://broker:1883")
            .device_topic("tasmota_switch")
            .credentials("user", "pass")
            .client_id("my_client")
            .keep_alive(Duration::from_secs(60));

        assert_eq!(builder.broker, Some("mqtt://broker:1883".to_string()));
        assert_eq!(builder.device_topic, Some("tasmota_switch".to_string()));
        assert_eq!(builder.username, Some("user".to_string()));
        assert_eq!(builder.password, Some("pass".to_string()));
        assert_eq!(builder.client_id, Some("my_client".to_string()));
        assert_eq!(builder.keep_alive, Some(Duration::from_secs(60)));
    }
}
