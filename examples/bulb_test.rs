// SPDX-License-Identifier: MPL-2.0

//! Test program: Turn on a bulb for 8 seconds, then turn it off.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example bulb_test -- <host> <topic> <username> <password>
//! ```
//!
//! # Example
//!
//! ```bash
//! cargo run --example bulb_test -- 192.168.1.50 tasmota_ABCDEF mqtt_user mqtt_pass
//! ```

use std::env;
use std::time::Duration;
use tasmor_lib::{CapabilitiesBuilder, MqttBroker};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        eprintln!("Usage: {} <host> <topic> <username> <password>", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!("  cargo run --example bulb_test -- 192.168.1.50 tasmota_ABCDEF user pass");
        std::process::exit(1);
    }

    let host = &args[1];
    let topic = &args[2];
    let username = &args[3];
    let password = &args[4];

    println!("Connecting to MQTT broker {host}...");

    let broker = MqttBroker::builder()
        .host(host)
        .credentials(username, password)
        .build()
        .await?;

    let capabilities = CapabilitiesBuilder::new()
        .with_dimmer_control()
        .with_color_temperature_control()
        .build();

    let (device, initial_state) = broker
        .device(topic)
        .with_capabilities(capabilities)
        .build_without_probe()
        .await?;

    println!("Connected!");
    if let Some(power) = initial_state.power(1) {
        println!("Initial power state: {power:?}");
    }
    println!("Turning on the bulb...");
    match device.power_on().await {
        Ok(resp) => println!("Bulb ON: {:?}", resp),
        Err(e) => println!("Power ON sent (response parse error: {e})"),
    }

    println!("Waiting 8 seconds...");
    tokio::time::sleep(Duration::from_secs(8)).await;

    println!("Turning off the bulb...");
    match device.power_off().await {
        Ok(resp) => println!("Bulb OFF: {:?}", resp),
        Err(e) => println!("Power OFF sent (response parse error: {e})"),
    }

    println!("Disconnecting...");
    device.disconnect().await;
    broker.disconnect().await?;

    println!("Done!");
    Ok(())
}
