// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! MQTT broker connection pooling.
//!
//! This module provides connection pooling for MQTT brokers, allowing multiple
//! devices on the same broker to share a single MQTT connection.

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Weak};
use std::time::Duration;

use rumqttc::{AsyncClient, EventLoop, MqttOptions, QoS};
use tokio::sync::{RwLock, mpsc};

use crate::error::ProtocolError;

/// Global counter for generating unique client IDs.
static POOL_CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Key identifying a unique broker connection.
///
/// Connections are uniquely identified by host, port, and credentials.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct BrokerKey {
    host: String,
    port: u16,
    username: Option<String>,
}

impl BrokerKey {
    /// Creates a broker key from URL and optional credentials.
    pub fn new(broker_url: &str, credentials: Option<(&str, &str)>) -> Result<Self, ProtocolError> {
        let (host, port) = parse_broker_url(broker_url)?;
        Ok(Self {
            host,
            port,
            username: credentials.map(|(u, _)| u.to_string()),
        })
    }
}

/// A subscription to a device topic on a shared connection.
pub(crate) struct TopicSubscription {
    /// Channel to send responses to this subscriber.
    pub response_tx: mpsc::Sender<String>,
}

/// A shared MQTT connection that can serve multiple device topics.
pub struct SharedConnection {
    /// The MQTT async client.
    client: AsyncClient,
    /// Active subscriptions by device topic.
    subscriptions: RwLock<HashMap<String, TopicSubscription>>,
    /// The broker key for this connection.
    broker_key: BrokerKey,
}

impl SharedConnection {
    /// Returns the MQTT client for publishing.
    pub(crate) fn client(&self) -> &AsyncClient {
        &self.client
    }

    /// Adds a subscription for a device topic.
    pub(crate) async fn add_subscription(
        &self,
        device_topic: String,
        response_tx: mpsc::Sender<String>,
    ) -> Result<(), ProtocolError> {
        // Subscribe to stat/<topic>/+ for responses
        let stat_topic = format!("stat/{device_topic}/+");
        self.client
            .subscribe(&stat_topic, QoS::AtLeastOnce)
            .await
            .map_err(ProtocolError::Mqtt)?;

        // Register the subscription
        let subscription = TopicSubscription { response_tx };
        self.subscriptions
            .write()
            .await
            .insert(device_topic, subscription);

        Ok(())
    }

    /// Removes a subscription for a device topic.
    pub(crate) async fn remove_subscription(&self, device_topic: &str) {
        self.subscriptions.write().await.remove(device_topic);
        // Note: We don't unsubscribe from MQTT as other code might still be interested
        // and it doesn't hurt to receive messages we ignore
    }

    /// Routes an incoming message to the appropriate subscriber.
    pub(crate) async fn route_message(&self, topic: &str, payload: String) {
        // Parse the topic to extract device topic
        // Format: stat/<device_topic>/RESULT
        let parts: Vec<&str> = topic.split('/').collect();
        if parts.len() >= 3 && parts[0] == "stat" && parts[2] == "RESULT" {
            let device_topic = parts[1];

            let subscriptions = self.subscriptions.read().await;
            if let Some(sub) = subscriptions.get(device_topic) {
                let _ = sub.response_tx.send(payload).await;
            }
        }
    }

    /// Returns the number of active subscriptions.
    pub async fn subscription_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }
}

impl std::fmt::Debug for SharedConnection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SharedConnection")
            .field("broker_key", &self.broker_key)
            .finish_non_exhaustive()
    }
}

/// Global pool of MQTT broker connections.
///
/// The pool maintains at most one connection per unique broker (identified by
/// host, port, and credentials). Connections are reference-counted and
/// automatically cleaned up when no longer in use.
///
/// # Examples
///
/// ```no_run
/// use tasmor_lib::protocol::BrokerPool;
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// let pool = BrokerPool::global();
///
/// // Get or create a connection to a broker
/// let conn = pool.get_connection("mqtt://192.168.1.50:1883", None).await?;
///
/// // Multiple calls with the same broker URL return the same connection
/// let conn2 = pool.get_connection("mqtt://192.168.1.50:1883", None).await?;
/// # Ok(())
/// # }
/// ```
pub struct BrokerPool {
    /// Active connections, keyed by broker key.
    /// Uses Weak references for automatic cleanup when all subscribers are gone.
    connections: RwLock<HashMap<BrokerKey, Weak<SharedConnection>>>,
}

impl BrokerPool {
    /// Returns the global broker pool instance.
    ///
    /// This is a singleton that should be used for all connection pooling.
    pub fn global() -> &'static Self {
        use std::sync::OnceLock;
        static POOL: OnceLock<BrokerPool> = OnceLock::new();
        POOL.get_or_init(Self::new)
    }

    /// Creates a new broker pool.
    ///
    /// Prefer using [`BrokerPool::global()`] for most use cases.
    #[must_use]
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }

    /// Gets or creates a connection to the specified broker.
    ///
    /// If a connection already exists for this broker, it is reused.
    /// Otherwise, a new connection is created.
    ///
    /// # Arguments
    ///
    /// * `broker_url` - The MQTT broker URL (e.g., `mqtt://192.168.1.50:1883`)
    /// * `credentials` - Optional (username, password) tuple for authentication
    ///
    /// # Errors
    ///
    /// Returns error if the broker URL is invalid or connection fails.
    pub async fn get_connection(
        &self,
        broker_url: &str,
        credentials: Option<(&str, &str)>,
    ) -> Result<Arc<SharedConnection>, ProtocolError> {
        let key = BrokerKey::new(broker_url, credentials)?;

        // Check for existing connection
        {
            let connections = self.connections.read().await;
            if let Some(weak) = connections.get(&key)
                && let Some(conn) = weak.upgrade()
            {
                tracing::debug!(?key, "Reusing existing broker connection");
                return Ok(conn);
            }
        }

        // Create new connection
        tracing::debug!(?key, "Creating new broker connection");
        let conn = self.create_connection(&key, credentials).await?;
        let arc = Arc::new(conn);

        // Store weak reference
        {
            let mut connections = self.connections.write().await;
            connections.insert(key, Arc::downgrade(&arc));
        }

        Ok(arc)
    }

    /// Creates a new connection to a broker.
    async fn create_connection(
        &self,
        key: &BrokerKey,
        credentials: Option<(&str, &str)>,
    ) -> Result<SharedConnection, ProtocolError> {
        // Generate unique client ID
        let counter = POOL_CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let client_id = format!("tasmor_pool_{}_{}", std::process::id(), counter);

        let mut mqtt_options = MqttOptions::new(&client_id, &key.host, key.port);
        mqtt_options.set_keep_alive(Duration::from_secs(30));
        mqtt_options.set_clean_session(true);

        // Set credentials if provided
        if let Some((username, password)) = credentials {
            mqtt_options.set_credentials(username, password);
        }

        let (client, event_loop) = AsyncClient::new(mqtt_options, 10);

        let conn = SharedConnection {
            client,
            subscriptions: RwLock::new(HashMap::new()),
            broker_key: key.clone(),
        };

        // Spawn event loop handler
        let conn_arc_for_task = Arc::new(conn);
        let conn_weak = Arc::downgrade(&conn_arc_for_task);

        tokio::spawn(async move {
            handle_pooled_mqtt_events(event_loop, conn_weak).await;
        });

        // Wait for connection to be established
        tokio::time::sleep(Duration::from_millis(500)).await;

        // Extract the inner value - this is safe because we just created it
        // and the only other reference is the weak one in the spawned task
        Ok(Arc::try_unwrap(conn_arc_for_task).unwrap_or_else(|arc| {
            // If we can't unwrap, clone the inner value
            SharedConnection {
                client: arc.client.clone(),
                subscriptions: RwLock::new(HashMap::new()),
                broker_key: arc.broker_key.clone(),
            }
        }))
    }

    /// Removes stale connections from the pool.
    ///
    /// This is called automatically, but can be invoked manually to clean up.
    pub async fn cleanup(&self) {
        let mut connections = self.connections.write().await;
        connections.retain(|_, weak| weak.strong_count() > 0);
    }

    /// Returns the number of active connections in the pool.
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections
            .values()
            .filter(|w| w.strong_count() > 0)
            .count()
    }
}

impl Default for BrokerPool {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for BrokerPool {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BrokerPool").finish()
    }
}

/// Handles MQTT events for a pooled connection.
async fn handle_pooled_mqtt_events(mut event_loop: EventLoop, conn: Weak<SharedConnection>) {
    use rumqttc::{Event, Packet};

    loop {
        match event_loop.poll().await {
            Ok(Event::Incoming(Packet::ConnAck(connack))) => {
                tracing::debug!(?connack, "Pooled MQTT connected");
            }
            Ok(Event::Incoming(Packet::SubAck(suback))) => {
                tracing::debug!(?suback, "Pooled MQTT subscription acknowledged");
            }
            Ok(Event::Incoming(Packet::Publish(publish))) => {
                // Route to appropriate subscriber
                if let Some(conn) = conn.upgrade() {
                    if let Ok(payload) = String::from_utf8(publish.payload.to_vec()) {
                        tracing::debug!(
                            topic = %publish.topic,
                            payload = %payload,
                            "Pooled MQTT received message"
                        );
                        conn.route_message(&publish.topic, payload).await;
                    }
                } else {
                    // Connection dropped, exit loop
                    break;
                }
            }
            Ok(_) => {}
            Err(e) => {
                tracing::error!(error = %e, "Pooled MQTT event loop error");
                break;
            }
        }
    }
}

/// Parses a broker URL into host and port.
fn parse_broker_url(url: &str) -> Result<(String, u16), ProtocolError> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn broker_key_equality() {
        let key1 = BrokerKey::new("mqtt://localhost:1883", None).unwrap();
        let key2 = BrokerKey::new("mqtt://localhost:1883", None).unwrap();
        assert_eq!(key1, key2);
    }

    #[test]
    fn broker_key_with_credentials() {
        let key1 = BrokerKey::new("mqtt://localhost:1883", Some(("user", "pass"))).unwrap();
        let key2 = BrokerKey::new("mqtt://localhost:1883", None).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn broker_key_different_ports() {
        let key1 = BrokerKey::new("mqtt://localhost:1883", None).unwrap();
        let key2 = BrokerKey::new("mqtt://localhost:1884", None).unwrap();
        assert_ne!(key1, key2);
    }

    #[test]
    fn parse_broker_url_with_scheme() {
        let (host, port) = parse_broker_url("mqtt://192.168.1.50:1883").unwrap();
        assert_eq!(host, "192.168.1.50");
        assert_eq!(port, 1883);
    }

    #[test]
    fn parse_broker_url_tcp_scheme() {
        let (host, port) = parse_broker_url("tcp://broker.local:8883").unwrap();
        assert_eq!(host, "broker.local");
        assert_eq!(port, 8883);
    }

    #[test]
    fn parse_broker_url_no_scheme() {
        let (host, port) = parse_broker_url("192.168.1.50:1883").unwrap();
        assert_eq!(host, "192.168.1.50");
        assert_eq!(port, 1883);
    }

    #[test]
    fn parse_broker_url_default_port() {
        let (host, port) = parse_broker_url("localhost").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 1883);
    }

    #[test]
    fn pool_is_singleton() {
        let pool1 = BrokerPool::global();
        let pool2 = BrokerPool::global();
        assert!(std::ptr::eq(pool1, pool2));
    }
}
