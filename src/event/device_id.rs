// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Device identifier type.

use std::fmt;

use uuid::Uuid;

/// Unique identifier for a managed device.
///
/// This is a wrapper around UUID v4 that provides a distinct type for
/// device identification, preventing accidental confusion with other
/// UUID-based identifiers.
///
/// # Examples
///
/// ```
/// use tasmor_lib::event::DeviceId;
///
/// let id = DeviceId::new();
/// println!("Device: {}", id);
/// ```
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DeviceId(Uuid);

impl DeviceId {
    /// Creates a new unique device identifier.
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// Creates a device identifier from an existing UUID.
    #[must_use]
    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the underlying UUID.
    #[must_use]
    pub fn as_uuid(&self) -> Uuid {
        self.0
    }

    /// Returns the UUID as a hyphenated string.
    #[must_use]
    pub fn to_string_hyphenated(&self) -> String {
        self.0.to_string()
    }
}

impl Default for DeviceId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Debug for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Show only first 8 characters for readability
        let short = &self.0.to_string()[..8];
        write!(f, "DeviceId({short}...)")
    }
}

impl fmt::Display for DeviceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<Uuid> for DeviceId {
    fn from(uuid: Uuid) -> Self {
        Self(uuid)
    }
}

impl From<DeviceId> for Uuid {
    fn from(id: DeviceId) -> Self {
        id.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_creates_unique_ids() {
        let id1 = DeviceId::new();
        let id2 = DeviceId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn from_uuid_round_trip() {
        let uuid = Uuid::new_v4();
        let id = DeviceId::from_uuid(uuid);
        assert_eq!(id.as_uuid(), uuid);
    }

    #[test]
    fn equality() {
        let uuid = Uuid::new_v4();
        let id1 = DeviceId::from_uuid(uuid);
        let id2 = DeviceId::from_uuid(uuid);
        assert_eq!(id1, id2);
    }

    #[test]
    fn debug_format() {
        let id = DeviceId::new();
        let debug = format!("{id:?}");
        assert!(debug.starts_with("DeviceId("));
        assert!(debug.ends_with("...)"));
    }

    #[test]
    fn display_format() {
        let uuid = Uuid::parse_str("a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8").unwrap();
        let id = DeviceId::from_uuid(uuid);
        assert_eq!(id.to_string(), "a1a2a3a4-b1b2-c1c2-d1d2-d3d4d5d6d7d8");
    }

    #[test]
    fn hashable() {
        use std::collections::HashSet;

        let mut set = HashSet::new();
        let id = DeviceId::new();
        set.insert(id);
        assert!(set.contains(&id));
    }
}
