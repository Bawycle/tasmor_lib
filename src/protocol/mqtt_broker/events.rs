// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

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

/// Processes broker events forwarded from paho-mqtt's C-thread callbacks.
///
/// Runs for the lifetime of the broker, handling:
/// - Incoming messages → routed to subscribed devices
/// - Connection loss → `on_disconnected` callbacks dispatched
/// - Reconnection → topics resubscribed, `on_reconnected` callbacks dispatched
pub(super) async fn handle_broker_events(
    mut event_rx: mpsc::UnboundedReceiver<BrokerEvent>,
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
