// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Response collection and aggregation for multi-message MQTT responses.
//!
//! Some Tasmota commands (notably `Status 0`) respond with multiple MQTT messages.
//! This module provides the infrastructure to collect and merge these messages
//! into a single response.
//!
//! # Overview
//!
//! When a command like `Status 0` is sent via MQTT, Tasmota responds with multiple
//! separate messages (STATUS, STATUS1, STATUS2, ..., STATUS11). Each message is
//! published to a different topic suffix (e.g., `stat/<topic>/STATUS5`).
//!
//! The [`ResponseSpec`] type describes what responses a command expects. This is
//! used by the [`Command`](crate::command::Command) trait's `response_spec()` method.
//!
//! # Public API
//!
//! Only [`ResponseSpec`] is part of the public API. It is needed by users who
//! implement custom commands via the [`Command`](crate::command::Command) trait.
//! All other types in this module are internal implementation details.

use std::collections::HashSet;
use std::time::Duration;

use serde_json::Value;
use tokio::sync::mpsc;
use tokio::time::Instant;

use crate::error::ProtocolError;

/// Specification of expected responses from a command.
///
/// Most commands expect a single response, but some (like `Status 0`) expect
/// multiple messages that should be aggregated into a single response.
#[derive(Debug, Clone, Default)]
pub enum ResponseSpec {
    /// A single response is expected.
    ///
    /// This is the default for most commands.
    #[default]
    Single,

    /// Multiple responses are expected, identified by topic suffixes.
    ///
    /// The responses will be collected and merged into a single JSON object.
    /// Collection continues until all expected topics are received or timeout.
    Multiple {
        /// Expected topic suffixes (e.g., `["STATUS", "STATUS1", "STATUS5"]`).
        expected_topics: Vec<String>,
        /// Maximum time to wait for all responses.
        /// If not all topics arrive within this duration, returns what was collected.
        timeout: Duration,
    },
}

impl ResponseSpec {
    /// Creates a spec for a single response (the default).
    #[must_use]
    pub const fn single() -> Self {
        Self::Single
    }

    /// Creates a spec for multiple responses with specific topic suffixes.
    #[must_use]
    pub fn multiple(expected_topics: Vec<String>, timeout: Duration) -> Self {
        Self::Multiple {
            expected_topics,
            timeout,
        }
    }

    /// Creates a spec for Status 0 which returns multiple STATUS* messages.
    ///
    /// Status 0 returns messages on these topic suffixes:
    /// - STATUS (device name, friendly names)
    /// - STATUS1 (device parameters)
    /// - STATUS2 (firmware info)
    /// - STATUS3 (logging settings)
    /// - STATUS4 (memory info)
    /// - STATUS5 (network info)
    /// - STATUS6 (MQTT settings)
    /// - STATUS7 (time info)
    /// - STATUS10 (sensor info) - optional, only if sensors present
    /// - STATUS11 (state info with uptime)
    #[must_use]
    pub fn status_all(timeout: Duration) -> Self {
        Self::Multiple {
            expected_topics: vec![
                "STATUS".to_string(),
                "STATUS1".to_string(),
                "STATUS2".to_string(),
                "STATUS3".to_string(),
                "STATUS4".to_string(),
                "STATUS5".to_string(),
                "STATUS6".to_string(),
                "STATUS7".to_string(),
                // STATUS8 and STATUS9 are deprecated/unused
                // STATUS10 is optional (sensors)
                "STATUS11".to_string(),
            ],
            timeout,
        }
    }

    /// Returns true if this spec expects multiple responses.
    #[must_use]
    pub const fn is_multiple(&self) -> bool {
        matches!(self, Self::Multiple { .. })
    }
}

/// An MQTT message with its topic suffix for routing.
///
/// This is an internal type used for communication between the MQTT broker
/// and shared client. Not part of the public API.
#[derive(Debug, Clone)]
pub(crate) struct MqttMessage {
    /// The topic suffix (e.g., "STATUS5", "RESULT").
    pub(crate) topic_suffix: String,
    /// The JSON payload.
    pub(crate) payload: String,
}

impl MqttMessage {
    /// Creates a new MQTT message.
    #[must_use]
    pub(crate) fn new(topic_suffix: String, payload: String) -> Self {
        Self {
            topic_suffix,
            payload,
        }
    }
}

/// Collects multiple MQTT messages and merges them into a single response.
///
/// This is an internal type. Not part of the public API.
struct ResponseCollector {
    /// Expected topic suffixes to collect.
    expected: HashSet<String>,
    /// Collected messages indexed by topic suffix.
    collected: Vec<(String, Value)>,
    /// Deadline for collection.
    deadline: Instant,
}

impl ResponseCollector {
    /// Creates a new collector for the given response spec.
    ///
    /// # Panics
    ///
    /// Panics if called with `ResponseSpec::Single`.
    #[must_use]
    fn new(spec: &ResponseSpec) -> Self {
        match spec {
            ResponseSpec::Single => {
                panic!("ResponseCollector should not be used for single responses")
            }
            ResponseSpec::Multiple {
                expected_topics,
                timeout,
            } => Self {
                expected: expected_topics.iter().cloned().collect(),
                collected: Vec::with_capacity(expected_topics.len()),
                deadline: Instant::now() + *timeout,
            },
        }
    }

    /// Returns the remaining time until the deadline.
    #[must_use]
    fn remaining_time(&self) -> Duration {
        self.deadline.saturating_duration_since(Instant::now())
    }

    /// Returns true if the deadline has been reached.
    #[must_use]
    fn is_timed_out(&self) -> bool {
        Instant::now() >= self.deadline
    }

    /// Returns true if all expected messages have been collected.
    #[must_use]
    fn is_complete(&self) -> bool {
        self.expected.is_empty()
    }

    /// Processes a received message.
    ///
    /// Returns `true` if the message was expected and collected.
    fn process_message(&mut self, msg: &MqttMessage) -> bool {
        if self.expected.remove(&msg.topic_suffix)
            && let Ok(value) = serde_json::from_str::<Value>(&msg.payload)
        {
            self.collected.push((msg.topic_suffix.clone(), value));
            return true;
        }
        false
    }

    /// Merges all collected messages into a single JSON object.
    ///
    /// The resulting JSON contains all top-level keys from each collected message,
    /// merged into a single object.
    #[must_use]
    fn merge_responses(self) -> String {
        let mut merged = serde_json::Map::new();

        for (_, value) in self.collected {
            if let Value::Object(obj) = value {
                for (key, val) in obj {
                    merged.insert(key, val);
                }
            }
        }

        Value::Object(merged).to_string()
    }

    /// Returns the number of messages still expected.
    #[must_use]
    fn pending_count(&self) -> usize {
        self.expected.len()
    }

    /// Returns the number of messages collected so far.
    #[must_use]
    fn collected_count(&self) -> usize {
        self.collected.len()
    }
}

/// Collects responses according to the given spec.
///
/// For `ResponseSpec::Single`, waits for one message.
/// For `ResponseSpec::Multiple`, collects messages until complete or timeout.
///
/// # Errors
///
/// Returns `ProtocolError::Timeout` if no message arrives within the timeout.
/// For multiple responses, returns partial results if some messages arrive.
pub(crate) async fn collect_responses(
    rx: &mut mpsc::Receiver<MqttMessage>,
    spec: &ResponseSpec,
    single_timeout: Duration,
) -> Result<String, ProtocolError> {
    match spec {
        ResponseSpec::Single => {
            // Wait for a single message
            #[allow(clippy::cast_possible_truncation)]
            let timeout_ms = single_timeout.as_millis() as u64;

            let msg = tokio::time::timeout(single_timeout, rx.recv())
                .await
                .map_err(|_| ProtocolError::Timeout(timeout_ms))?
                .ok_or_else(|| {
                    ProtocolError::ConnectionFailed("Response channel closed".to_string())
                })?;

            Ok(msg.payload)
        }
        ResponseSpec::Multiple { .. } => {
            let mut collector = ResponseCollector::new(spec);

            while !collector.is_complete() && !collector.is_timed_out() {
                let remaining = collector.remaining_time();

                match tokio::time::timeout(remaining, rx.recv()).await {
                    Ok(Some(msg)) => {
                        tracing::trace!(
                            topic = %msg.topic_suffix,
                            collected = collector.collected_count(),
                            pending = collector.pending_count(),
                            "Received response message"
                        );
                        collector.process_message(&msg);
                    }
                    Ok(None) => {
                        // Channel closed
                        break;
                    }
                    Err(_) => {
                        // Timeout
                        tracing::debug!(
                            collected = collector.collected_count(),
                            pending = collector.pending_count(),
                            "Response collection timed out"
                        );
                        break;
                    }
                }
            }

            if collector.collected_count() == 0 {
                #[allow(clippy::cast_possible_truncation)]
                let timeout_ms = match spec {
                    ResponseSpec::Multiple { timeout, .. } => timeout.as_millis() as u64,
                    ResponseSpec::Single => single_timeout.as_millis() as u64,
                };
                return Err(ProtocolError::Timeout(timeout_ms));
            }

            Ok(collector.merge_responses())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn response_spec_default_is_single() {
        let spec = ResponseSpec::default();
        assert!(!spec.is_multiple());
    }

    #[test]
    fn response_spec_multiple() {
        let spec = ResponseSpec::multiple(
            vec!["STATUS".to_string(), "STATUS1".to_string()],
            Duration::from_secs(5),
        );
        assert!(spec.is_multiple());
    }

    #[test]
    fn response_spec_status_all() {
        let spec = ResponseSpec::status_all(Duration::from_secs(5));
        if let ResponseSpec::Multiple {
            expected_topics, ..
        } = spec
        {
            assert!(expected_topics.contains(&"STATUS".to_string()));
            assert!(expected_topics.contains(&"STATUS11".to_string()));
            assert!(!expected_topics.contains(&"STATUS10".to_string())); // Optional, not included by default
        } else {
            panic!("Expected Multiple variant");
        }
    }

    #[test]
    fn collector_processes_expected_message() {
        let spec = ResponseSpec::multiple(
            vec!["STATUS".to_string(), "STATUS1".to_string()],
            Duration::from_secs(5),
        );
        let mut collector = ResponseCollector::new(&spec);

        let msg = MqttMessage::new(
            "STATUS".to_string(),
            r#"{"Status":{"Topic":"test"}}"#.to_string(),
        );
        assert!(collector.process_message(&msg));
        assert_eq!(collector.collected_count(), 1);
        assert_eq!(collector.pending_count(), 1);
    }

    #[test]
    fn collector_ignores_unexpected_message() {
        let spec = ResponseSpec::multiple(vec!["STATUS".to_string()], Duration::from_secs(5));
        let mut collector = ResponseCollector::new(&spec);

        let msg = MqttMessage::new("RESULT".to_string(), r#"{"POWER":"ON"}"#.to_string());
        assert!(!collector.process_message(&msg));
        assert_eq!(collector.collected_count(), 0);
    }

    #[test]
    fn collector_merges_responses() {
        let spec = ResponseSpec::multiple(
            vec!["STATUS".to_string(), "STATUS11".to_string()],
            Duration::from_secs(5),
        );
        let mut collector = ResponseCollector::new(&spec);

        let msg1 = MqttMessage::new(
            "STATUS".to_string(),
            r#"{"Status":{"Topic":"test"}}"#.to_string(),
        );
        let msg2 = MqttMessage::new(
            "STATUS11".to_string(),
            r#"{"StatusSTS":{"UptimeSec":12345}}"#.to_string(),
        );

        collector.process_message(&msg1);
        collector.process_message(&msg2);

        let merged = collector.merge_responses();
        let value: Value = serde_json::from_str(&merged).unwrap();

        assert!(value.get("Status").is_some());
        assert!(value.get("StatusSTS").is_some());
    }

    #[test]
    fn collector_is_complete_when_all_collected() {
        let spec = ResponseSpec::multiple(vec!["STATUS".to_string()], Duration::from_secs(5));
        let mut collector = ResponseCollector::new(&spec);

        assert!(!collector.is_complete());

        let msg = MqttMessage::new("STATUS".to_string(), r#"{"Status":{}}"#.to_string());
        collector.process_message(&msg);

        assert!(collector.is_complete());
    }

    #[tokio::test]
    async fn collect_single_response() {
        let (tx, mut rx) = mpsc::channel(10);

        tx.send(MqttMessage::new(
            "RESULT".to_string(),
            r#"{"POWER":"ON"}"#.to_string(),
        ))
        .await
        .unwrap();

        let result = collect_responses(&mut rx, &ResponseSpec::Single, Duration::from_secs(1))
            .await
            .unwrap();

        assert_eq!(result, r#"{"POWER":"ON"}"#);
    }

    #[tokio::test]
    async fn collect_multiple_responses() {
        let (tx, mut rx) = mpsc::channel(10);
        let spec = ResponseSpec::multiple(
            vec!["STATUS".to_string(), "STATUS11".to_string()],
            Duration::from_secs(5),
        );

        // Send messages in background
        tokio::spawn(async move {
            tx.send(MqttMessage::new(
                "STATUS".to_string(),
                r#"{"Status":{"Topic":"test"}}"#.to_string(),
            ))
            .await
            .unwrap();

            tokio::time::sleep(Duration::from_millis(10)).await;

            tx.send(MqttMessage::new(
                "STATUS11".to_string(),
                r#"{"StatusSTS":{"UptimeSec":100}}"#.to_string(),
            ))
            .await
            .unwrap();
        });

        let result = collect_responses(&mut rx, &spec, Duration::from_secs(1))
            .await
            .unwrap();

        let value: Value = serde_json::from_str(&result).unwrap();
        assert!(value.get("Status").is_some());
        assert!(value.get("StatusSTS").is_some());
    }

    #[tokio::test]
    async fn collect_partial_on_timeout() {
        let (tx, mut rx) = mpsc::channel(10);
        let spec = ResponseSpec::multiple(
            vec!["STATUS".to_string(), "STATUS11".to_string()],
            Duration::from_millis(100),
        );

        // Send only one message
        tx.send(MqttMessage::new(
            "STATUS".to_string(),
            r#"{"Status":{"Topic":"test"}}"#.to_string(),
        ))
        .await
        .unwrap();

        let result = collect_responses(&mut rx, &spec, Duration::from_secs(1))
            .await
            .unwrap();

        let value: Value = serde_json::from_str(&result).unwrap();
        assert!(value.get("Status").is_some());
        // STATUS11 not received, but we still get partial result
        assert!(value.get("StatusSTS").is_none());
    }
}
