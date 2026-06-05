// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Duration;

use tokio::sync::{RwLock, mpsc};

use crate::credentials::Credentials;
use crate::error::ProtocolError;

use super::broker::{MqttBroker, MqttBrokerInner};
use super::config::{MqttBrokerConfig, TlsConfig};
use super::events::{BrokerEvent, handle_broker_events};

/// Global counter for generating unique client IDs.
static BROKER_CLIENT_ID_COUNTER: AtomicU64 = AtomicU64::new(0);

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
    pub(super) config: MqttBrokerConfig,
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

    /// Enables TLS for the broker connection, verifying it against the CA certificate
    /// at `ca_cert_pem_path` (PEM format).
    ///
    /// When TLS is enabled, the port is automatically set to 8883 if it was still at the
    /// plain-text default (1883). Call `.port()` after `tls_ca_cert()` to override.
    ///
    /// Server certificate verification is always enforced; there is intentionally no
    /// insecure mode.
    ///
    /// # Errors at `build()` time
    ///
    /// `build()` performs a brief access check on the certificate file and returns
    /// [`ProtocolError::Tls`] if the file cannot be opened — for example because it does
    /// not exist or because of a permission error.
    #[must_use]
    pub fn tls_ca_cert(mut self, ca_cert_pem_path: impl Into<PathBuf>) -> Self {
        self.config.tls = TlsConfig::Enabled {
            ca_cert_path: ca_cert_pem_path.into(),
        };
        if self.config.port == 1883 {
            self.config.port = 8883;
        }
        self
    }

    /// Builds and connects to the MQTT broker.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Host is not set
    /// - TLS is enabled and the CA certificate file cannot be accessed
    /// - Connection fails or times out
    pub async fn build(self) -> Result<MqttBroker, ProtocolError> {
        if self.config.host.is_empty() {
            return Err(ProtocolError::InvalidAddress(
                "MQTT broker host is required".to_string(),
            ));
        }

        if let TlsConfig::Enabled { ca_cert_path } = &self.config.tls
            && let Err(e) = std::fs::File::open(ca_cert_path)
        {
            let msg = match e.kind() {
                std::io::ErrorKind::NotFound => {
                    format!("CA certificate file not found: {}", ca_cert_path.display())
                }
                std::io::ErrorKind::PermissionDenied => format!(
                    "CA certificate file is not readable (permission denied): {}",
                    ca_cert_path.display()
                ),
                _ => format!("CA certificate file cannot be opened: {e}"),
            };
            return Err(ProtocolError::Tls(msg));
        }

        let counter = BROKER_CLIENT_ID_COUNTER.fetch_add(1, Ordering::Relaxed);
        let client_id = format!("tasmor_{}_{}", std::process::id(), counter);

        let create_opts = paho_mqtt::CreateOptionsBuilder::new()
            .server_uri(build_server_uri(&self.config))
            .client_id(client_id)
            .finalize();

        let client = paho_mqtt::AsyncClient::new(create_opts)
            .map_err(|e| ProtocolError::ConnectionFailed(e.to_string()))?;

        let conn_opts = build_connect_options(&self.config)?;

        let inner = MqttBrokerInner {
            client,
            subscriptions: RwLock::new(HashMap::new()),
            config: self.config,
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

        // Connect and wait with a user-defined timeout.
        let timeout = broker.inner.config.connection_timeout;
        match tokio::time::timeout(timeout, broker.inner.client.connect(conn_opts)).await {
            Ok(Ok(_)) => {
                broker.set_connected(true);
                tracing::info!(
                    host = %broker.inner.config.host,
                    port = %broker.inner.config.port,
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

/// Builds the broker server URI from its config.
///
/// Returns `ssl://` scheme when TLS is enabled, `tcp://` otherwise.
/// IPv6 addresses are wrapped in brackets as required by the URI format.
fn build_server_uri(config: &MqttBrokerConfig) -> String {
    let scheme = match config.tls {
        TlsConfig::Disabled => "tcp",
        TlsConfig::Enabled { .. } => "ssl",
    };
    let host = if config.host.contains(':') && !config.host.starts_with('[') {
        format!("[{}]", config.host)
    } else {
        config.host.clone()
    };
    format!("{scheme}://{host}:{}", config.port)
}

/// Builds the paho `ConnectOptions` from the broker config.
///
/// Configures keep-alive, session, auto-reconnect, optional credentials, and
/// TLS (CA cert + mandatory server verification) when enabled.
fn build_connect_options(
    config: &MqttBrokerConfig,
) -> Result<paho_mqtt::ConnectOptions, ProtocolError> {
    let mut b = paho_mqtt::ConnectOptionsBuilder::new();
    b.keep_alive_interval(config.keep_alive)
        .clean_session(true)
        .automatic_reconnect(Duration::from_millis(500), Duration::from_secs(60));
    if let Some(creds) = &config.credentials {
        b.user_name(creds.username()).password(creds.password());
    }
    if let TlsConfig::Enabled { ca_cert_path } = &config.tls {
        let mut ssl_b = paho_mqtt::SslOptionsBuilder::new();
        ssl_b
            .trust_store(ca_cert_path)
            .map_err(|e| ProtocolError::Tls(e.to_string()))?
            .enable_server_cert_auth(true)
            .verify(true);
        b.ssl_options(ssl_b.finalize());
    }
    Ok(b.finalize())
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
    fn builder_default_tls_disabled() {
        let builder = MqttBrokerBuilder::default();
        assert!(matches!(builder.config.tls, TlsConfig::Disabled));
    }

    #[test]
    fn builder_tls_ca_cert_sets_enabled() {
        let builder = MqttBrokerBuilder::default().tls_ca_cert("/path/to/ca.pem");
        assert!(matches!(builder.config.tls, TlsConfig::Enabled { .. }));
    }

    #[test]
    fn builder_tls_ca_cert_overrides_previous_call() {
        let builder = MqttBrokerBuilder::default()
            .tls_ca_cert("/first/ca.pem")
            .tls_ca_cert("/second/ca.pem");
        if let TlsConfig::Enabled { ca_cert_path } = &builder.config.tls {
            assert_eq!(ca_cert_path.to_str().unwrap(), "/second/ca.pem");
        } else {
            panic!("expected TlsConfig::Enabled");
        }
    }

    #[test]
    fn builder_tls_ca_cert_bumps_default_port() {
        let builder = MqttBrokerBuilder::default().tls_ca_cert("/ca.pem");
        assert_eq!(builder.config.port, 8883);
    }

    #[test]
    fn builder_tls_ca_cert_preserves_explicit_port() {
        let builder = MqttBrokerBuilder::default()
            .port(8885)
            .tls_ca_cert("/ca.pem");
        assert_eq!(builder.config.port, 8885);
    }

    #[test]
    fn builder_chain_includes_tls_ca_cert() {
        let builder = MqttBrokerBuilder::default()
            .host("192.168.1.50")
            .port(8883)
            .credentials("admin", "secret")
            .keep_alive(Duration::from_secs(45))
            .connection_timeout(Duration::from_secs(15))
            .command_timeout(Duration::from_secs(10))
            .tls_ca_cert("/etc/ssl/broker-ca.pem");

        assert_eq!(builder.config.host, "192.168.1.50");
        assert_eq!(builder.config.port, 8883);
        assert!(builder.config.credentials.is_some());
        assert!(matches!(builder.config.tls, TlsConfig::Enabled { .. }));
    }

    #[test]
    fn build_server_uri_tcp() {
        let config = MqttBrokerConfig {
            host: "192.168.1.50".to_string(),
            port: 1883,
            ..MqttBrokerConfig::default()
        };
        assert_eq!(build_server_uri(&config), "tcp://192.168.1.50:1883");
    }

    #[test]
    fn build_server_uri_ssl() {
        let config = MqttBrokerConfig {
            host: "broker.example.com".to_string(),
            port: 8883,
            tls: TlsConfig::Enabled {
                ca_cert_path: "/etc/ssl/ca.pem".into(),
            },
            ..MqttBrokerConfig::default()
        };
        assert_eq!(build_server_uri(&config), "ssl://broker.example.com:8883");
    }

    #[test]
    fn build_server_uri_ipv6_wrapped_in_brackets() {
        let config = MqttBrokerConfig {
            host: "::1".to_string(),
            port: 1883,
            ..MqttBrokerConfig::default()
        };
        assert_eq!(build_server_uri(&config), "tcp://[::1]:1883");
    }

    #[test]
    fn build_server_uri_ipv6_already_bracketed() {
        let config = MqttBrokerConfig {
            host: "[::1]".to_string(),
            port: 1883,
            ..MqttBrokerConfig::default()
        };
        assert_eq!(build_server_uri(&config), "tcp://[::1]:1883");
    }

    #[test]
    fn build_connect_options_tls_nul_byte_returns_tls_error() {
        let config = MqttBrokerConfig {
            host: "broker.example.com".to_string(),
            port: 8883,
            tls: TlsConfig::Enabled {
                ca_cert_path: "path/with\0nul".into(),
            },
            ..MqttBrokerConfig::default()
        };
        let result = build_connect_options(&config);
        assert!(matches!(result, Err(ProtocolError::Tls(_))));
    }

    #[test]
    fn build_connect_options_preserves_credentials() {
        let mut config = MqttBrokerConfig::default();
        config.credentials = Some(crate::credentials::Credentials::new("user", "pass"));
        let opts = build_connect_options(&config).unwrap();
        // We cannot inspect ConnectOptions fields directly; verify build succeeds
        // when credentials are present (regression guard for the extraction refactor).
        drop(opts);
    }

    #[tokio::test]
    async fn build_tls_missing_cert_returns_tls_error() {
        let result = MqttBrokerBuilder::default()
            .host("127.0.0.1")
            .tls_ca_cert("/nonexistent/path/ca.pem")
            .build()
            .await;
        assert!(matches!(result, Err(ProtocolError::Tls(_))));
    }

    #[tokio::test]
    async fn build_tls_not_found_message_contains_path() {
        let result = MqttBrokerBuilder::default()
            .host("127.0.0.1")
            .tls_ca_cert("/nonexistent/path/ca.pem")
            .build()
            .await;
        let Err(ProtocolError::Tls(msg)) = result else {
            panic!("expected Tls error");
        };
        assert!(
            msg.contains("/nonexistent/path/ca.pem"),
            "error message did not contain the cert path: {msg}"
        );
    }

    #[tokio::test]
    async fn build_missing_host_and_tls_cert_returns_invalid_address_not_tls() {
        // The host guard must fire before the TLS file check.
        let result = MqttBrokerBuilder::default()
            .tls_ca_cert("/nonexistent/ca.pem")
            .build()
            .await;
        assert!(matches!(result, Err(ProtocolError::InvalidAddress(_))));
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn build_tls_unreadable_cert_returns_permission_denied_message() {
        use std::os::unix::fs::PermissionsExt as _;
        let dir = tempfile::tempdir().unwrap();
        let cert = dir.path().join("ca.pem");
        std::fs::write(&cert, b"fake cert").unwrap();
        std::fs::set_permissions(&cert, std::fs::Permissions::from_mode(0o000)).unwrap();
        // Skip when running as root — root bypasses file permission checks.
        if std::fs::File::open(&cert).is_ok() {
            return;
        }
        let result = MqttBrokerBuilder::default()
            .host("127.0.0.1")
            .tls_ca_cert(&cert)
            .build()
            .await;
        let Err(ProtocolError::Tls(msg)) = result else {
            panic!("expected Tls error");
        };
        assert!(
            msg.contains("permission denied"),
            "error message did not mention permission denied: {msg}"
        );
    }

    #[test]
    fn protocol_error_mqtt_from_paho_error_preserves_message() {
        let paho_err = paho_mqtt::Error::Failure;
        let proto_err: ProtocolError = paho_err.into();
        assert!(matches!(proto_err, ProtocolError::Mqtt(ref msg) if !msg.is_empty()));
    }
}
