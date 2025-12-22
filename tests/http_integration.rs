// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Integration tests for HTTP protocol using wiremock.

use tasmor_lib::command::{
    ColorTempCommand, DimmerCommand, EnergyCommand, FadeCommand, HsbColorCommand, PowerCommand,
    PowerOnFadeCommand, SpeedCommand, StatusCommand,
};
use tasmor_lib::protocol::{HttpClient, HttpClientBuilder, Protocol};
use tasmor_lib::types::{ColorTemp, Dimmer, FadeSpeed, HsbColor, PowerIndex, PowerState};
use tasmor_lib::{Capabilities, Device};
use wiremock::matchers::{method, query_param, query_param_contains};
use wiremock::{Mock, MockServer, ResponseTemplate};

// ============================================================================
// HttpClient Tests
// ============================================================================

mod http_client {
    use super::*;

    #[tokio::test]
    async fn send_power_on_command() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param_contains("cmnd", "Power1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "POWER1": "ON"
            })))
            .mount(&mock_server)
            .await;

        let client = HttpClientBuilder::new()
            .host(mock_server.uri().replace("http://", ""))
            .build()
            .unwrap();

        let cmd = PowerCommand::Set {
            index: PowerIndex::one(),
            state: PowerState::On,
        };

        let response = client.send_command(&cmd).await.unwrap();
        assert!(response.body.contains("ON"));
    }

    #[tokio::test]
    async fn send_power_query_command() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Power1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "POWER1": "OFF"
            })))
            .mount(&mock_server)
            .await;

        let client = HttpClientBuilder::new()
            .host(mock_server.uri().replace("http://", ""))
            .build()
            .unwrap();

        let cmd = PowerCommand::Get {
            index: PowerIndex::one(),
        };

        let response = client.send_command(&cmd).await.unwrap();
        assert!(response.body.contains("OFF"));
    }

    #[tokio::test]
    async fn send_status_command() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Status 0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Status": {
                    "Module": 18,
                    "DeviceName": "Tasmota",
                    "FriendlyName": ["Light"],
                    "Topic": "tasmota",
                    "Power": 1
                },
                "StatusFWR": {
                    "Version": "13.1.0",
                    "BuildDateTime": "2024-01-01T00:00:00"
                },
                "StatusNET": {
                    "Hostname": "tasmota-device",
                    "IPAddress": "192.168.1.100"
                }
            })))
            .mount(&mock_server)
            .await;

        let client = HttpClientBuilder::new()
            .host(mock_server.uri().replace("http://", ""))
            .build()
            .unwrap();

        let cmd = StatusCommand::all();
        let response = client.send_command(&cmd).await.unwrap();

        assert!(response.body.contains("Tasmota"));
        assert!(response.body.contains("13.1.0"));
    }

    #[tokio::test]
    async fn send_dimmer_command() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Dimmer 75"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Dimmer": 75
            })))
            .mount(&mock_server)
            .await;

        let client = HttpClientBuilder::new()
            .host(mock_server.uri().replace("http://", ""))
            .build()
            .unwrap();

        let cmd = DimmerCommand::Set(Dimmer::new(75).unwrap());
        let response = client.send_command(&cmd).await.unwrap();

        assert!(response.body.contains("75"));
    }

    #[tokio::test]
    async fn send_color_temp_command() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "CT 250"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "CT": 250
            })))
            .mount(&mock_server)
            .await;

        let client = HttpClientBuilder::new()
            .host(mock_server.uri().replace("http://", ""))
            .build()
            .unwrap();

        let cmd = ColorTempCommand::Set(ColorTemp::new(250).unwrap());
        let response = client.send_command(&cmd).await.unwrap();

        assert!(response.body.contains("250"));
    }

    #[tokio::test]
    async fn send_hsb_color_command() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "HSBColor 120,100,80"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "HSBColor": "120,100,80"
            })))
            .mount(&mock_server)
            .await;

        let client = HttpClientBuilder::new()
            .host(mock_server.uri().replace("http://", ""))
            .build()
            .unwrap();

        let cmd = HsbColorCommand::Set(HsbColor::new(120, 100, 80).unwrap());
        let response = client.send_command(&cmd).await.unwrap();

        assert!(response.body.contains("120,100,80"));
    }

    #[tokio::test]
    async fn send_energy_command() {
        let mock_server = MockServer::start().await;

        // EnergyCommand::Get sends "Status 10" to get energy sensor data
        // (Status 10 replaces deprecated Status 8)
        Mock::given(method("GET"))
            .and(query_param("cmnd", "Status 10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "StatusSNS": {
                    "ENERGY": {
                        "TotalStartTime": "2024-01-01T00:00:00",
                        "Total": 123.456,
                        "Yesterday": 1.234,
                        "Today": 0.567,
                        "Power": 45,
                        "Voltage": 230,
                        "Current": 0.196
                    }
                }
            })))
            .mount(&mock_server)
            .await;

        let client = HttpClientBuilder::new()
            .host(mock_server.uri().replace("http://", ""))
            .build()
            .unwrap();

        let cmd = EnergyCommand::Get;
        let response = client.send_command(&cmd).await.unwrap();

        assert!(response.body.contains("45"));
        assert!(response.body.contains("230"));
    }

    #[tokio::test]
    async fn send_fade_command() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Fade 1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Fade": "ON"
            })))
            .mount(&mock_server)
            .await;

        let client = HttpClientBuilder::new()
            .host(mock_server.uri().replace("http://", ""))
            .build()
            .unwrap();

        let cmd = FadeCommand::Enable;
        let response = client.send_command(&cmd).await.unwrap();

        assert!(response.body.contains("ON"));
    }

    #[tokio::test]
    async fn send_speed_command() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Speed 20"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Speed": 20
            })))
            .mount(&mock_server)
            .await;

        let client = HttpClientBuilder::new()
            .host(mock_server.uri().replace("http://", ""))
            .build()
            .unwrap();

        let cmd = SpeedCommand::Set(FadeSpeed::new(20).unwrap());
        let response = client.send_command(&cmd).await.unwrap();

        assert!(response.body.contains("20"));
    }

    #[tokio::test]
    async fn send_power_on_fade_command() {
        let mock_server = MockServer::start().await;

        // PowerOnFadeCommand uses SetOption91 in Tasmota
        Mock::given(method("GET"))
            .and(query_param("cmnd", "SetOption91 1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "SetOption91": "ON"
            })))
            .mount(&mock_server)
            .await;

        let client = HttpClientBuilder::new()
            .host(mock_server.uri().replace("http://", ""))
            .build()
            .unwrap();

        let cmd = PowerOnFadeCommand::Enable;
        let response = client.send_command(&cmd).await.unwrap();

        assert!(response.body.contains("ON"));
    }

    #[tokio::test]
    async fn client_with_authentication() {
        let mock_server = MockServer::start().await;

        // The auth is passed as query params in Tasmota
        Mock::given(method("GET"))
            .and(query_param("user", "admin"))
            .and(query_param("password", "secret"))
            .and(query_param_contains("cmnd", "Power1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "POWER1": "ON"
            })))
            .mount(&mock_server)
            .await;

        let client = HttpClientBuilder::new()
            .host(mock_server.uri().replace("http://", ""))
            .credentials("admin", "secret")
            .build()
            .unwrap();

        let cmd = PowerCommand::Set {
            index: PowerIndex::one(),
            state: PowerState::On,
        };

        let response = client.send_command(&cmd).await.unwrap();
        assert!(response.body.contains("ON"));
    }
}

// ============================================================================
// Device with Auto-Detection Tests
// ============================================================================

mod device_auto_detection {
    use super::*;

    fn create_full_status_response() -> serde_json::Value {
        serde_json::json!({
            "Status": {
                "Module": 18,
                "DeviceName": "Tasmota RGB Bulb",
                "FriendlyName": ["Living Room Light"],
                "Topic": "tasmota_bulb",
                "Power": 1
            },
            "StatusFWR": {
                "Version": "13.1.0",
                "BuildDateTime": "2024-01-01T00:00:00"
            },
            "StatusNET": {
                "Hostname": "tasmota-bulb",
                "IPAddress": "192.168.1.100"
            }
        })
    }

    #[tokio::test]
    async fn build_device_with_auto_detection() {
        let mock_server = MockServer::start().await;

        // Mock Status 0 for capability detection
        Mock::given(method("GET"))
            .and(query_param("cmnd", "Status 0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(create_full_status_response()))
            .mount(&mock_server)
            .await;

        let host = mock_server.uri().replace("http://", "");
        let device = Device::http(&host).build().await.unwrap();

        assert_eq!(device.capabilities().power_channels, 1);
    }

    #[tokio::test]
    async fn build_device_detects_neo_coolcam() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Status 0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Status": {
                    "Module": 49,
                    "DeviceName": "Neo Coolcam Plug",
                    "FriendlyName": ["Smart Plug"],
                    "Topic": "tasmota_plug"
                },
                "StatusSTS": {
                    "ENERGY": {
                        "Power": 45,
                        "Voltage": 230
                    }
                }
            })))
            .mount(&mock_server)
            .await;

        let host = mock_server.uri().replace("http://", "");
        let device = Device::http(&host).build().await.unwrap();

        assert!(device.capabilities().energy);
    }

    #[tokio::test]
    async fn build_device_without_probe() {
        let mock_server = MockServer::start().await;

        // No Status mock needed - using manual capabilities
        let host = mock_server.uri().replace("http://", "");
        let device = Device::http(&host)
            .with_capabilities(Capabilities::rgbcct_light())
            .build_without_probe()
            .unwrap();

        assert!(device.capabilities().dimmer);
        assert!(device.capabilities().color_temp);
        assert!(device.capabilities().rgb);
    }
}

// ============================================================================
// Device Power Commands Tests
// ============================================================================

mod device_power_commands {
    use super::*;

    async fn create_device_with_mock(mock_server: &MockServer) -> Device<HttpClient> {
        let host = mock_server.uri().replace("http://", "");
        Device::http(&host)
            .with_capabilities(Capabilities::basic())
            .build_without_probe()
            .unwrap()
    }

    #[tokio::test]
    async fn power_on() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param_contains("cmnd", "Power1 ON"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "POWER1": "ON"
            })))
            .mount(&mock_server)
            .await;

        let device = create_device_with_mock(&mock_server).await;
        let response = device.power_on().await.unwrap();

        assert_eq!(response.first_power_state().unwrap(), PowerState::On);
    }

    #[tokio::test]
    async fn power_off() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param_contains("cmnd", "Power1 OFF"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "POWER1": "OFF"
            })))
            .mount(&mock_server)
            .await;

        let device = create_device_with_mock(&mock_server).await;
        let response = device.power_off().await.unwrap();

        assert_eq!(response.first_power_state().unwrap(), PowerState::Off);
    }

    #[tokio::test]
    async fn power_toggle() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param_contains("cmnd", "Power1 TOGGLE"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "POWER1": "ON"
            })))
            .mount(&mock_server)
            .await;

        let device = create_device_with_mock(&mock_server).await;
        let response = device.power_toggle().await.unwrap();

        assert_eq!(response.first_power_state().unwrap(), PowerState::On);
    }

    #[tokio::test]
    async fn power_query() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Power1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "POWER1": "OFF"
            })))
            .mount(&mock_server)
            .await;

        let device = create_device_with_mock(&mock_server).await;
        let response = device.get_power().await.unwrap();

        assert_eq!(response.first_power_state().unwrap(), PowerState::Off);
    }

    #[tokio::test]
    async fn set_power_specific_relay() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param_contains("cmnd", "Power2 ON"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "POWER2": "ON"
            })))
            .mount(&mock_server)
            .await;

        let host = mock_server.uri().replace("http://", "");
        let caps = tasmor_lib::CapabilitiesBuilder::new()
            .power_channels(4)
            .build();

        let device = Device::http(&host)
            .with_capabilities(caps)
            .build_without_probe()
            .unwrap();

        let response = device
            .set_power(PowerIndex::new(2).unwrap(), PowerState::On)
            .await
            .unwrap();

        assert_eq!(response.power_state(2).unwrap().unwrap(), PowerState::On);
    }
}

// ============================================================================
// Device Light Commands Tests
// ============================================================================

mod device_light_commands {
    use super::*;

    async fn create_light_device(mock_server: &MockServer) -> Device<HttpClient> {
        let host = mock_server.uri().replace("http://", "");
        Device::http(&host)
            .with_capabilities(Capabilities::rgbcct_light())
            .build_without_probe()
            .unwrap()
    }

    #[tokio::test]
    async fn set_dimmer() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Dimmer 75"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Dimmer": 75
            })))
            .mount(&mock_server)
            .await;

        let device = create_light_device(&mock_server).await;
        device.set_dimmer(Dimmer::new(75).unwrap()).await.unwrap();
    }

    #[tokio::test]
    async fn set_color_temp() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "CT 300"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "CT": 300
            })))
            .mount(&mock_server)
            .await;

        let device = create_light_device(&mock_server).await;
        device
            .set_color_temp(ColorTemp::new(300).unwrap())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn set_hsb_color() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "HSBColor 240,100,50"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "HSBColor": "240,100,50"
            })))
            .mount(&mock_server)
            .await;

        let device = create_light_device(&mock_server).await;
        device
            .set_hsb_color(HsbColor::new(240, 100, 50).unwrap())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn enable_fade() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Fade 1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Fade": "ON"
            })))
            .mount(&mock_server)
            .await;

        let device = create_light_device(&mock_server).await;
        device.enable_fade().await.unwrap();
    }

    #[tokio::test]
    async fn disable_fade() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Fade 0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Fade": "OFF"
            })))
            .mount(&mock_server)
            .await;

        let device = create_light_device(&mock_server).await;
        device.disable_fade().await.unwrap();
    }

    #[tokio::test]
    async fn set_speed() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Speed 15"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Speed": 15
            })))
            .mount(&mock_server)
            .await;

        let device = create_light_device(&mock_server).await;
        device.set_speed(FadeSpeed::new(15).unwrap()).await.unwrap();
    }

    #[tokio::test]
    async fn dimmer_fails_without_capability() {
        let mock_server = MockServer::start().await;

        let host = mock_server.uri().replace("http://", "");
        let device = Device::http(&host)
            .with_capabilities(Capabilities::basic()) // No dimmer
            .build_without_probe()
            .unwrap();

        let result = device.set_dimmer(Dimmer::new(50).unwrap()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn color_temp_fails_without_capability() {
        let mock_server = MockServer::start().await;

        let host = mock_server.uri().replace("http://", "");
        let device = Device::http(&host)
            .with_capabilities(Capabilities::rgb_light()) // No CCT
            .build_without_probe()
            .unwrap();

        let result = device.set_color_temp(ColorTemp::NEUTRAL).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn hsb_color_fails_without_capability() {
        let mock_server = MockServer::start().await;

        let host = mock_server.uri().replace("http://", "");
        let device = Device::http(&host)
            .with_capabilities(Capabilities::cct_light()) // No RGB
            .build_without_probe()
            .unwrap();

        let result = device.set_hsb_color(HsbColor::red()).await;
        assert!(result.is_err());
    }
}

// ============================================================================
// Device Energy Commands Tests
// ============================================================================

mod device_energy_commands {
    use super::*;

    async fn create_energy_device(mock_server: &MockServer) -> Device<HttpClient> {
        let host = mock_server.uri().replace("http://", "");
        Device::http(&host)
            .with_capabilities(Capabilities::neo_coolcam())
            .build_without_probe()
            .unwrap()
    }

    #[tokio::test]
    async fn get_energy() {
        let mock_server = MockServer::start().await;

        // Energy command uses "Status 10" in Tasmota (replaces deprecated Status 8)
        Mock::given(method("GET"))
            .and(query_param("cmnd", "Status 10"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "StatusSNS": {
                    "ENERGY": {
                        "TotalStartTime": "2024-01-01T00:00:00",
                        "Total": 123.456,
                        "Yesterday": 1.234,
                        "Today": 0.567,
                        "Power": 45,
                        "Voltage": 230,
                        "Current": 0.196
                    }
                }
            })))
            .mount(&mock_server)
            .await;

        let device = create_energy_device(&mock_server).await;
        let response = device.energy().await.unwrap();

        let energy = response.energy().unwrap();
        assert_eq!(energy.power, 45);
        assert_eq!(energy.voltage, 230);
    }

    #[tokio::test]
    async fn energy_fails_without_capability() {
        let mock_server = MockServer::start().await;

        let host = mock_server.uri().replace("http://", "");
        let device = Device::http(&host)
            .with_capabilities(Capabilities::basic()) // No energy
            .build_without_probe()
            .unwrap();

        let result = device.energy().await;
        assert!(result.is_err());
    }
}

// ============================================================================
// Device Status Commands Tests
// ============================================================================

mod device_status_commands {
    use super::*;

    #[tokio::test]
    async fn get_status() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .and(query_param("cmnd", "Status 0"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "Status": {
                    "Module": 18,
                    "DeviceName": "Test Device",
                    "FriendlyName": ["Light"],
                    "Topic": "tasmota"
                },
                "StatusFWR": {
                    "Version": "13.1.0"
                },
                "StatusNET": {
                    "Hostname": "tasmota",
                    "IPAddress": "192.168.1.100"
                }
            })))
            .mount(&mock_server)
            .await;

        let host = mock_server.uri().replace("http://", "");
        let device = Device::http(&host)
            .with_capabilities(Capabilities::basic())
            .build_without_probe()
            .unwrap();

        let status = device.status().await.unwrap();

        assert_eq!(status.module_id(), Some(18));
        assert_eq!(status.device_name(), Some("Test Device"));
        assert_eq!(status.firmware_version(), Some("13.1.0"));
        assert_eq!(status.ip_address(), Some("192.168.1.100"));
    }
}

// ============================================================================
// Error Handling Tests
// ============================================================================

mod error_handling {
    use super::*;

    #[tokio::test]
    async fn handles_server_error() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&mock_server)
            .await;

        let host = mock_server.uri().replace("http://", "");
        let device = Device::http(&host)
            .with_capabilities(Capabilities::basic())
            .build_without_probe()
            .unwrap();

        let result = device.power_on().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn handles_invalid_json_response() {
        let mock_server = MockServer::start().await;

        Mock::given(method("GET"))
            .respond_with(ResponseTemplate::new(200).set_body_string("not json"))
            .mount(&mock_server)
            .await;

        let host = mock_server.uri().replace("http://", "");
        let device = Device::http(&host)
            .with_capabilities(Capabilities::basic())
            .build_without_probe()
            .unwrap();

        let result = device.power_on().await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn handles_connection_refused() {
        // Use a port that's definitely not listening
        let device = Device::http("127.0.0.1:59999")
            .with_capabilities(Capabilities::basic())
            .build_without_probe()
            .unwrap();

        let result = device.power_on().await;
        assert!(result.is_err());
    }
}
