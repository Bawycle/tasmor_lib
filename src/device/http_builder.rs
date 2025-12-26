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
use crate::state::DeviceState;

/// Builder for creating HTTP-based devices.
///
/// This builder can be created in two ways:
/// - `Device::http("host")` - Simple host string
/// - `Device::http_config(HttpConfig::new("host").with_port(8080))` - Advanced configuration
///
/// Both `build()` and `build_without_probe()` return the device along with its
/// initial state, containing current values for power, energy, colors, etc.
///
/// # Examples
///
/// ```no_run
/// use tasmor_lib::Device;
///
/// # async fn example() -> tasmor_lib::Result<()> {
/// // Simple: with auto-detection - returns (device, initial_state)
/// let (device, initial_state) = Device::http("192.168.1.100")
///     .build()
///     .await?;
///
/// // Access initial state
/// println!("Power: {:?}", initial_state.power(1));
///
/// // With credentials
/// let (device, state) = Device::http("192.168.1.100")
///     .with_credentials("admin", "password")
///     .build()
///     .await?;
///
/// // With manual capabilities (skips capability probe, still queries state)
/// let (device, state) = Device::http("192.168.1.100")
///     .with_capabilities(tasmor_lib::Capabilities::rgbcct_light())
///     .build_without_probe()
///     .await?;
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
    /// This will query the device status to detect capabilities, then query
    /// the device for its current state (power, energy, colors, etc.).
    ///
    /// Returns a tuple of `(Device, DeviceState)` where `DeviceState` contains
    /// the initial values for all supported capabilities.
    ///
    /// Use [`build_without_probe`](Self::build_without_probe) if you've set
    /// capabilities manually and want to skip the capability detection query.
    ///
    /// # Errors
    ///
    /// Returns error if:
    /// - Connection fails
    /// - Capability detection fails
    /// - Initial state query fails
    pub async fn build(self) -> Result<(Device<HttpClient>, DeviceState), Error> {
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

        let device = Device::new(client, capabilities);

        // Query initial state
        let initial_state = device.query_state().await?;

        Ok((device, initial_state))
    }

    /// Builds the device without probing for capabilities.
    ///
    /// Use this when you've set capabilities manually via [`with_capabilities`](Self::with_capabilities).
    /// Still queries the device for its current state.
    ///
    /// Returns a tuple of `(Device, DeviceState)` where `DeviceState` contains
    /// the initial values for all supported capabilities.
    ///
    /// If capabilities were not set, defaults to [`Capabilities::default()`].
    ///
    /// # Errors
    ///
    /// Returns error if the HTTP client cannot be created or state query fails.
    pub async fn build_without_probe(self) -> Result<(Device<HttpClient>, DeviceState), Error> {
        let client = self.config.into_client().map_err(Error::Protocol)?;
        let capabilities = self.capabilities.unwrap_or_default();
        let device = Device::new(client, capabilities);

        // Query initial state
        let initial_state = device.query_state().await?;

        Ok((device, initial_state))
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

    // Note: build() and build_without_probe() tests are in integration tests
    // as they require network access to query initial state.
}
