// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use zeroize::Zeroizing;

/// MQTT/HTTP authentication credentials.
///
/// Both username and password are stored in [`zeroize::Zeroizing`]-backed allocations —
/// the heap bytes are overwritten with zeros on `Drop`.
///
/// # Security
///
/// This provides best-effort mitigation of CWE-316 (cleartext storage in memory)
/// for Rust-owned copies. The underlying C libraries (`paho-mqtt`, `reqwest`) maintain
/// their own copies that are outside Rust's control.
#[derive(Clone)]
pub(crate) struct Credentials {
    username: Zeroizing<String>,
    password: Zeroizing<String>,
}

impl Credentials {
    pub(crate) fn new(username: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            username: Zeroizing::new(username.into()),
            password: Zeroizing::new(password.into()),
        }
    }

    pub(crate) fn username(&self) -> &str {
        self.username.as_str()
    }

    pub(crate) fn password(&self) -> &str {
        self.password.as_str()
    }
}

impl std::fmt::Debug for Credentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Credentials")
            .field("username", &"[REDACTED]")
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn credentials_accessors() {
        let creds = Credentials::new("admin", "secret");
        assert_eq!(creds.username(), "admin");
        assert_eq!(creds.password(), "secret");
    }

    #[test]
    fn credentials_debug_redacts_password() {
        let creds = Credentials::new("admin", "super_secret_password");
        let debug_output = format!("{creds:?}");
        assert!(!debug_output.contains("super_secret_password"));
        assert!(!debug_output.contains("admin"));
        assert!(debug_output.contains("[REDACTED]"));
    }

    #[test]
    fn credentials_clone() {
        let creds = Credentials::new("user", "pass");
        let cloned = creds.clone();
        assert_eq!(cloned.username(), "user");
        assert_eq!(cloned.password(), "pass");
    }
}
