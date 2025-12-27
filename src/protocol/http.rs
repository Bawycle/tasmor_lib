// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! HTTP protocol implementation for Tasmota devices.

use std::time::Duration;

use reqwest::Client;

use crate::command::Command;
use crate::error::ProtocolError;
use crate::protocol::{CommandResponse, Protocol};

// ============================================================================
// HttpConfig - Configuration for HTTP devices (new device-centric API)
// ============================================================================

/// Configuration for an HTTP Tasmota device.
///
/// This is a simple configuration struct that holds connection parameters.
/// HTTP is stateless - each command is an independent request.
/// No persistent connection, no event subscriptions.
///
/// # Examples
///
/// ```
/// use tasmor_lib::protocol::HttpConfig;
/// use std::time::Duration;
///
/// // Simple configuration
/// let config = HttpConfig::new("192.168.1.100");
///
/// // With all options
/// let config = HttpConfig::new("192.168.1.100")
///     .with_port(8080)
///     .with_https()
///     .with_credentials("admin", "password")
///     .with_timeout(Duration::from_secs(5));
/// ```
#[derive(Debug, Clone)]
pub struct HttpConfig {
    host: String,
    port: u16,
    use_https: bool,
    credentials: Option<(String, String)>,
    timeout: Duration,
}

impl HttpConfig {
    /// Default HTTP port.
    pub const DEFAULT_PORT: u16 = 80;
    /// Default HTTPS port.
    pub const DEFAULT_HTTPS_PORT: u16 = 443;
    /// Default request timeout.
    pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

    /// Creates a new HTTP configuration for the specified host.
    ///
    /// # Arguments
    ///
    /// * `host` - The hostname or IP address of the Tasmota device
    #[must_use]
    pub fn new(host: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: Self::DEFAULT_PORT,
            use_https: false,
            credentials: None,
            timeout: Self::DEFAULT_TIMEOUT,
        }
    }

    /// Sets a custom port.
    #[must_use]
    pub fn with_port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// Enables HTTPS.
    ///
    /// If port hasn't been explicitly set, it will be changed to 443.
    #[must_use]
    pub fn with_https(mut self) -> Self {
        self.use_https = true;
        if self.port == Self::DEFAULT_PORT {
            self.port = Self::DEFAULT_HTTPS_PORT;
        }
        self
    }

    /// Sets authentication credentials.
    #[must_use]
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.credentials = Some((username.into(), password.into()));
        self
    }

    /// Sets the request timeout.
    #[must_use]
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Returns the host.
    #[must_use]
    pub fn host(&self) -> &str {
        &self.host
    }

    /// Returns the port.
    #[must_use]
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Returns whether HTTPS is enabled.
    #[must_use]
    pub fn use_https(&self) -> bool {
        self.use_https
    }

    /// Returns the credentials if set.
    #[must_use]
    pub fn credentials(&self) -> Option<(&str, &str)> {
        self.credentials
            .as_ref()
            .map(|(u, p)| (u.as_str(), p.as_str()))
    }

    /// Returns the timeout.
    #[must_use]
    pub fn timeout(&self) -> Duration {
        self.timeout
    }

    /// Builds the base URL from this configuration.
    #[must_use]
    pub fn base_url(&self) -> String {
        let scheme = if self.use_https { "https" } else { "http" };
        let port_suffix =
            if (self.use_https && self.port == 443) || (!self.use_https && self.port == 80) {
                String::new()
            } else {
                format!(":{}", self.port)
            };
        format!("{scheme}://{}{port_suffix}", self.host)
    }

    /// Creates an `HttpClient` from this configuration.
    ///
    /// # Errors
    ///
    /// Returns error if the HTTP client cannot be created.
    pub fn into_client(self) -> Result<HttpClient, ProtocolError> {
        let base_url = self.base_url();

        let client = Client::builder()
            .timeout(self.timeout)
            .build()
            .map_err(ProtocolError::Http)?;

        let credentials = self
            .credentials
            .map(|(username, password)| Credentials { username, password });

        Ok(HttpClient {
            base_url,
            client,
            credentials,
        })
    }
}

// ============================================================================
// HttpClient - Internal HTTP client implementation
// ============================================================================

/// HTTP client for communicating with Tasmota devices.
///
/// Uses the Tasmota web API endpoint `/cm?cmnd=<command>` for sending commands.
///
/// # Examples
///
/// ```no_run
/// use tasmor_lib::protocol::{HttpClient, Protocol};
/// use tasmor_lib::command::PowerCommand;
/// use tasmor_lib::types::PowerIndex;
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// let client = HttpClient::new("192.168.1.100")?;
/// let response = client.send_command(&PowerCommand::query(PowerIndex::one())).await?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone)]
pub struct HttpClient {
    base_url: String,
    client: Client,
    credentials: Option<Credentials>,
}

/// HTTP authentication credentials.
#[derive(Debug, Clone)]
pub struct Credentials {
    /// Username for authentication.
    pub username: String,
    /// Password for authentication.
    pub password: String,
}

impl HttpClient {
    /// Creates a new HTTP client for the specified host.
    ///
    /// # Arguments
    ///
    /// * `host` - The hostname or IP address of the Tasmota device
    ///
    /// # Errors
    ///
    /// Returns error if the HTTP client cannot be created.
    pub fn new(host: impl Into<String>) -> Result<Self, ProtocolError> {
        let host = host.into();
        let base_url = if host.starts_with("http://") || host.starts_with("https://") {
            host
        } else {
            format!("http://{host}")
        };

        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .map_err(ProtocolError::Http)?;

        Ok(Self {
            base_url,
            client,
            credentials: None,
        })
    }

    /// Sets authentication credentials.
    #[must_use]
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        self.credentials = Some(Credentials {
            username: username.into(),
            password: password.into(),
        });
        self
    }

    /// Returns the base URL of the device.
    #[must_use]
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Builds the URL for a command.
    fn build_url(&self, command: &str) -> String {
        let encoded_command = urlencoding::encode(command);

        match &self.credentials {
            Some(creds) => {
                format!(
                    "{}/cm?user={}&password={}&cmnd={}",
                    self.base_url,
                    urlencoding::encode(&creds.username),
                    urlencoding::encode(&creds.password),
                    encoded_command
                )
            }
            None => {
                format!("{}/cm?cmnd={}", self.base_url, encoded_command)
            }
        }
    }
}

impl Protocol for HttpClient {
    async fn send_command<C: Command + Sync>(
        &self,
        command: &C,
    ) -> Result<CommandResponse, ProtocolError> {
        self.send_raw(&command.to_http_command()).await
    }

    async fn send_raw(&self, command: &str) -> Result<CommandResponse, ProtocolError> {
        let url = self.build_url(command);

        tracing::debug!(url = %url, "Sending HTTP command");

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(ProtocolError::Http)?;

        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            return Err(ProtocolError::AuthenticationFailed);
        }

        if !response.status().is_success() {
            return Err(ProtocolError::ConnectionFailed(format!(
                "HTTP {} - {}",
                response.status().as_u16(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let body = response.text().await.map_err(ProtocolError::Http)?;

        tracing::debug!(body = %body, "Received HTTP response");

        Ok(CommandResponse::new(body))
    }
}

/// Builder for creating an HTTP client with custom configuration.
#[derive(Debug, Default)]
pub struct HttpClientBuilder {
    host: Option<String>,
    username: Option<String>,
    password: Option<String>,
    timeout: Option<Duration>,
}

impl HttpClientBuilder {
    /// Creates a new builder.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the host address.
    #[must_use]
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = Some(host.into());
        self
    }

    /// Sets authentication credentials.
    #[must_use]
    pub fn credentials(mut self, username: impl Into<String>, password: impl Into<String>) -> Self {
        self.username = Some(username.into());
        self.password = Some(password.into());
        self
    }

    /// Sets the request timeout.
    #[must_use]
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Builds the HTTP client.
    ///
    /// # Errors
    ///
    /// Returns error if host is not set or client creation fails.
    pub fn build(self) -> Result<HttpClient, ProtocolError> {
        let host = self
            .host
            .ok_or_else(|| ProtocolError::InvalidAddress("host is required".to_string()))?;

        let base_url = if host.starts_with("http://") || host.starts_with("https://") {
            host
        } else {
            format!("http://{host}")
        };

        let client = Client::builder()
            .timeout(self.timeout.unwrap_or(Duration::from_secs(10)))
            .build()
            .map_err(ProtocolError::Http)?;

        let credentials = match (self.username, self.password) {
            (Some(username), Some(password)) => Some(Credentials { username, password }),
            _ => None,
        };

        Ok(HttpClient {
            base_url,
            client,
            credentials,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_url_without_auth() {
        let client = HttpClient::new("192.168.1.100").unwrap();
        let url = client.build_url("Power ON");
        assert_eq!(url, "http://192.168.1.100/cm?cmnd=Power%20ON");
    }

    #[test]
    fn build_url_with_auth() {
        let client = HttpClient::new("192.168.1.100")
            .unwrap()
            .with_credentials("admin", "pass");
        let url = client.build_url("Power ON");
        assert_eq!(
            url,
            "http://192.168.1.100/cm?user=admin&password=pass&cmnd=Power%20ON"
        );
    }

    #[test]
    fn build_url_with_https() {
        let client = HttpClient::new("https://192.168.1.100").unwrap();
        assert_eq!(client.base_url(), "https://192.168.1.100");
    }

    #[test]
    fn builder_missing_host() {
        let result = HttpClientBuilder::new().build();
        assert!(result.is_err());
    }

    #[test]
    fn builder_with_all_options() {
        let client = HttpClientBuilder::new()
            .host("192.168.1.100")
            .credentials("user", "pass")
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();

        assert!(client.credentials.is_some());
    }

    // =========================================================================
    // HttpConfig tests
    // =========================================================================

    #[test]
    fn http_config_default_values() {
        let config = HttpConfig::new("192.168.1.100");
        assert_eq!(config.host(), "192.168.1.100");
        assert_eq!(config.port(), 80);
        assert!(!config.use_https());
        assert!(config.credentials().is_none());
        assert_eq!(config.timeout(), Duration::from_secs(10));
    }

    #[test]
    fn http_config_with_port() {
        let config = HttpConfig::new("192.168.1.100").with_port(8080);
        assert_eq!(config.port(), 8080);
    }

    #[test]
    fn http_config_with_https() {
        let config = HttpConfig::new("192.168.1.100").with_https();
        assert!(config.use_https());
        assert_eq!(config.port(), 443); // Port should change to 443
    }

    #[test]
    fn http_config_with_https_custom_port() {
        let config = HttpConfig::new("192.168.1.100")
            .with_port(8443)
            .with_https();
        assert!(config.use_https());
        assert_eq!(config.port(), 8443); // Port should stay as explicitly set
    }

    #[test]
    fn http_config_with_credentials() {
        let config = HttpConfig::new("192.168.1.100").with_credentials("admin", "secret");
        let creds = config.credentials().unwrap();
        assert_eq!(creds.0, "admin");
        assert_eq!(creds.1, "secret");
    }

    #[test]
    fn http_config_with_timeout() {
        let config = HttpConfig::new("192.168.1.100").with_timeout(Duration::from_secs(30));
        assert_eq!(config.timeout(), Duration::from_secs(30));
    }

    #[test]
    fn http_config_base_url_http() {
        let config = HttpConfig::new("192.168.1.100");
        assert_eq!(config.base_url(), "http://192.168.1.100");
    }

    #[test]
    fn http_config_base_url_http_custom_port() {
        let config = HttpConfig::new("192.168.1.100").with_port(8080);
        assert_eq!(config.base_url(), "http://192.168.1.100:8080");
    }

    #[test]
    fn http_config_base_url_https() {
        let config = HttpConfig::new("192.168.1.100").with_https();
        assert_eq!(config.base_url(), "https://192.168.1.100");
    }

    #[test]
    fn http_config_base_url_https_custom_port() {
        let config = HttpConfig::new("192.168.1.100")
            .with_port(8443)
            .with_https();
        assert_eq!(config.base_url(), "https://192.168.1.100:8443");
    }

    #[test]
    fn http_config_into_client() {
        let config = HttpConfig::new("192.168.1.100").with_credentials("user", "pass");
        let client = config.into_client().unwrap();
        assert_eq!(client.base_url(), "http://192.168.1.100");
        assert!(client.credentials.is_some());
    }

    #[test]
    fn http_config_builder_chain() {
        let config = HttpConfig::new("192.168.1.100")
            .with_port(8080)
            .with_credentials("admin", "password")
            .with_timeout(Duration::from_secs(5));

        assert_eq!(config.host(), "192.168.1.100");
        assert_eq!(config.port(), 8080);
        assert!(!config.use_https());
        assert!(config.credentials().is_some());
        assert_eq!(config.timeout(), Duration::from_secs(5));
    }
}
