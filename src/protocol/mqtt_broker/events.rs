// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::sync::atomic::{AtomicU64, Ordering};

use tokio::sync::mpsc;

use super::broker::MqttBroker;

/// Events dispatched from paho-mqtt callbacks to the Tokio event loop.
pub(super) enum BrokerEvent {
    /// An incoming MQTT message was received.
    Message { topic: String, payload: String },
    /// The connection to the broker was lost.
    ConnectionLost,
    /// The connection to the broker was (re)established.
    Reconnected,
}

/// Forwards a broker event into the bounded bridge channel without blocking.
///
/// The paho callback runs on a synchronous C thread that must never block, so
/// this uses `try_send`. When the channel is full the event is dropped, the
/// drop counter is incremented, and a warning is logged — making backpressure
/// observable rather than silently growing memory (the unbounded alternative).
pub(super) fn try_forward(
    tx: &mpsc::Sender<BrokerEvent>,
    event: BrokerEvent,
    drop_counter: &AtomicU64,
) {
    if tx.try_send(event).is_err() {
        drop_counter.fetch_add(1, Ordering::Relaxed);
        tracing::warn!("MQTT broker event channel full — event dropped");
    }
}

/// Processes broker events forwarded from paho-mqtt's C-thread callbacks.
///
/// Runs for the lifetime of the broker, handling:
/// - Incoming messages → routed to subscribed devices
/// - Connection loss → `on_disconnected` callbacks dispatched
/// - Reconnection → topics resubscribed, `on_reconnected` callbacks dispatched
pub(super) async fn handle_broker_events(
    mut event_rx: mpsc::Receiver<BrokerEvent>,
    broker: MqttBroker,
) {
    while let Some(event) = event_rx.recv().await {
        match event {
            BrokerEvent::Reconnected => {
                broker.set_connected(true);
                tracing::info!("MQTT broker reconnected, restoring subscriptions");
                broker.handle_reconnection().await;
            }
            BrokerEvent::ConnectionLost => {
                let was_connected = broker.swap_connected(false);
                if was_connected {
                    tracing::warn!("MQTT connection lost, waiting for reconnection");
                    broker.dispatch_disconnected_all().await;
                }
            }
            BrokerEvent::Message { topic, payload } => {
                tracing::debug!(
                    topic = %topic,
                    payload = %payload,
                    "MQTT message received"
                );
                broker.route_message(&topic, payload).await;
            }
        }
    }
    tracing::info!("MQTT broker event loop ended");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_forward_succeeds_on_open_channel() {
        let (tx, _rx) = mpsc::channel(4);
        let counter = AtomicU64::new(0);

        try_forward(&tx, BrokerEvent::Reconnected, &counter);

        assert_eq!(counter.load(Ordering::Relaxed), 0);
    }

    #[test]
    fn try_forward_increments_counter_on_full_channel() {
        // Capacity 1, no consumer: first send fills it, second must be dropped.
        let (tx, _rx) = mpsc::channel(1);
        let counter = AtomicU64::new(0);

        try_forward(&tx, BrokerEvent::ConnectionLost, &counter);
        assert_eq!(counter.load(Ordering::Relaxed), 0);

        try_forward(&tx, BrokerEvent::Reconnected, &counter);
        assert_eq!(counter.load(Ordering::Relaxed), 1);
    }
}
