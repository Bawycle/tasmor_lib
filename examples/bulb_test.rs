// SPDX-License-Identifier: MPL-2.0

//! Test program: Turn on a bulb for 8 seconds, then turn it off.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example bulb_test -- <broker> <topic> <username> <password>
//! ```
//!
//! # Example
//!
//! ```bash
//! cargo run --example bulb_test -- mqtt://192.168.1.50:1883 tasmota_ABCDEF mqtt_user mqtt_pass
//! ```

use std::env;
use std::time::Duration;
use tasmor_lib::{CapabilitiesBuilder, Device};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        eprintln!("Usage: {} <broker> <topic> <username> <password>", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!(
            "  cargo run --example bulb_test -- mqtt://192.168.1.50:1883 tasmota_ABCDEF user pass"
        );
        std::process::exit(1);
    }

    let broker = &args[1];
    let topic = &args[2];
    let username = &args[3];
    let password = &args[4];

    println!("Connecting to MQTT broker {broker}...");

    let capabilities = CapabilitiesBuilder::new()
        .with_dimmer_control()
        .with_color_temperature_control()
        .build();

    let (device, initial_state) = Device::mqtt(broker, topic)
        .with_credentials(username, password)
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

    println!("Done!");
    Ok(())
}
