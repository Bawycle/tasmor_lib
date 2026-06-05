// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::path::PathBuf;
use std::time::Duration;

use crate::credentials::Credentials;

use super::DEFAULT_COMMAND_TIMEOUT;

/// TLS configuration for the broker connection.
#[derive(Clone, Default)]
pub(super) enum TlsConfig {
    /// Plaintext connection (default).
    #[default]
    Disabled,
    /// TLS connection using the OS system CA trust store.
    SystemRoots,
    /// TLS connection with mandatory CA certificate verification against an explicit PEM file.
    CaCert { ca_cert_path: PathBuf },
}

impl std::fmt::Debug for TlsConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Disabled => write!(f, "Disabled"),
            Self::SystemRoots => write!(f, "SystemRoots"),
            Self::CaCert { .. } => f
                .debug_struct("CaCert")
                .field("ca_cert_path", &"[REDACTED]")
                .finish(),
        }
    }
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
/// lifetime — that copy is outside Rust's control. Use [`crate::protocol::MqttBrokerBuilder::tls_ca_cert`]
/// to enable TLS and protect credentials in transit.
#[derive(Clone)]
pub struct MqttBrokerConfig {
    pub(super) host: String,
    pub(super) port: u16,
    pub(super) credentials: Option<Credentials>,
    pub(super) keep_alive: Duration,
    pub(super) connection_timeout: Duration,
    pub(super) command_timeout: Duration,
    pub(super) tls: TlsConfig,
}

impl std::fmt::Debug for MqttBrokerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MqttBrokerConfig")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("credentials", &self.credentials)
            .field("keep_alive", &self.keep_alive)
            .field("connection_timeout", &self.connection_timeout)
            .field("command_timeout", &self.command_timeout)
            .field("tls", &self.tls)
            .finish()
    }
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
            tls: TlsConfig::Disabled,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_default() {
        let config = MqttBrokerConfig::default();
        assert!(config.host.is_empty());
        assert_eq!(config.port, 1883);
        assert!(config.credentials.is_none());
    }
}
