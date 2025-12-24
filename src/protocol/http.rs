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

/// HTTP client for communicating with Tasmota devices.
///
/// Uses the Tasmota web API endpoint `/cm?cmnd=<command>` for sending commands.
///
/// # Examples
///
/// ```ignore
/// use tasmor_lib::protocol::HttpClient;
/// use tasmor_lib::command::PowerCommand;
/// use tasmor_lib::types::PowerIndex;
///
/// let client = HttpClient::new("192.168.1.100")?;
/// let response = client.send_command(&PowerCommand::query(PowerIndex::one())).await?;
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
}
