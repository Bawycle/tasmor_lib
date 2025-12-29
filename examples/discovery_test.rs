// SPDX-License-Identifier: MPL-2.0

//! Test program: Discover Tasmota devices on an MQTT broker.
//!
//! This example demonstrates:
//! - How to use `MqttBroker::discover_devices` to automatically find Tasmota devices
//! - How to create devices manually with `broker.device(topic)`
//! - **Connection pooling**: All devices share a single MQTT connection to the broker
//!
//! # Connection Sharing
//!
//! When you create devices via `broker.device()` or `broker.discover_devices()`,
//! they all share the broker's single MQTT connection. This is more efficient
//! than each device creating its own connection:
//!
//! - 1 broker + 10 devices = 1 TCP connection (not 11!)
//! - Lower memory usage
//! - Fewer broker resources consumed
//! - Faster device creation
//!
//! # Usage
//!
//! ```bash
//! cargo run --example discovery_test -- <host> [port] [username] [password]
//! ```
//!
//! # Example
//!
//! ```bash
//! # Without authentication (default port 1883)
//! cargo run --example discovery_test -- 192.168.1.50
//!
//! # With custom port
//! cargo run --example discovery_test -- 192.168.1.50 1883
//!
//! # With authentication
//! cargo run --example discovery_test -- 192.168.1.50 1883 mqtt_user mqtt_pass
//! ```

use std::env;
use std::time::Duration;
use tasmor_lib::MqttBroker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Usage: {} <host> [port] [username] [password]", args[0]);
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  cargo run --example discovery_test -- 192.168.1.50");
        eprintln!("  cargo run --example discovery_test -- 192.168.1.50 1883");
        eprintln!("  cargo run --example discovery_test -- 192.168.1.50 1883 user pass");
        std::process::exit(1);
    }

    let host = &args[1];
    let port: u16 = args.get(2).and_then(|p| p.parse().ok()).unwrap_or(1883);
    let credentials = if args.len() >= 5 {
        Some((&args[3], &args[4]))
    } else {
        None
    };

    println!("Connecting to MQTT broker {}:{}...", host, port);

    // Build broker connection
    let mut builder = MqttBroker::builder().host(host).port(port);

    if let Some((username, password)) = credentials {
        builder = builder.credentials(username, password);
    }

    let broker = builder.build().await?;
    println!("Connected! (1 MQTT connection established)");
    println!();

    println!("Discovering Tasmota devices...");
    println!("(Listening for 10 seconds)");
    println!();

    // Discover devices
    match broker.discover_devices(Duration::from_secs(10)).await {
        Ok(devices) => {
            if devices.is_empty() {
                println!("No devices found.");
                println!();
                println!("Tips:");
                println!("  - Make sure your Tasmota devices are connected to the MQTT broker");
                println!("  - Try increasing the discovery timeout");
                println!("  - Check that Tasmota devices have MQTT enabled and configured");
            } else {
                println!("Found {} device(s):", devices.len());
                println!(
                    "All {} devices share the same MQTT connection!",
                    devices.len()
                );
                println!(
                    "Active device subscriptions: {}",
                    broker.subscription_count().await
                );
                println!();

                for (i, (device, state)) in devices.iter().enumerate() {
                    println!("Device #{}", i + 1);
                    println!("  Capabilities:");
                    let caps = device.capabilities();
                    println!("    - Power channels: {}", caps.power_channels());
                    println!("    - Dimmer: {}", caps.supports_dimmer_control());
                    println!("    - RGB: {}", caps.supports_rgb_control());
                    println!(
                        "    - Color temp: {}",
                        caps.supports_color_temperature_control()
                    );
                    println!("    - Energy: {}", caps.supports_energy_monitoring());
                    println!("  Current state:");
                    if let Some(power) = state.power(1) {
                        println!("    - Power: {power:?}");
                    }
                    if let Some(dimmer) = state.dimmer() {
                        println!("    - Dimmer: {dimmer}");
                    }
                    if let Some(ct) = state.color_temperature() {
                        println!("    - Color temp: {} mireds", ct.value());
                    }
                    println!();
                }

                // Example: Toggle the first device
                if !devices.is_empty() {
                    println!("Toggling power on first device...");
                    let (device, _) = &devices[0];
                    match device.power_toggle().await {
                        Ok(resp) => {
                            println!("Power toggled! New state: {:?}", resp.first_power_state())
                        }
                        Err(e) => println!("Error toggling power: {e}"),
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Discovery failed: {e}");
            std::process::exit(1);
        }
    }

    // Demonstrate manual device creation with shared connection
    println!();
    println!("--- Manual Device Creation Demo ---");
    println!();
    println!("You can also create devices manually with broker.device(topic):");
    println!("  let (bulb, _) = broker.device(\"tasmota_bulb\").build().await?;");
    println!("  let (plug, _) = broker.device(\"tasmota_plug\").build().await?;");
    println!();
    println!("Both devices will share the broker's single MQTT connection.");
    println!(
        "Current subscription count: {}",
        broker.subscription_count().await
    );

    // Disconnect
    let _ = broker.disconnect().await;

    println!();
    println!("Done!");
    Ok(())
}
