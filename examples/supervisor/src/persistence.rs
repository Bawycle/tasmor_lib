//! Configuration persistence for Tasmota Supervisor.
//!
//! This module handles saving and loading device configurations to/from disk.

use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::device_config::DeviceConfig;

/// Application configuration that gets persisted.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    /// List of configured devices.
    pub devices: Vec<DeviceConfig>,
}

impl AppConfig {
    /// Returns the path to the configuration file.
    fn config_path() -> Option<PathBuf> {
        dirs::config_dir().map(|mut path| {
            path.push("tasmota-supervisor");
            path.push("config.json");
            path
        })
    }

    /// Loads the configuration from disk.
    ///
    /// Returns a default configuration if the file doesn't exist or can't be read.
    #[must_use]
    pub fn load() -> Self {
        let Some(path) = Self::config_path() else {
            tracing::warn!("Could not determine config directory");
            return Self::default();
        };

        if !path.exists() {
            tracing::info!("No config file found at {}, using defaults", path.display());
            return Self::default();
        }

        match fs::read_to_string(&path) {
            Ok(contents) => match serde_json::from_str(&contents) {
                Ok(config) => {
                    tracing::info!("Loaded configuration from {}", path.display());
                    config
                }
                Err(e) => {
                    tracing::error!("Failed to parse config file: {e}");
                    Self::default()
                }
            },
            Err(e) => {
                tracing::error!("Failed to read config file: {e}");
                Self::default()
            }
        }
    }

    /// Saves the configuration to disk.
    ///
    /// # Errors
    ///
    /// Returns an error if the configuration cannot be saved.
    pub fn save(&self) -> io::Result<()> {
        let Some(path) = Self::config_path() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "Could not determine config directory",
            ));
        };

        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&path, contents)?;

        tracing::info!("Saved configuration to {}", path.display());
        Ok(())
    }

    /// Adds a device to the configuration and saves it.
    pub fn add_device(&mut self, config: DeviceConfig) {
        self.devices.push(config);
        if let Err(e) = self.save() {
            tracing::error!("Failed to save config after adding device: {e}");
        }
    }

    /// Removes a device from the configuration and saves it.
    pub fn remove_device(&mut self, id: uuid::Uuid) {
        self.devices.retain(|d| d.id != id);
        if let Err(e) = self.save() {
            tracing::error!("Failed to save config after removing device: {e}");
        }
    }

    /// Updates a device in the configuration and saves it.
    pub fn update_device(&mut self, config: DeviceConfig) {
        // Find and replace the device with matching ID
        if let Some(device) = self.devices.iter_mut().find(|d| d.id == config.id) {
            *device = config;
            if let Err(e) = self.save() {
                tracing::error!("Failed to save config after updating device: {e}");
            }
        } else {
            // Device not found, add it
            self.add_device(config);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_exists() {
        let path = AppConfig::config_path();
        assert!(path.is_some());
        let path = path.unwrap();
        assert!(path.to_string_lossy().contains("tasmota-supervisor"));
    }

    #[test]
    fn default_config_is_empty() {
        let config = AppConfig::default();
        assert!(config.devices.is_empty());
    }
}
