// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Event bus for broadcasting device events.

use tokio::sync::broadcast;

use super::DeviceEvent;

/// Default channel capacity for the event bus.
const DEFAULT_CHANNEL_CAPACITY: usize = 256;

/// Event bus for broadcasting device events to multiple subscribers.
///
/// The `EventBus` uses tokio's broadcast channel to allow multiple
/// subscribers to receive the same events. Each subscriber gets their
/// own copy of each event.
///
/// # Capacity
///
/// The event bus has a fixed capacity (default 256). If the channel fills
/// up because a subscriber is slow, older events may be dropped for that
/// subscriber (they will receive a `RecvError::Lagged` error).
///
/// # Examples
///
/// ```
/// use tasmor_lib::event::{DeviceId, DeviceEvent, EventBus};
///
/// let bus = EventBus::new();
///
/// // Create a subscriber
/// let mut rx = bus.subscribe();
///
/// // Publish an event
/// bus.publish(DeviceEvent::DeviceAdded {
///     device_id: DeviceId::new(),
/// });
///
/// // Multiple subscribers can exist
/// let mut rx2 = bus.subscribe();
/// ```
#[derive(Debug)]
pub struct EventBus {
    sender: broadcast::Sender<DeviceEvent>,
}

impl EventBus {
    /// Creates a new event bus with default capacity.
    #[must_use]
    pub fn new() -> Self {
        Self::with_capacity(DEFAULT_CHANNEL_CAPACITY)
    }

    /// Creates a new event bus with the specified capacity.
    ///
    /// # Arguments
    ///
    /// * `capacity` - Maximum number of events that can be buffered
    #[must_use]
    pub fn with_capacity(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// Subscribes to device events.
    ///
    /// Returns a receiver that will receive all events published after
    /// the subscription is created.
    #[must_use]
    pub fn subscribe(&self) -> broadcast::Receiver<DeviceEvent> {
        self.sender.subscribe()
    }

    /// Returns the number of active subscribers.
    #[must_use]
    pub fn subscriber_count(&self) -> usize {
        self.sender.receiver_count()
    }

    /// Publishes an event to all subscribers.
    ///
    /// If there are no subscribers, the event is silently discarded.
    /// If the channel is full for a slow subscriber, that subscriber
    /// will lose events.
    pub fn publish(&self, event: DeviceEvent) {
        // Ignore errors (no subscribers or channel closed)
        let _ = self.sender.send(event);
    }

    /// Publishes an event and returns the number of receivers that received it.
    ///
    /// Returns 0 if there are no subscribers.
    #[must_use]
    pub fn publish_counted(&self, event: DeviceEvent) -> usize {
        self.sender.send(event).unwrap_or(0)
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventBus {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::DeviceId;

    #[test]
    fn new_bus_has_no_subscribers() {
        let bus = EventBus::new();
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[test]
    fn subscribe_increments_count() {
        let bus = EventBus::new();

        let _rx1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        let _rx2 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 2);
    }

    #[test]
    fn drop_subscriber_decrements_count() {
        let bus = EventBus::new();

        let rx1 = bus.subscribe();
        assert_eq!(bus.subscriber_count(), 1);

        drop(rx1);
        assert_eq!(bus.subscriber_count(), 0);
    }

    #[tokio::test]
    async fn publish_delivers_to_subscriber() {
        let bus = EventBus::new();
        let mut rx = bus.subscribe();

        let device_id = DeviceId::new();
        bus.publish(DeviceEvent::device_added(device_id));

        let event = rx.recv().await.unwrap();
        assert_eq!(event.device_id(), device_id);
    }

    #[tokio::test]
    async fn publish_delivers_to_multiple_subscribers() {
        let bus = EventBus::new();
        let mut rx1 = bus.subscribe();
        let mut rx2 = bus.subscribe();

        let device_id = DeviceId::new();
        bus.publish(DeviceEvent::device_added(device_id));

        let event1 = rx1.recv().await.unwrap();
        let event2 = rx2.recv().await.unwrap();

        assert_eq!(event1.device_id(), device_id);
        assert_eq!(event2.device_id(), device_id);
    }

    #[test]
    fn publish_counted_returns_receiver_count() {
        let bus = EventBus::new();
        let _rx1 = bus.subscribe();
        let _rx2 = bus.subscribe();

        let device_id = DeviceId::new();
        let count = bus.publish_counted(DeviceEvent::device_added(device_id));

        assert_eq!(count, 2);
    }

    #[test]
    fn publish_counted_returns_zero_without_subscribers() {
        let bus = EventBus::new();
        let device_id = DeviceId::new();
        let count = bus.publish_counted(DeviceEvent::device_added(device_id));

        assert_eq!(count, 0);
    }

    #[test]
    fn clone_shares_same_channel() {
        let bus1 = EventBus::new();
        let bus2 = bus1.clone();

        let _rx = bus1.subscribe();
        // Subscriber from bus1 should be visible in bus2
        assert_eq!(bus2.subscriber_count(), 1);
    }

    #[test]
    fn with_capacity_creates_bus() {
        let bus = EventBus::with_capacity(512);
        assert_eq!(bus.subscriber_count(), 0);
    }
}
