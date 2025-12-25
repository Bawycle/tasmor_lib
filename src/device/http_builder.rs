// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! HTTP device builder.

use crate::capabilities::Capabilities;
use crate::command::StatusCommand;
use crate::device::Device;
use crate::error::Error;
use crate::protocol::{HttpClient, HttpConfig, Protocol};
use crate::response::StatusResponse;

/// Builder for creating HTTP-based devices.
///
/// This builder can be created in two ways:
/// - `Device::http("host")` - Simple host string
/// - `Device::http_config(HttpConfig::new("host").with_port(8080))` - Advanced configuration
///
/// # Examples
///
/// ```no_run
/// use tasmor_lib::Device;
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// // Simple: with auto-detection
/// let device = Device::http("192.168.1.100")
///     .build()
///     .await?;
///
/// // With credentials
/// let device = Device::http("192.168.1.100")
///     .with_credentials("admin", "password")
///     .build()
///     .await?;
///
/// // With manual capabilities (no network probe)
/// let device = Device::http("192.168.1.100")
///     .with_capabilities(tasmor_lib::Capabilities::rgbcct_light())
///     .build_without_probe()?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug)]
pub struct HttpDeviceBuilder {
    config: HttpConfig,
    capabilities: Option<Capabilities>,
}

impl HttpDeviceBuilder {
    /// Creates a new builder with the specified HTTP configuration.
    pub(crate) fn new(config: HttpConfig) -> Self {
        Self {
            config,
            capabilities: None,
        }
    }

    /// Sets authentication credentials.
    ///
    /// This creates a new `HttpConfig` with the credentials added.
    ///
    /// # Arguments
    ///
    /// * `username` - The username for HTTP basic authentication
    /// * `password` - The password for HTTP basic authentication
    #[must_use]
    pub fn with_credentials(
        mut self,
        username: impl Into<String>,
        password: impl Into<String>,
    ) -> Self {
        // Store current values before modifying
        let host = self.config.host().to_string();
        let port = self.config.port();
        let timeout = self.config.timeout();
        let use_https = self.config.use_https();

        // Reconstruct the config with credentials
        let mut new_config = HttpConfig::new(host)
            .with_port(port)
            .with_timeout(timeout)
            .with_credentials(username, password);

        if use_https {
            new_config = new_config.with_https();
        }

        self.config = new_config;
        self
    }

    /// Sets the device capabilities manually (skips auto-detection).
    ///
    /// Use this when you know the device capabilities and want to avoid
    /// the initial status query.
    #[must_use]
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = Some(capabilities);
        self
    }

    /// Returns the currently set capabilities, if any.
    #[must_use]
    pub fn capabilities(&self) -> Option<&Capabilities> {
        self.capabilities.as_ref()
    }

    /// Builds the device with auto-detection of capabilities.
    ///
    /// This will query the device status to detect capabilities.
    /// Use [`build_without_probe`](Self::build_without_probe) if you've set
    /// capabilities manually and want to skip the network query.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Connection fails
    /// - Capability detection fails
    pub async fn build(self) -> Result<Device<HttpClient>, Error> {
        let client = self.config.into_client().map_err(Error::Protocol)?;

        // Auto-detect capabilities if not set
        let capabilities = if let Some(caps) = self.capabilities {
            caps
        } else {
            let cmd = StatusCommand::all();
            let response = client.send_command(&cmd).await.map_err(Error::Protocol)?;
            let status: StatusResponse = response.parse().map_err(Error::Parse)?;
            Capabilities::from_status(&status)
        };

        Ok(Device::new(client, capabilities))
    }

    /// Builds the device without probing for capabilities.
    ///
    /// Use this when you've set capabilities manually via [`with_capabilities`](Self::with_capabilities)
    /// or want faster startup without network access.
    ///
    /// If capabilities were not set, defaults to [`Capabilities::default()`].
    ///
    /// # Errors
    ///
    /// Returns error if the HTTP client cannot be created.
    pub fn build_without_probe(self) -> Result<Device<HttpClient>, Error> {
        let client = self.config.into_client().map_err(Error::Protocol)?;
        let capabilities = self.capabilities.unwrap_or_default();
        Ok(Device::new(client, capabilities))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_new() {
        let config = HttpConfig::new("192.168.1.100");
        let builder = HttpDeviceBuilder::new(config);
        assert!(builder.capabilities.is_none());
    }

    #[test]
    fn builder_with_capabilities() {
        let config = HttpConfig::new("192.168.1.100");
        let builder =
            HttpDeviceBuilder::new(config).with_capabilities(Capabilities::rgbcct_light());
        assert!(builder.capabilities.is_some());
    }

    #[test]
    fn builder_capabilities_accessor() {
        let config = HttpConfig::new("192.168.1.100");
        let builder = HttpDeviceBuilder::new(config);
        assert!(builder.capabilities().is_none());

        let builder = builder.with_capabilities(Capabilities::basic());
        assert!(builder.capabilities().is_some());
    }

    #[test]
    fn builder_build_without_probe() {
        let config = HttpConfig::new("192.168.1.100");
        let result = HttpDeviceBuilder::new(config)
            .with_capabilities(Capabilities::neo_coolcam())
            .build_without_probe();

        assert!(result.is_ok());
        let device = result.unwrap();
        assert!(device.capabilities().supports_energy_monitoring());
    }

    #[test]
    fn builder_build_without_probe_default_capabilities() {
        let config = HttpConfig::new("192.168.1.100");
        let result = HttpDeviceBuilder::new(config).build_without_probe();

        assert!(result.is_ok());
        let device = result.unwrap();
        // Default capabilities - verify we got a device
        // (default capabilities are minimal)
        assert!(!device.capabilities().is_multi_relay());
    }
}
