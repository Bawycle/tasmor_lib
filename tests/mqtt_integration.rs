// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Integration tests for MQTT protocol using mockforge-mqtt.

use std::time::Duration;

use mockforge_mqtt::broker::MqttConfig;
use mockforge_mqtt::start_mqtt_server;
use tasmor_lib::Capabilities;
use tasmor_lib::Device;
use tasmor_lib::protocol::MqttClient;
use tokio::time::sleep;

/// Helper to find an available port for testing.
fn get_test_port() -> u16 {
    use std::sync::atomic::{AtomicU16, Ordering};
    static PORT_COUNTER: AtomicU16 = AtomicU16::new(18850);
    PORT_COUNTER.fetch_add(1, Ordering::SeqCst)
}

/// Starts a mock MQTT broker on the given port.
async fn start_mock_broker(port: u16) {
    let config = MqttConfig {
        port,
        host: "127.0.0.1".to_string(),
        ..Default::default()
    };

    tokio::spawn(async move {
        let _ = start_mqtt_server(config).await;
    });

    // Give the broker time to start, bind to port, and be ready to accept connections
    sleep(Duration::from_millis(500)).await;
}

// ============================================================================
// MqttClient Connection Tests
// ============================================================================

mod mqtt_client_connection {
    use super::*;

    #[tokio::test]
    async fn connect_to_broker() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("mqtt://127.0.0.1:{port}");
        let result = MqttClient::connect(&broker_url, "tasmota_test").await;

        assert!(result.is_ok(), "Failed to connect: {:?}", result.err());

        let client = result.unwrap();
        assert_eq!(client.topic(), "tasmota_test");
    }

    #[tokio::test]
    async fn connect_with_tcp_scheme() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("tcp://127.0.0.1:{port}");
        let result = MqttClient::connect(&broker_url, "device_topic").await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn connect_without_scheme() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("127.0.0.1:{port}");
        let result = MqttClient::connect(&broker_url, "my_device").await;

        assert!(result.is_ok());
    }
}

// ============================================================================
// Device MQTT Tests
// ============================================================================

mod device_mqtt {
    use super::*;

    #[tokio::test]
    async fn create_mqtt_device_without_probe() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("mqtt://127.0.0.1:{port}");
        let result = Device::mqtt(&broker_url, "tasmota_bulb")
            .with_capabilities(Capabilities::rgbcct_light())
            .build_without_probe()
            .await;

        assert!(result.is_ok());

        let (device, _initial_state) = result.unwrap();
        assert!(device.capabilities().supports_dimmer_control());
        assert!(device.capabilities().supports_rgb_control());
        assert!(device.capabilities().supports_color_temperature_control());
    }

    #[tokio::test]
    async fn create_mqtt_device_neo_coolcam() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("mqtt://127.0.0.1:{port}");
        let result = Device::mqtt(&broker_url, "tasmota_plug")
            .with_capabilities(Capabilities::neo_coolcam())
            .build_without_probe()
            .await;

        assert!(result.is_ok());

        let (device, _initial_state) = result.unwrap();
        assert!(device.capabilities().supports_energy_monitoring());
        assert!(!device.capabilities().supports_dimmer_control());
    }

    #[tokio::test]
    async fn create_mqtt_device_basic() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("mqtt://127.0.0.1:{port}");
        let result = Device::mqtt(&broker_url, "tasmota_switch")
            .with_capabilities(Capabilities::basic())
            .build_without_probe()
            .await;

        assert!(result.is_ok());

        let (device, _initial_state) = result.unwrap();
        assert_eq!(device.capabilities().power_channels(), 1);
        assert!(!device.capabilities().supports_dimmer_control());
    }
}

// ============================================================================
// MqttClientBuilder Tests
// ============================================================================

mod mqtt_client_builder {
    use super::*;
    use tasmor_lib::protocol::MqttClientBuilder;

    #[tokio::test]
    async fn build_with_broker_and_topic() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("mqtt://127.0.0.1:{port}");
        let client = MqttClientBuilder::new()
            .broker(&broker_url)
            .device_topic("test_device")
            .build()
            .await;

        assert!(client.is_ok());
        assert_eq!(client.unwrap().topic(), "test_device");
    }

    #[tokio::test]
    async fn build_missing_broker_fails() {
        let result = MqttClientBuilder::new()
            .device_topic("test_device")
            .build()
            .await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn build_missing_topic_fails() {
        let result = MqttClientBuilder::new()
            .broker("mqtt://localhost:1883")
            .build()
            .await;

        assert!(result.is_err());
    }
}

// ============================================================================
// URL Parsing Tests (via connection)
// ============================================================================

mod url_parsing {
    use super::*;

    #[tokio::test]
    async fn parse_mqtt_url_with_port() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("mqtt://127.0.0.1:{port}");
        let result = MqttClient::connect(&broker_url, "test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn parse_tcp_url_with_port() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("tcp://127.0.0.1:{port}");
        let result = MqttClient::connect(&broker_url, "test").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn parse_bare_host_with_port() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("127.0.0.1:{port}");
        let result = MqttClient::connect(&broker_url, "test").await;
        assert!(result.is_ok());
    }
}

// ============================================================================
// Capability Verification Tests
// ============================================================================

mod capability_verification {
    use super::*;

    #[tokio::test]
    async fn dimmer_requires_capability() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("mqtt://127.0.0.1:{port}");
        let (device, _) = Device::mqtt(&broker_url, "switch")
            .with_capabilities(Capabilities::basic()) // No dimmer
            .build_without_probe()
            .await
            .unwrap();

        let result = device
            .set_dimmer(tasmor_lib::Dimmer::new(50).unwrap())
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn color_temp_requires_capability() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("mqtt://127.0.0.1:{port}");
        let (device, _) = Device::mqtt(&broker_url, "switch")
            .with_capabilities(Capabilities::rgb_light()) // No CCT
            .build_without_probe()
            .await
            .unwrap();

        let result = device
            .set_color_temperature(tasmor_lib::ColorTemperature::NEUTRAL)
            .await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn rgb_requires_capability() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("mqtt://127.0.0.1:{port}");
        let (device, _) = Device::mqtt(&broker_url, "switch")
            .with_capabilities(Capabilities::cct_light()) // No RGB
            .build_without_probe()
            .await
            .unwrap();

        let result = device.set_hsb_color(tasmor_lib::HsbColor::red()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn energy_requires_capability() {
        let port = get_test_port();
        start_mock_broker(port).await;

        let broker_url = format!("mqtt://127.0.0.1:{port}");
        let (device, _) = Device::mqtt(&broker_url, "bulb")
            .with_capabilities(Capabilities::rgbcct_light()) // No energy
            .build_without_probe()
            .await
            .unwrap();

        let result = device.energy().await;
        assert!(result.is_err());
    }
}

// ============================================================================
// MQTT Callback Tests
// ============================================================================
//
// NOTE: The mockforge-mqtt broker used for testing doesn't fully support
// pub/sub message forwarding between clients. The callback routing logic
// is tested via unit tests in:
//   - src/protocol/topic_router.rs (TopicRouter tests)
//   - src/subscription/callback.rs (CallbackRegistry tests)
//
// For full integration testing with callbacks, use a real MQTT broker
// like Mosquitto.
