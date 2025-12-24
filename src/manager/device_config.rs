// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device configuration types for the device manager.

use std::time::Duration;

use crate::Capabilities;

/// Configuration for a managed device.
///
/// # Examples
///
/// ```
/// use tasmor_lib::manager::DeviceConfig;
///
/// // MQTT device configuration
/// let config = DeviceConfig::mqtt("mqtt://192.168.1.50:1883", "tasmota_bulb");
///
/// // HTTP device configuration
/// let config = DeviceConfig::http("192.168.1.100");
///
/// // With optional settings
/// let config = DeviceConfig::mqtt("mqtt://broker:1883", "device")
///     .with_capabilities(tasmor_lib::Capabilities::rgbcct_light())
///     .with_friendly_name("Living Room Light");
/// ```
#[derive(Debug, Clone)]
pub struct DeviceConfig {
    /// The connection configuration.
    pub connection: ConnectionConfig,
    /// Optional device capabilities (auto-detected if not provided).
    pub capabilities: Option<Capabilities>,
    /// Optional friendly name for the device.
    pub friendly_name: Option<String>,
    /// Reconnection policy.
    pub reconnection: ReconnectionPolicy,
}

impl DeviceConfig {
    /// Creates a configuration for an MQTT device.
    #[must_use]
    pub fn mqtt(broker_url: impl Into<String>, topic: impl Into<String>) -> Self {
        Self {
            connection: ConnectionConfig::Mqtt {
                broker_url: broker_url.into(),
                topic: topic.into(),
                credentials: None,
            },
            capabilities: None,
            friendly_name: None,
            reconnection: ReconnectionPolicy::default(),
        }
    }

    /// Creates a configuration for an HTTP device.
    #[must_use]
    pub fn http(host: impl Into<String>) -> Self {
        Self {
            connection: ConnectionConfig::Http {
                host: host.into(),
                port: 80,
                credentials: None,
                use_https: false,
            },
            capabilities: None,
            friendly_name: None,
            reconnection: ReconnectionPolicy::default(),
        }
    }

    /// Sets the device capabilities.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    /// Sets a friendly name for the device.
    #[must_use]
    pub fn with_friendly_name(mut self, name: impl Into<String>) -> Self {
        self.friendly_name = Some(name.into());
        self
    }

    /// Sets MQTT credentials.
    ///
    /// Only applicable for MQTT connections.
    #[must_use]
    pub fn with_mqtt_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        if let ConnectionConfig::Mqtt { credentials, .. } = &mut self.connection {
            *credentials = Some((username.into(), password.into()));
        }
        self
    }

    /// Sets HTTP credentials.
    ///
    /// Only applicable for HTTP connections.
    #[must_use]
    pub fn with_http_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        if let ConnectionConfig::Http { credentials, .. } = &mut self.connection {
            *credentials = Some((username.into(), password.into()));
        }
        self
    }

    /// Sets the HTTP port.
    ///
    /// Only applicable for HTTP connections.
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        if let ConnectionConfig::Http { port: p, .. } = &mut self.connection {
            *p = port;
        }
        self
    }

    /// Enables HTTPS.
    ///
    /// Only applicable for HTTP connections.
    #[must_use]
    pub fn with_https(mut self) -> Self {
        if let ConnectionConfig::Http { use_https, .. } = &mut self.connection {
            *use_https = true;
        }
        self
    }

    /// Sets the reconnection policy.
    #[must_use]
    pub fn with_reconnection(mut self, policy: ReconnectionPolicy) -> Self {
        self.reconnection = policy;
        self
    }

    /// Returns true if this is an MQTT connection.
    #[must_use]
    pub fn is_mqtt(&self) -> bool {
        matches!(self.connection, ConnectionConfig::Mqtt { .. })
    }

    /// Returns true if this is an HTTP connection.
    #[must_use]
    pub fn is_http(&self) -> bool {
        matches!(self.connection, ConnectionConfig::Http { .. })
    }
}

/// Connection configuration for a device.
#[derive(Debug, Clone)]
pub enum ConnectionConfig {
    /// MQTT connection configuration.
    Mqtt {
        /// The MQTT broker URL (e.g., `mqtt://192.168.1.50:1883`).
        broker_url: String,
        /// The device topic (e.g., `tasmota_switch`).
        topic: String,
        /// Optional (username, password) for broker authentication.
        credentials: Option<(String, String)>,
    },
    /// HTTP connection configuration.
    Http {
        /// The device host or IP address.
        host: String,
        /// The HTTP port (default 80).
        port: u16,
        /// Optional (username, password) for device authentication.
        credentials: Option<(String, String)>,
        /// Whether to use HTTPS.
        use_https: bool,
    },
}

/// Configuration for automatic reconnection.
///
/// # Examples
///
/// ```
/// use std::time::Duration;
/// use tasmor_lib::manager::ReconnectionPolicy;
///
/// // Default policy (enabled with exponential backoff)
/// let policy = ReconnectionPolicy::default();
///
/// // Disable reconnection
/// let policy = ReconnectionPolicy::disabled();
///
/// // Custom policy
/// let policy = ReconnectionPolicy::new()
///     .with_max_retries(5)
///     .with_initial_delay(Duration::from_millis(500))
///     .with_max_delay(Duration::from_secs(30));
/// ```
#[derive(Debug, Clone)]
pub struct ReconnectionPolicy {
    /// Whether automatic reconnection is enabled.
    pub enabled: bool,
    /// Maximum number of retries before giving up (None = infinite).
    pub max_retries: Option<u32>,
    /// Initial delay between retry attempts.
    pub initial_delay: Duration,
    /// Maximum delay between retry attempts (for exponential backoff).
    pub max_delay: Duration,
    /// Multiplier for exponential backoff.
    pub backoff_multiplier: f32,
}

impl ReconnectionPolicy {
    /// Creates a new reconnection policy with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a disabled reconnection policy.
    #[must_use]
    pub fn disabled() -> Self {
        Self {
            enabled: false,
            ..Self::default()
        }
    }

    /// Sets the maximum number of retries.
    #[must_use]
    pub fn with_max_retries(mut self, max_retries: u32) -> Self {
        self.max_retries = Some(max_retries);
        self
    }

    /// Sets infinite retries.
    #[must_use]
    pub fn with_infinite_retries(mut self) -> Self {
        self.max_retries = None;
        self
    }

    /// Sets the initial delay between retry attempts.
    #[must_use]
    pub fn with_initial_delay(mut self, delay: Duration) -> Self {
        self.initial_delay = delay;
        self
    }

    /// Sets the maximum delay between retry attempts.
    #[must_use]
    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    /// Sets the backoff multiplier.
    #[must_use]
    pub fn with_backoff_multiplier(mut self, multiplier: f32) -> Self {
        self.backoff_multiplier = multiplier;
        self
    }

    /// Calculates the delay for a given retry attempt.
    #[must_use]
    pub fn delay_for_attempt(&self, attempt: u32) -> Duration {
        if attempt == 0 {
            return self.initial_delay;
        }

        let multiplier = self
            .backoff_multiplier
            .powi(i32::try_from(attempt).unwrap_or(i32::MAX));

        // Safe: initial_delay is typically seconds/minutes, not near u128 max
        #[allow(clippy::cast_precision_loss)]
        let delay_ms = self.initial_delay.as_millis() as f32 * multiplier;

        // Safe: delay_ms is always positive (from Duration) and within practical bounds
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let delay = Duration::from_millis(delay_ms as u64);

        delay.min(self.max_delay)
    }

    /// Returns true if another retry should be attempted.
    #[must_use]
    pub fn should_retry(&self, attempt: u32) -> bool {
        self.enabled && self.max_retries.is_none_or(|max| attempt < max)
    }
}

impl Default for ReconnectionPolicy {
    fn default() -> Self {
        Self {
            enabled: true,
            max_retries: Some(10),
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            backoff_multiplier: 2.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mqtt_config_creation() {
        let config = DeviceConfig::mqtt("mqtt://localhost:1883", "tasmota_bulb");

        assert!(config.is_mqtt());
        assert!(!config.is_http());

        if let ConnectionConfig::Mqtt {
            broker_url, topic, ..
        } = &config.connection
        {
            assert_eq!(broker_url, "mqtt://localhost:1883");
            assert_eq!(topic, "tasmota_bulb");
        } else {
            panic!("Expected MQTT config");
        }
    }

    #[test]
    fn http_config_creation() {
        let config = DeviceConfig::http("192.168.1.100");

        assert!(config.is_http());
        assert!(!config.is_mqtt());

        if let ConnectionConfig::Http { host, port, .. } = &config.connection {
            assert_eq!(host, "192.168.1.100");
            assert_eq!(*port, 80);
        } else {
            panic!("Expected HTTP config");
        }
    }

    #[test]
    fn config_with_options() {
        let config = DeviceConfig::mqtt("mqtt://broker:1883", "device")
            .with_friendly_name("Test Device")
            .with_capabilities(Capabilities::default());

        assert_eq!(config.friendly_name, Some("Test Device".to_string()));
        assert!(config.capabilities.is_some());
    }

    #[test]
    fn config_with_mqtt_credentials() {
        let config = DeviceConfig::mqtt("mqtt://broker:1883", "device")
            .with_mqtt_credentials("user", "pass");

        if let ConnectionConfig::Mqtt { credentials, .. } = &config.connection {
            assert_eq!(credentials, &Some(("user".to_string(), "pass".to_string())));
        } else {
            panic!("Expected MQTT config");
        }
    }

    #[test]
    fn config_with_http_options() {
        let config = DeviceConfig::http("192.168.1.100")
            .with_port(8080)
            .with_https()
            .with_http_credentials("admin", "secret");

        if let ConnectionConfig::Http {
            port,
            use_https,
            credentials,
            ..
        } = &config.connection
        {
            assert_eq!(*port, 8080);
            assert!(*use_https);
            assert_eq!(
                credentials,
                &Some(("admin".to_string(), "secret".to_string()))
            );
        } else {
            panic!("Expected HTTP config");
        }
    }

    #[test]
    fn reconnection_policy_default() {
        let policy = ReconnectionPolicy::default();

        assert!(policy.enabled);
        assert_eq!(policy.max_retries, Some(10));
        assert_eq!(policy.initial_delay, Duration::from_secs(1));
    }

    #[test]
    fn reconnection_policy_disabled() {
        let policy = ReconnectionPolicy::disabled();

        assert!(!policy.enabled);
        assert!(!policy.should_retry(0));
    }

    #[test]
    fn reconnection_delay_calculation() {
        let policy = ReconnectionPolicy::new()
            .with_initial_delay(Duration::from_secs(1))
            .with_backoff_multiplier(2.0)
            .with_max_delay(Duration::from_secs(10));

        assert_eq!(policy.delay_for_attempt(0), Duration::from_secs(1));
        assert_eq!(policy.delay_for_attempt(1), Duration::from_secs(2));
        assert_eq!(policy.delay_for_attempt(2), Duration::from_secs(4));
        assert_eq!(policy.delay_for_attempt(3), Duration::from_secs(8));
        // Should be capped at max_delay
        assert_eq!(policy.delay_for_attempt(4), Duration::from_secs(10));
    }

    #[test]
    fn reconnection_should_retry() {
        let policy = ReconnectionPolicy::new().with_max_retries(3);

        assert!(policy.should_retry(0));
        assert!(policy.should_retry(1));
        assert!(policy.should_retry(2));
        assert!(!policy.should_retry(3));
        assert!(!policy.should_retry(4));
    }

    #[test]
    fn reconnection_infinite_retries() {
        let policy = ReconnectionPolicy::new().with_infinite_retries();

        assert!(policy.should_retry(0));
        assert!(policy.should_retry(100));
        assert!(policy.should_retry(1000));
    }
}
