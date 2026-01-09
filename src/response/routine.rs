// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Response parsing for routines.
//!
//! When executing a routine via Tasmota's `Backlog0` command, the device
//! returns a JSON object containing the results of individual actions.
//! The exact structure depends on which actions were executed.
//!
//! # Response Format
//!
//! Tasmota combines the responses from all actions in the routine into a
//! single JSON object. For example, executing a routine with `Power1 ON`
//! and `Dimmer 75` might return:
//!
//! ```json
//! {
//!   "POWER1": "ON",
//!   "Dimmer": 75
//! }
//! ```
//!
//! # Examples
//!
//! ```
//! use tasmor_lib::response::RoutineResponse;
//!
//! let json = r#"{"POWER1":"ON","Dimmer":75}"#;
//! let response: RoutineResponse = serde_json::from_str(json).unwrap();
//!
//! assert!(response.contains_key("POWER1"));
//! assert_eq!(response.get_as::<u8>("Dimmer").unwrap(), 75);
//! ```

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::error::ParseError;

/// Response from executing a routine.
///
/// This type provides a flexible interface for accessing the combined results
/// of multiple actions executed in a routine. The response fields depend on
/// which actions were executed.
///
/// # Common Fields
///
/// | Field | Type | Description |
/// |-------|------|-------------|
/// | `POWER`, `POWER1`-`POWER8` | `String` | Relay state ("ON" or "OFF") |
/// | `Dimmer` | `u8` | Brightness level (0-100) |
/// | `HSBColor` | `String` | Color in HSB format ("hue,sat,bri") |
/// | `CT` | `u16` | Color temperature in mireds |
/// | `Fade` | `String` | Fade enabled ("ON" or "OFF") |
/// | `Speed` | `u8` | Fade duration raw value (1-40) |
///
/// # Examples
///
/// ```
/// use tasmor_lib::response::RoutineResponse;
///
/// let json = r#"{"POWER":"ON","Dimmer":50,"Fade":"ON"}"#;
/// let response: RoutineResponse = serde_json::from_str(json).unwrap();
///
/// // Check if fields exist
/// assert!(response.contains_key("POWER"));
///
/// // Get typed values
/// let dimmer: u8 = response.get_as("Dimmer").unwrap();
/// assert_eq!(dimmer, 50);
///
/// // Iterate over all fields
/// for (key, value) in response.iter() {
///     println!("{}: {}", key, value);
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RoutineResponse {
    #[serde(flatten)]
    fields: HashMap<String, serde_json::Value>,
}

impl RoutineResponse {
    /// Creates a new empty routine response.
    ///
    /// Primarily useful for testing.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns `true` if the response contains the specified field.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::response::RoutineResponse;
    ///
    /// let json = r#"{"POWER":"ON"}"#;
    /// let response: RoutineResponse = serde_json::from_str(json).unwrap();
    ///
    /// assert!(response.contains_key("POWER"));
    /// assert!(!response.contains_key("Dimmer"));
    /// ```
    #[must_use]
    pub fn contains_key(&self, key: &str) -> bool {
        self.fields.contains_key(key)
    }

    /// Gets a field value by name as a raw JSON value.
    ///
    /// Returns `None` if the field doesn't exist.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::response::RoutineResponse;
    ///
    /// let json = r#"{"POWER":"ON","Dimmer":75}"#;
    /// let response: RoutineResponse = serde_json::from_str(json).unwrap();
    ///
    /// assert_eq!(response.get("POWER"), Some(&serde_json::json!("ON")));
    /// assert_eq!(response.get("Missing"), None);
    /// ```
    #[must_use]
    pub fn get(&self, key: &str) -> Option<&serde_json::Value> {
        self.fields.get(key)
    }

    /// Gets a field value parsed as a specific type.
    ///
    /// # Errors
    ///
    /// Returns [`ParseError::MissingField`] if the field doesn't exist.
    /// Returns [`ParseError::Json`] if the value cannot be parsed as type `T`.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::response::RoutineResponse;
    ///
    /// let json = r#"{"Dimmer":75,"POWER":"ON"}"#;
    /// let response: RoutineResponse = serde_json::from_str(json).unwrap();
    ///
    /// let dimmer: u8 = response.get_as("Dimmer").unwrap();
    /// assert_eq!(dimmer, 75);
    ///
    /// let power: String = response.get_as("POWER").unwrap();
    /// assert_eq!(power, "ON");
    /// ```
    pub fn get_as<T: serde::de::DeserializeOwned>(&self, key: &str) -> Result<T, ParseError> {
        let value = self
            .fields
            .get(key)
            .ok_or_else(|| ParseError::MissingField(key.to_string()))?;
        serde_json::from_value(value.clone()).map_err(Into::into)
    }

    /// Tries to get a field value as a specific type, returning `None` if
    /// the field doesn't exist or cannot be parsed.
    ///
    /// This is a more lenient version of [`get_as`](Self::get_as) that returns
    /// `None` instead of an error.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::response::RoutineResponse;
    ///
    /// let json = r#"{"Dimmer":75}"#;
    /// let response: RoutineResponse = serde_json::from_str(json).unwrap();
    ///
    /// let dimmer: Option<u8> = response.try_get_as("Dimmer");
    /// assert_eq!(dimmer, Some(75));
    ///
    /// let missing: Option<u8> = response.try_get_as("Missing");
    /// assert_eq!(missing, None);
    /// ```
    #[must_use]
    pub fn try_get_as<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        self.get_as(key).ok()
    }

    /// Returns an iterator over all field names and values.
    ///
    /// # Examples
    ///
    /// ```
    /// use tasmor_lib::response::RoutineResponse;
    ///
    /// let json = r#"{"POWER":"ON","Dimmer":75}"#;
    /// let response: RoutineResponse = serde_json::from_str(json).unwrap();
    ///
    /// for (key, value) in response.iter() {
    ///     println!("{}: {}", key, value);
    /// }
    /// ```
    pub fn iter(&self) -> impl Iterator<Item = (&String, &serde_json::Value)> {
        self.fields.iter()
    }

    /// Returns the number of fields in the response.
    #[must_use]
    pub fn len(&self) -> usize {
        self.fields.len()
    }

    /// Returns `true` if the response has no fields.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.fields.is_empty()
    }

    /// Returns the underlying field map.
    ///
    /// This is useful for advanced use cases where you need direct access
    /// to the raw JSON values.
    #[must_use]
    pub fn raw(&self) -> &HashMap<String, serde_json::Value> {
        &self.fields
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_response() {
        let json = r#"{"POWER":"ON"}"#;
        let response: RoutineResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.len(), 1);
        assert!(response.contains_key("POWER"));
        assert!(!response.contains_key("Dimmer"));
    }

    #[test]
    fn parse_multi_field_response() {
        let json = r#"{"POWER1":"ON","POWER2":"OFF","Dimmer":75,"CT":350}"#;
        let response: RoutineResponse = serde_json::from_str(json).unwrap();

        assert_eq!(response.len(), 4);
        assert!(response.contains_key("POWER1"));
        assert!(response.contains_key("POWER2"));
        assert!(response.contains_key("Dimmer"));
        assert!(response.contains_key("CT"));
    }

    #[test]
    fn get_typed_string() {
        let json = r#"{"POWER":"ON"}"#;
        let response: RoutineResponse = serde_json::from_str(json).unwrap();

        let power: String = response.get_as("POWER").unwrap();
        assert_eq!(power, "ON");
    }

    #[test]
    fn get_typed_number() {
        let json = r#"{"Dimmer":75,"CT":350}"#;
        let response: RoutineResponse = serde_json::from_str(json).unwrap();

        let dimmer: u8 = response.get_as("Dimmer").unwrap();
        assert_eq!(dimmer, 75);

        let ct: u16 = response.get_as("CT").unwrap();
        assert_eq!(ct, 350);
    }

    #[test]
    fn get_missing_field_error() {
        let response = RoutineResponse::new();
        let result: Result<String, _> = response.get_as("Missing");

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), ParseError::MissingField(_)));
    }

    #[test]
    fn try_get_missing_returns_none() {
        let response = RoutineResponse::new();
        let result: Option<String> = response.try_get_as("Missing");

        assert!(result.is_none());
    }

    #[test]
    fn try_get_existing_returns_value() {
        let json = r#"{"Dimmer":75}"#;
        let response: RoutineResponse = serde_json::from_str(json).unwrap();

        let result: Option<u8> = response.try_get_as("Dimmer");
        assert_eq!(result, Some(75));
    }

    #[test]
    fn try_get_wrong_type_returns_none() {
        let json = r#"{"POWER":"ON"}"#;
        let response: RoutineResponse = serde_json::from_str(json).unwrap();

        // Try to parse "ON" as u8 - should fail
        let result: Option<u8> = response.try_get_as("POWER");
        assert!(result.is_none());
    }

    #[test]
    fn empty_response() {
        let json = r#"{}"#;
        let response: RoutineResponse = serde_json::from_str(json).unwrap();

        assert!(response.is_empty());
        assert_eq!(response.len(), 0);
    }

    #[test]
    fn iterate_over_fields() {
        let json = r#"{"POWER":"ON","Dimmer":75}"#;
        let response: RoutineResponse = serde_json::from_str(json).unwrap();

        let keys: Vec<&String> = response.iter().map(|(k, _)| k).collect();
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&&"POWER".to_string()));
        assert!(keys.contains(&&"Dimmer".to_string()));
    }

    #[test]
    fn raw_access() {
        let json = r#"{"POWER":"ON"}"#;
        let response: RoutineResponse = serde_json::from_str(json).unwrap();

        let raw = response.raw();
        assert!(raw.contains_key("POWER"));
    }

    #[test]
    fn response_is_cloneable() {
        let json = r#"{"POWER":"ON"}"#;
        let response: RoutineResponse = serde_json::from_str(json).unwrap();

        let cloned = response.clone();
        assert_eq!(response.len(), cloned.len());
    }

    #[test]
    fn response_is_serializable() {
        let json = r#"{"Dimmer":75,"POWER":"ON"}"#;
        let response: RoutineResponse = serde_json::from_str(json).unwrap();

        let serialized = serde_json::to_string(&response).unwrap();
        // Order might differ, so just check it's valid JSON with same fields
        let reparsed: RoutineResponse = serde_json::from_str(&serialized).unwrap();
        assert_eq!(response.len(), reparsed.len());
    }
}
