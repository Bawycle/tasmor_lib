// SPDX-License-Identifier: MPL-2.0
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Integration tests against real Tasmota devices.
//!
//! These tests require real devices on the network and are ignored by default.
//! Run with: `cargo test --test real_devices -- --ignored --test-threads=1`
//!
//! # Environment Variables
//!
//! Required environment variables:
//!
//! ## MQTT Broker
//! - `MQTT_BROKER_IP` - Broker IP address
//! - `MQTT_BROKER_PORT` - Broker port (default: 1883)
//! - `MQTT_USER` - MQTT username
//! - `MQTT_PASSWORD` - MQTT password
//!
//! ## Devices (LIGHT_1, LIGHT_2, LIGHT_3, PLUG_1, PLUG_2)
//! For each device, set:
//! - `{DEVICE}_HTTP_IP` - Device IP address
//! - `{DEVICE}_HTTP_USER` - HTTP username
//! - `{DEVICE}_HTTP_PASSWORD` - HTTP password
//! - `{DEVICE}_MQTT_TOPIC` - MQTT topic
//!
//! # Example
//!
//! ```bash
//! export MQTT_BROKER_IP=192.168.1.100
//! export MQTT_BROKER_PORT=1883
//! export MQTT_USER=mqtt
//! export MQTT_PASSWORD=secret
//! export LIGHT_1_HTTP_IP=192.168.1.50
//! export LIGHT_1_HTTP_USER=admin
//! export LIGHT_1_HTTP_PASSWORD=password
//! export LIGHT_1_MQTT_TOPIC=tasmota_ABC123
//! # ... other devices ...
//! cargo test --test real_devices -- --ignored --test-threads=1
//! ```

use std::env;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use tasmor_lib::subscription::Subscribable;
use tasmor_lib::types::{
    ColorTemperature, Dimmer, FadeSpeed, HsbColor, PowerIndex, PowerState, RgbColor, Scheme,
    WakeupDuration,
};
use tasmor_lib::{Capabilities, Device, MqttBroker};
use tokio::time::sleep;

// =============================================================================
// Test Configuration from Environment Variables
// =============================================================================

/// MQTT Broker configuration loaded from environment variables.
struct BrokerConfig {
    ip: String,
    port: u16,
    user: String,
    password: String,
}

impl BrokerConfig {
    fn from_env() -> Self {
        Self {
            ip: env::var("MQTT_BROKER_IP").expect("MQTT_BROKER_IP not set"),
            port: env::var("MQTT_BROKER_PORT")
                .unwrap_or_else(|_| "1883".to_string())
                .parse()
                .expect("Invalid MQTT_BROKER_PORT"),
            user: env::var("MQTT_USER").expect("MQTT_USER not set"),
            password: env::var("MQTT_PASSWORD").expect("MQTT_PASSWORD not set"),
        }
    }
}

/// Device configuration loaded from environment variables.
struct DeviceConfig {
    http_ip: String,
    http_user: String,
    http_password: String,
    mqtt_topic: String,
}

impl DeviceConfig {
    fn from_env(prefix: &str) -> Self {
        Self {
            http_ip: env::var(format!("{prefix}_HTTP_IP"))
                .unwrap_or_else(|_| panic!("{prefix}_HTTP_IP not set")),
            http_user: env::var(format!("{prefix}_HTTP_USER"))
                .unwrap_or_else(|_| panic!("{prefix}_HTTP_USER not set")),
            http_password: env::var(format!("{prefix}_HTTP_PASSWORD"))
                .unwrap_or_else(|_| panic!("{prefix}_HTTP_PASSWORD not set")),
            mqtt_topic: env::var(format!("{prefix}_MQTT_TOPIC"))
                .unwrap_or_else(|_| panic!("{prefix}_MQTT_TOPIC not set")),
        }
    }
}

// Helper functions to load device configs
fn cfg_broker() -> BrokerConfig {
    BrokerConfig::from_env()
}

fn cfg_light_1() -> DeviceConfig {
    DeviceConfig::from_env("LIGHT_1")
}

fn cfg_light_2() -> DeviceConfig {
    DeviceConfig::from_env("LIGHT_2")
}

fn cfg_light_3() -> DeviceConfig {
    DeviceConfig::from_env("LIGHT_3")
}

fn cfg_plug_1() -> DeviceConfig {
    DeviceConfig::from_env("PLUG_1")
}

fn cfg_plug_2() -> DeviceConfig {
    DeviceConfig::from_env("PLUG_2")
}

// =============================================================================
// HTTP Protocol Tests
// =============================================================================

mod http_protocol {
    use super::*;

    // -------------------------------------------------------------------------
    // Connection & Device Creation
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn connect_to_light_with_auto_detection() {
        let config = cfg_light_1();
        let (device, initial_state) = Device::http(&config.http_ip)
            .with_credentials(&config.http_user, &config.http_password)
            .build()
            .await
            .expect("Failed to connect to light");

        println!("Device capabilities: {:?}", device.capabilities());
        println!("Initial state: {:?}", initial_state);

        // Light should support dimmer
        assert!(
            device.capabilities().supports_dimmer_control(),
            "Light should support dimmer"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn connect_to_plug_with_auto_detection() {
        let (device, initial_state) = Device::http(cfg_plug_1().http_ip)
            .with_credentials(cfg_plug_1().http_user, cfg_plug_1().http_password)
            .build()
            .await
            .expect("Failed to connect to plug");

        println!("Device capabilities: {:?}", device.capabilities());
        println!("Initial state: {:?}", initial_state);

        // Plug should support energy monitoring
        assert!(
            device.capabilities().supports_energy_monitoring(),
            "Plug should support energy monitoring"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn connect_with_manual_capabilities() {
        let (device, _) = Device::http(cfg_light_1().http_ip)
            .with_credentials(cfg_light_1().http_user, cfg_light_1().http_password)
            .with_capabilities(Capabilities::rgbcct_light())
            .build_without_probe()
            .await
            .expect("Failed to connect");

        assert!(device.capabilities().supports_rgb_control());
        assert!(device.capabilities().supports_color_temperature_control());
    }

    // -------------------------------------------------------------------------
    // Power Control
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn power_on_off_toggle_light() {
        let (device, _) = Device::http(cfg_light_1().http_ip)
            .with_credentials(cfg_light_1().http_user, cfg_light_1().http_password)
            .build()
            .await
            .unwrap();

        // Get initial state
        let initial = device.get_power().await.unwrap();
        println!("Initial power state: {:?}", initial.first_power_state());

        // Turn ON
        let response = device.power_on().await.unwrap();
        assert_eq!(
            response.first_power_state().unwrap(),
            PowerState::On,
            "Power should be ON"
        );

        sleep(Duration::from_millis(500)).await;

        // Turn OFF
        let response = device.power_off().await.unwrap();
        assert_eq!(
            response.first_power_state().unwrap(),
            PowerState::Off,
            "Power should be OFF"
        );

        sleep(Duration::from_millis(500)).await;

        // Toggle (should turn ON)
        let response = device.power_toggle().await.unwrap();
        assert_eq!(
            response.first_power_state().unwrap(),
            PowerState::On,
            "Power should be ON after toggle"
        );

        // Restore initial state
        if initial.first_power_state().unwrap() == PowerState::Off {
            device.power_off().await.unwrap();
        }
    }

    #[tokio::test]
    #[ignore]
    async fn power_control_plug() {
        let (device, _) = Device::http(cfg_plug_2().http_ip)
            .with_credentials(cfg_plug_2().http_user, cfg_plug_2().http_password)
            .build()
            .await
            .unwrap();

        // Get and restore initial state
        let initial = device.get_power().await.unwrap();
        println!("PLUG_2 initial power: {:?}", initial.first_power_state());

        // Toggle twice to return to original state
        device.power_toggle().await.unwrap();
        sleep(Duration::from_millis(300)).await;
        device.power_toggle().await.unwrap();

        let final_state = device.get_power().await.unwrap();
        assert_eq!(
            initial.first_power_state().unwrap(),
            final_state.first_power_state().unwrap(),
            "Power should return to initial state"
        );
    }

    // -------------------------------------------------------------------------
    // Status
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn get_full_status() {
        let (device, _) = Device::http(cfg_light_1().http_ip)
            .with_credentials(cfg_light_1().http_user, cfg_light_1().http_password)
            .build()
            .await
            .unwrap();

        let status = device.status().await.unwrap();
        println!("Full status: {:?}", status);

        // Should have basic status info
        assert!(status.device_name().is_some(), "Should have device name");
        assert!(status.hostname().is_some(), "Should have hostname");
        assert!(status.ip_address().is_some(), "Should have IP address");
    }

    #[tokio::test]
    #[ignore]
    async fn get_abbreviated_status() {
        let (device, _) = Device::http(cfg_plug_1().http_ip)
            .with_credentials(cfg_plug_1().http_user, cfg_plug_1().http_password)
            .build()
            .await
            .unwrap();

        let status = device.status_abbreviated().await.unwrap();
        println!("Abbreviated status: {:?}", status);
    }

    // -------------------------------------------------------------------------
    // Dimmer (Lights only)
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn dimmer_control() {
        let (device, _) = Device::http(cfg_light_1().http_ip)
            .with_credentials(cfg_light_1().http_user, cfg_light_1().http_password)
            .build()
            .await
            .unwrap();

        // Ensure light is on
        device.power_on().await.unwrap();
        sleep(Duration::from_millis(300)).await;

        // Get initial dimmer
        let initial = device.get_dimmer().await.unwrap();
        println!("Initial dimmer: {}", initial.dimmer());

        // Set dimmer to 50%
        let dimmer = Dimmer::new(50).unwrap();
        let response = device.set_dimmer(dimmer).await.unwrap();
        assert_eq!(response.dimmer(), 50, "Dimmer should be 50%");

        sleep(Duration::from_millis(300)).await;

        // Set dimmer to 100%
        let dimmer = Dimmer::new(100).unwrap();
        let response = device.set_dimmer(dimmer).await.unwrap();
        assert_eq!(response.dimmer(), 100, "Dimmer should be 100%");

        // Restore initial dimmer
        let dimmer = Dimmer::new(initial.dimmer()).unwrap();
        device.set_dimmer(dimmer).await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn dimmer_unsupported_on_plug() {
        let (device, _) = Device::http(cfg_plug_1().http_ip)
            .with_credentials(cfg_plug_1().http_user, cfg_plug_1().http_password)
            .build()
            .await
            .unwrap();

        // Dimmer should fail on plug
        let result = device.set_dimmer(Dimmer::new(50).unwrap()).await;
        assert!(
            result.is_err(),
            "Dimmer should not be supported on plug: {:?}",
            result
        );
    }

    // -------------------------------------------------------------------------
    // Color Temperature (Lights only)
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn color_temperature_control() {
        let (device, _) = Device::http(cfg_light_2().http_ip)
            .with_credentials(cfg_light_2().http_user, cfg_light_2().http_password)
            .build()
            .await
            .unwrap();

        device.power_on().await.unwrap();
        sleep(Duration::from_millis(300)).await;

        // Get initial CT
        let initial = device.get_color_temperature().await.unwrap();
        println!("Initial CT: {}", initial.color_temperature());

        // Set warm white (500 mireds = 2000K)
        let ct = ColorTemperature::new(500).unwrap();
        let response = device.set_color_temperature(ct).await.unwrap();
        assert_eq!(response.color_temperature(), 500, "CT should be 500 mireds");

        sleep(Duration::from_millis(500)).await;

        // Set cool white (153 mireds = 6500K)
        let ct = ColorTemperature::new(153).unwrap();
        let response = device.set_color_temperature(ct).await.unwrap();
        assert_eq!(response.color_temperature(), 153, "CT should be 153 mireds");

        // Restore initial CT
        let ct = ColorTemperature::new(initial.color_temperature()).unwrap();
        device.set_color_temperature(ct).await.unwrap();
    }

    // -------------------------------------------------------------------------
    // HSB Color (RGB Lights only)
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn hsb_color_control() {
        let (device, _) = Device::http(cfg_light_1().http_ip)
            .with_credentials(cfg_light_1().http_user, cfg_light_1().http_password)
            .build()
            .await
            .unwrap();

        device.power_on().await.unwrap();
        sleep(Duration::from_millis(300)).await;

        // Get initial color
        let initial = device.get_hsb_color().await.unwrap();
        println!("Initial HSB: {:?}", initial.hsb_color());

        // Set red (hue=0, sat=100, bri=100)
        let red = HsbColor::new(0, 100, 100).unwrap();
        let response = device.set_hsb_color(red).await.unwrap();
        let returned = response.hsb_color().unwrap();
        assert_eq!(returned.hue(), 0, "Hue should be 0 (red)");

        sleep(Duration::from_millis(500)).await;

        // Set green (hue=120)
        let green = HsbColor::new(120, 100, 100).unwrap();
        let response = device.set_hsb_color(green).await.unwrap();
        let returned = response.hsb_color().unwrap();
        assert_eq!(returned.hue(), 120, "Hue should be 120 (green)");

        sleep(Duration::from_millis(500)).await;

        // Set blue (hue=240)
        let blue = HsbColor::new(240, 100, 100).unwrap();
        let response = device.set_hsb_color(blue).await.unwrap();
        let returned = response.hsb_color().unwrap();
        assert_eq!(returned.hue(), 240, "Hue should be 240 (blue)");

        // Restore initial color
        if let Ok(color) = initial.hsb_color() {
            device.set_hsb_color(color).await.unwrap();
        }
    }

    // -------------------------------------------------------------------------
    // RGB Color
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn rgb_color_control() {
        let (device, _) = Device::http(cfg_light_3().http_ip)
            .with_credentials(cfg_light_3().http_user, cfg_light_3().http_password)
            .build()
            .await
            .unwrap();

        device.power_on().await.unwrap();
        sleep(Duration::from_millis(300)).await;

        // Set color using hex
        let color = RgbColor::from_hex("#FF5733").unwrap();
        let response = device.set_rgb_color(color).await.unwrap();
        println!("RGB response: {}", response.to_hex_with_hash());

        sleep(Duration::from_millis(500)).await;

        // Set pure white
        let white = RgbColor::new(255, 255, 255);
        device.set_rgb_color(white).await.unwrap();
    }

    // -------------------------------------------------------------------------
    // Scheme & Wakeup Duration
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn scheme_control() {
        let (device, _) = Device::http(cfg_light_1().http_ip)
            .with_credentials(cfg_light_1().http_user, cfg_light_1().http_password)
            .build()
            .await
            .unwrap();

        device.power_on().await.unwrap();
        sleep(Duration::from_millis(300)).await;

        // Get initial scheme
        let initial = device.get_scheme().await.unwrap();
        println!("Initial scheme: {:?}", initial.scheme());

        // Set random color scheme
        let response = device.set_scheme(Scheme::RANDOM).await.unwrap();
        assert_eq!(response.scheme().unwrap(), Scheme::RANDOM);

        sleep(Duration::from_secs(2)).await;

        // Set single color scheme
        let response = device.set_scheme(Scheme::SINGLE).await.unwrap();
        assert_eq!(response.scheme().unwrap(), Scheme::SINGLE);

        // Restore initial scheme
        if let Ok(scheme) = initial.scheme() {
            device.set_scheme(scheme).await.unwrap();
        }
    }

    #[tokio::test]
    #[ignore]
    async fn wakeup_duration_control() {
        let (device, _) = Device::http(cfg_light_1().http_ip)
            .with_credentials(cfg_light_1().http_user, cfg_light_1().http_password)
            .build()
            .await
            .unwrap();

        // Get initial duration
        let initial = device.get_wakeup_duration().await.unwrap();
        println!("Initial wakeup duration: {:?}", initial.duration());

        // Set to 5 minutes
        let duration = WakeupDuration::from_minutes(5).unwrap();
        let response = device.set_wakeup_duration(duration).await.unwrap();
        assert_eq!(response.duration().unwrap().seconds(), 300);

        // Set to 30 seconds
        let duration = WakeupDuration::new(30).unwrap();
        let response = device.set_wakeup_duration(duration).await.unwrap();
        assert_eq!(response.duration().unwrap().seconds(), 30);

        // Restore initial duration
        if let Ok(d) = initial.duration() {
            device.set_wakeup_duration(d).await.unwrap();
        }
    }

    // -------------------------------------------------------------------------
    // Fade Control
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn fade_control() {
        let (device, _) = Device::http(cfg_light_2().http_ip)
            .with_credentials(cfg_light_2().http_user, cfg_light_2().http_password)
            .build()
            .await
            .unwrap();

        // Get initial fade state
        let initial_fade = device.get_fade().await.unwrap();
        let initial_speed = device.get_fade_speed().await.unwrap();
        println!(
            "Initial fade: {:?}, speed: {:?}",
            initial_fade.is_enabled(),
            initial_speed.speed()
        );

        // Enable fade
        let response = device.enable_fade().await.unwrap();
        assert!(response.is_enabled().unwrap(), "Fade should be enabled");

        // Set fade speed to 5
        let speed = FadeSpeed::new(5).unwrap();
        let response = device.set_fade_speed(speed).await.unwrap();
        assert_eq!(response.speed().unwrap().value(), 5);

        // Test fade effect
        device.power_on().await.unwrap();
        device.set_dimmer(Dimmer::new(100).unwrap()).await.unwrap();
        sleep(Duration::from_secs(1)).await;
        device.set_dimmer(Dimmer::new(10).unwrap()).await.unwrap();
        sleep(Duration::from_secs(2)).await;

        // Disable fade
        let response = device.disable_fade().await.unwrap();
        assert!(!response.is_enabled().unwrap(), "Fade should be disabled");

        // Restore initial state
        if initial_fade.is_enabled().unwrap() {
            device.enable_fade().await.unwrap();
        }
        if let Ok(s) = initial_speed.speed() {
            device.set_fade_speed(s).await.unwrap();
        }
    }

    #[tokio::test]
    #[ignore]
    async fn fade_at_startup_control() {
        let (device, _) = Device::http(cfg_light_1().http_ip)
            .with_credentials(cfg_light_1().http_user, cfg_light_1().http_password)
            .build()
            .await
            .unwrap();

        // Get initial state
        let initial = device.get_fade_at_startup().await.unwrap();
        println!("Initial fade at startup: {:?}", initial.is_enabled());

        // Enable
        let response = device.enable_fade_at_startup().await.unwrap();
        assert!(
            response.is_enabled().unwrap(),
            "Fade at startup should be enabled"
        );

        // Disable
        let response = device.disable_fade_at_startup().await.unwrap();
        assert!(
            !response.is_enabled().unwrap(),
            "Fade at startup should be disabled"
        );

        // Restore initial state
        if initial.is_enabled().unwrap() {
            device.enable_fade_at_startup().await.unwrap();
        }
    }

    // -------------------------------------------------------------------------
    // Energy Monitoring (Plugs only)
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn energy_monitoring() {
        let (device, _) = Device::http(cfg_plug_1().http_ip)
            .with_credentials(cfg_plug_1().http_user, cfg_plug_1().http_password)
            .build()
            .await
            .unwrap();

        let response = device.energy().await.unwrap();
        let energy = response.energy().expect("Should have energy data");

        println!("Energy data:");
        println!("  Power: {} W", energy.power);
        println!("  Voltage: {} V", energy.voltage);
        println!("  Current: {} A", energy.current);
        println!("  Today: {} kWh", energy.today);
        println!("  Total: {} kWh", energy.total);
        println!("  Power factor: {}", energy.factor);

        // Basic sanity checks
        assert!(
            energy.voltage > 200 && energy.voltage < 250,
            "Voltage should be ~230V"
        );
    }

    #[tokio::test]
    #[ignore]
    async fn energy_unsupported_on_light() {
        let (device, _) = Device::http(cfg_light_1().http_ip)
            .with_credentials(cfg_light_1().http_user, cfg_light_1().http_password)
            .build()
            .await
            .unwrap();

        let result = device.energy().await;
        assert!(
            result.is_err(),
            "Energy should not be supported on light: {:?}",
            result
        );
    }

    // -------------------------------------------------------------------------
    // Routines
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn run_routine() {
        use tasmor_lib::command::Routine;

        let (device, _) = Device::http(cfg_light_1().http_ip)
            .with_credentials(cfg_light_1().http_user, cfg_light_1().http_password)
            .build()
            .await
            .unwrap();

        // Build a routine: turn on, set dimmer to 50, wait, set to 100
        let routine = Routine::builder()
            .power_on(PowerIndex::one())
            .set_dimmer(Dimmer::new(50).unwrap())
            .delay(Duration::from_millis(500))
            .set_dimmer(Dimmer::new(100).unwrap())
            .build()
            .unwrap();

        let response = device.run(&routine).await.unwrap();
        println!("Routine response: {:?}", response);

        // Check final dimmer value
        if let Ok(dimmer) = response.get_as::<u8>("Dimmer") {
            assert_eq!(dimmer, 100, "Final dimmer should be 100");
        }

        // Cleanup
        device.power_off().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn run_color_routine() {
        use tasmor_lib::command::Routine;

        let (device, _) = Device::http(cfg_light_3().http_ip)
            .with_credentials(cfg_light_3().http_user, cfg_light_3().http_password)
            .build()
            .await
            .unwrap();

        // Color transition routine
        let routine = Routine::builder()
            .power_on(PowerIndex::one())
            .set_hsb_color(HsbColor::new(0, 100, 100).unwrap()) // Red
            .delay(Duration::from_millis(500))
            .set_hsb_color(HsbColor::new(120, 100, 100).unwrap()) // Green
            .delay(Duration::from_millis(500))
            .set_hsb_color(HsbColor::new(240, 100, 100).unwrap()) // Blue
            .build()
            .unwrap();

        let response = device.run(&routine).await.unwrap();
        println!("Color routine response: {:?}", response);

        sleep(Duration::from_secs(1)).await;
        device.power_off().await.unwrap();
    }
}

// =============================================================================
// MQTT Protocol Tests
// =============================================================================

mod mqtt_protocol {
    use super::*;

    // -------------------------------------------------------------------------
    // Broker Connection
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn connect_to_broker() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .expect("Failed to connect to MQTT broker");

        println!(
            "Connected to MQTT broker at {}:{}",
            cfg_broker().ip,
            cfg_broker().port
        );

        broker.disconnect().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn connect_to_device_via_mqtt() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, initial_state) = broker
            .device(cfg_light_1().mqtt_topic)
            .build()
            .await
            .expect("Failed to connect to device via MQTT");

        println!("Device topic: {}", device.topic());
        println!("Initial state: {:?}", initial_state);
        println!("Capabilities: {:?}", device.capabilities());

        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // Power Control via MQTT
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn mqtt_power_control() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_2().mqtt_topic)
            .build()
            .await
            .unwrap();

        // Get initial state
        let initial = device.get_power().await.unwrap();
        println!("Initial power: {:?}", initial.first_power_state());

        // Toggle
        let response = device.power_toggle().await.unwrap();
        println!("After toggle: {:?}", response.first_power_state());

        sleep(Duration::from_millis(500)).await;

        // Toggle back
        device.power_toggle().await.unwrap();

        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // Subscriptions (MQTT only feature)
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn power_change_subscription() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_1().mqtt_topic)
            .build()
            .await
            .unwrap();

        let callback_count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&callback_count);

        // Subscribe to power changes
        let _sub_id = device.on_power_changed(move |idx, state| {
            println!("Power callback: relay {} = {:?}", idx, state);
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Trigger power changes
        device.power_on().await.unwrap();
        sleep(Duration::from_millis(500)).await;
        device.power_off().await.unwrap();
        sleep(Duration::from_millis(500)).await;

        // Check callbacks were triggered
        let count = callback_count.load(Ordering::SeqCst);
        println!("Power callbacks received: {}", count);
        assert!(
            count >= 2,
            "Should have received at least 2 power callbacks"
        );

        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn dimmer_change_subscription() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_1().mqtt_topic)
            .build()
            .await
            .unwrap();

        let callback_count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&callback_count);

        // Subscribe to dimmer changes
        let _sub_id = device.on_dimmer_changed(move |dimmer| {
            println!("Dimmer callback: {:?}", dimmer.value());
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        // Trigger dimmer changes
        device.power_on().await.unwrap();
        sleep(Duration::from_millis(300)).await;
        device.set_dimmer(Dimmer::new(30).unwrap()).await.unwrap();
        sleep(Duration::from_millis(300)).await;
        device.set_dimmer(Dimmer::new(80).unwrap()).await.unwrap();
        sleep(Duration::from_millis(300)).await;

        let count = callback_count.load(Ordering::SeqCst);
        println!("Dimmer callbacks received: {}", count);
        assert!(
            count >= 2,
            "Should have received at least 2 dimmer callbacks"
        );

        device.power_off().await.unwrap();
        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn color_change_subscription() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_3().mqtt_topic)
            .build()
            .await
            .unwrap();

        let callback_count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&callback_count);

        let _sub_id = device.on_color_changed(move |color| {
            println!(
                "Color callback: H={}, S={}, B={}",
                color.hue(),
                color.saturation(),
                color.brightness()
            );
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        device.power_on().await.unwrap();
        sleep(Duration::from_millis(300)).await;

        device
            .set_hsb_color(HsbColor::new(0, 100, 100).unwrap())
            .await
            .unwrap();
        sleep(Duration::from_millis(300)).await;
        device
            .set_hsb_color(HsbColor::new(180, 100, 100).unwrap())
            .await
            .unwrap();
        sleep(Duration::from_millis(300)).await;

        let count = callback_count.load(Ordering::SeqCst);
        println!("Color callbacks received: {}", count);
        assert!(
            count >= 2,
            "Should have received at least 2 color callbacks"
        );

        device.power_off().await.unwrap();
        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn unsubscribe_callback() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_1().mqtt_topic)
            .build()
            .await
            .unwrap();

        let callback_count = Arc::new(AtomicUsize::new(0));
        let count_clone = Arc::clone(&callback_count);

        // Subscribe
        let sub_id = device.on_power_changed(move |_, _| {
            count_clone.fetch_add(1, Ordering::SeqCst);
        });

        device.power_toggle().await.unwrap();
        sleep(Duration::from_millis(300)).await;

        let count_before = callback_count.load(Ordering::SeqCst);

        // Unsubscribe
        assert!(device.unsubscribe(sub_id), "Unsubscribe should succeed");

        device.power_toggle().await.unwrap();
        sleep(Duration::from_millis(300)).await;

        let count_after = callback_count.load(Ordering::SeqCst);
        assert_eq!(
            count_before, count_after,
            "No callbacks should be received after unsubscribe"
        );

        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // Multiple Devices on Same Broker
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn multiple_devices_same_broker() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device1, _) = broker
            .device(cfg_light_1().mqtt_topic)
            .build()
            .await
            .unwrap();

        let (device2, _) = broker
            .device(cfg_light_2().mqtt_topic)
            .build()
            .await
            .unwrap();

        let (device3, _) = broker
            .device(cfg_plug_1().mqtt_topic)
            .build()
            .await
            .unwrap();

        println!("Device 1 topic: {}", device1.topic());
        println!("Device 2 topic: {}", device2.topic());
        println!("Device 3 topic: {}", device3.topic());

        // Control devices independently
        device1.power_on().await.unwrap();
        device2.power_on().await.unwrap();
        let energy = device3.energy().await.unwrap();
        println!("Plug energy: {:?}", energy.energy());

        sleep(Duration::from_millis(500)).await;

        device1.power_off().await.unwrap();
        device2.power_off().await.unwrap();

        device1.disconnect().await;
        device2.disconnect().await;
        device3.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // MQTT Light Control
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn mqtt_dimmer_control() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_2().mqtt_topic)
            .build()
            .await
            .unwrap();

        device.power_on().await.unwrap();
        sleep(Duration::from_millis(300)).await;

        let initial = device.get_dimmer().await.unwrap();
        println!("Initial dimmer: {}", initial.dimmer());

        device.set_dimmer(Dimmer::new(25).unwrap()).await.unwrap();
        sleep(Duration::from_millis(300)).await;

        let response = device.get_dimmer().await.unwrap();
        assert_eq!(response.dimmer(), 25);

        // Restore
        device
            .set_dimmer(Dimmer::new(initial.dimmer()).unwrap())
            .await
            .unwrap();
        device.power_off().await.unwrap();

        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    #[tokio::test]
    #[ignore]
    async fn mqtt_color_temperature_control() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_1().mqtt_topic)
            .build()
            .await
            .unwrap();

        device.power_on().await.unwrap();
        sleep(Duration::from_millis(300)).await;

        // Set warm
        device
            .set_color_temperature(ColorTemperature::new(400).unwrap())
            .await
            .unwrap();
        sleep(Duration::from_millis(500)).await;

        // Set cool
        device
            .set_color_temperature(ColorTemperature::new(200).unwrap())
            .await
            .unwrap();
        sleep(Duration::from_millis(500)).await;

        device.power_off().await.unwrap();
        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // MQTT Fade Control
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn mqtt_fade_control() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_2().mqtt_topic)
            .build()
            .await
            .unwrap();

        // Get initial fade state
        let initial_fade = device.get_fade().await.unwrap();
        let initial_speed = device.get_fade_speed().await.unwrap();
        println!(
            "MQTT Initial fade: {:?}, speed: {:?}",
            initial_fade.is_enabled(),
            initial_speed.speed()
        );

        // Enable fade
        let response = device.enable_fade().await.unwrap();
        assert!(response.is_enabled().unwrap(), "Fade should be enabled");

        // Set fade speed
        let speed = FadeSpeed::new(4).unwrap();
        let response = device.set_fade_speed(speed).await.unwrap();
        assert_eq!(response.speed().unwrap().value(), 4);

        // Disable fade
        let response = device.disable_fade().await.unwrap();
        assert!(!response.is_enabled().unwrap(), "Fade should be disabled");

        // Restore initial state
        if initial_fade.is_enabled().unwrap() {
            device.enable_fade().await.unwrap();
        }
        if let Ok(s) = initial_speed.speed() {
            device.set_fade_speed(s).await.unwrap();
        }

        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // MQTT Scheme Control
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn mqtt_scheme_control() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_1().mqtt_topic)
            .build()
            .await
            .unwrap();

        device.power_on().await.unwrap();
        sleep(Duration::from_millis(300)).await;

        // Get initial scheme
        let initial = device.get_scheme().await.unwrap();
        println!("MQTT Initial scheme: {:?}", initial.scheme());

        // Set random scheme
        let response = device.set_scheme(Scheme::RANDOM).await.unwrap();
        assert_eq!(response.scheme().unwrap(), Scheme::RANDOM);

        sleep(Duration::from_secs(1)).await;

        // Set single scheme
        let response = device.set_scheme(Scheme::SINGLE).await.unwrap();
        assert_eq!(response.scheme().unwrap(), Scheme::SINGLE);

        // Restore initial scheme
        if let Ok(scheme) = initial.scheme() {
            device.set_scheme(scheme).await.unwrap();
        }

        device.power_off().await.unwrap();
        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // MQTT Wakeup Duration Control
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn mqtt_wakeup_duration_control() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_1().mqtt_topic)
            .build()
            .await
            .unwrap();

        // Get initial duration
        let initial = device.get_wakeup_duration().await.unwrap();
        println!("MQTT Initial wakeup duration: {:?}", initial.duration());

        // Set to 3 minutes
        let duration = WakeupDuration::from_minutes(3).unwrap();
        let response = device.set_wakeup_duration(duration).await.unwrap();
        assert_eq!(response.duration().unwrap().seconds(), 180);

        // Set to 45 seconds
        let duration = WakeupDuration::new(45).unwrap();
        let response = device.set_wakeup_duration(duration).await.unwrap();
        assert_eq!(response.duration().unwrap().seconds(), 45);

        // Restore initial duration
        if let Ok(d) = initial.duration() {
            device.set_wakeup_duration(d).await.unwrap();
        }

        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // MQTT Routines
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn mqtt_run_routine() {
        use tasmor_lib::command::Routine;

        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_3().mqtt_topic)
            .build()
            .await
            .unwrap();

        // Build a routine: turn on, set dimmer, wait, change color
        let routine = Routine::builder()
            .power_on(PowerIndex::one())
            .set_dimmer(Dimmer::new(80).unwrap())
            .delay(Duration::from_millis(300))
            .set_hsb_color(HsbColor::new(60, 100, 100).unwrap()) // Yellow
            .build()
            .unwrap();

        let response = device.run(&routine).await.unwrap();
        println!("MQTT Routine response: {:?}", response);

        sleep(Duration::from_millis(500)).await;

        // Cleanup
        device.power_off().await.unwrap();
        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // MQTT Energy Monitoring
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn mqtt_energy_monitoring() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_plug_2().mqtt_topic)
            .build()
            .await
            .unwrap();

        let response = device.energy().await.unwrap();
        let energy = response.energy().expect("Should have energy data");

        println!("MQTT Energy data:");
        println!("  Power: {} W", energy.power);
        println!("  Voltage: {} V", energy.voltage);
        println!("  Current: {} A", energy.current);

        device.disconnect().await;
        broker.disconnect().await.unwrap();
    }

    // -------------------------------------------------------------------------
    // Device Disconnect
    // -------------------------------------------------------------------------

    #[tokio::test]
    #[ignore]
    async fn device_disconnect_is_idempotent() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (device, _) = broker
            .device(cfg_light_1().mqtt_topic)
            .build()
            .await
            .unwrap();

        assert!(!device.is_disconnected());

        device.disconnect().await;
        assert!(device.is_disconnected());

        // Second disconnect should be safe
        device.disconnect().await;
        assert!(device.is_disconnected());

        broker.disconnect().await.unwrap();
    }
}

// =============================================================================
// Cross-Protocol Tests
// =============================================================================

mod cross_protocol {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn http_and_mqtt_see_same_state() {
        // Connect via HTTP
        let (http_device, _) = Device::http(cfg_light_1().http_ip)
            .with_credentials(cfg_light_1().http_user, cfg_light_1().http_password)
            .build()
            .await
            .unwrap();

        // Connect via MQTT
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let (mqtt_device, _) = broker
            .device(cfg_light_1().mqtt_topic)
            .build()
            .await
            .unwrap();

        // Set state via HTTP
        http_device.power_on().await.unwrap();
        http_device
            .set_dimmer(Dimmer::new(75).unwrap())
            .await
            .unwrap();
        sleep(Duration::from_millis(500)).await;

        // Read via MQTT
        let mqtt_power = mqtt_device.get_power().await.unwrap();
        let mqtt_dimmer = mqtt_device.get_dimmer().await.unwrap();

        assert_eq!(mqtt_power.first_power_state().unwrap(), PowerState::On);
        assert_eq!(mqtt_dimmer.dimmer(), 75);

        // Set state via MQTT
        mqtt_device
            .set_dimmer(Dimmer::new(50).unwrap())
            .await
            .unwrap();
        sleep(Duration::from_millis(500)).await;

        // Read via HTTP
        let http_dimmer = http_device.get_dimmer().await.unwrap();
        assert_eq!(http_dimmer.dimmer(), 50);

        // Cleanup
        http_device.power_off().await.unwrap();
        mqtt_device.disconnect().await;
        broker.disconnect().await.unwrap();
    }
}

// =============================================================================
// Error Handling Tests
// =============================================================================

mod error_handling {
    use super::*;

    #[tokio::test]
    #[ignore]
    async fn http_connection_to_invalid_ip() {
        let result = Device::http("192.168.11.254") // Non-existent IP
            .with_credentials("admin", "password")
            .build()
            .await;

        assert!(result.is_err(), "Should fail to connect to invalid IP");
        println!("Error: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore]
    async fn http_wrong_credentials() {
        let result = Device::http(cfg_light_1().http_ip)
            .with_credentials("wrong_user", "wrong_password")
            .build()
            .await;

        assert!(result.is_err(), "Should fail with wrong credentials");
        println!("Error: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore]
    async fn mqtt_connection_to_invalid_broker() {
        let result = MqttBroker::builder()
            .host("192.168.11.254") // Non-existent broker
            .port(1883)
            .credentials("user", "pass")
            .build()
            .await;

        assert!(result.is_err(), "Should fail to connect to invalid broker");
        println!("Error: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore]
    async fn mqtt_wrong_credentials() {
        let result = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials("wrong_user", "wrong_password")
            .build()
            .await;

        assert!(result.is_err(), "Should fail with wrong credentials");
        println!("Error: {:?}", result.err());
    }

    #[tokio::test]
    #[ignore]
    async fn mqtt_invalid_device_topic() {
        let broker = MqttBroker::builder()
            .host(cfg_broker().ip)
            .port(cfg_broker().port)
            .credentials(cfg_broker().user, cfg_broker().password)
            .build()
            .await
            .unwrap();

        let result = broker
            .device("nonexistent_device_topic_12345")
            .build()
            .await;

        // This might timeout or fail depending on implementation
        match result {
            Ok(_) => println!("Unexpectedly succeeded"),
            Err(e) => println!("Error for invalid topic: {:?}", e),
        }

        broker.disconnect().await.unwrap();
    }
}
