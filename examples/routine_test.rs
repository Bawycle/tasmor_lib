// SPDX-License-Identifier: MPL-2.0

//! Test program: Execute a wakeup routine that gradually increases brightness.
//!
//! This example demonstrates how to use the `Routine` builder to execute
//! multiple commands atomically on a Tasmota device.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example routine_test -- <host> <topic> <username> <password>
//! ```
//!
//! # Example
//!
//! ```bash
//! cargo run --example routine_test -- 192.168.1.50 tasmota_ABCDEF mqtt_user mqtt_pass
//! ```

use std::env;
use std::time::Duration;
use tasmor_lib::{CapabilitiesBuilder, Dimmer, MqttBroker, PowerIndex, Routine};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() != 5 {
        eprintln!("Usage: {} <host> <topic> <username> <password>", args[0]);
        eprintln!();
        eprintln!("Example:");
        eprintln!("  cargo run --example routine_test -- 192.168.1.50 tasmota_ABCDEF user pass");
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
    if let Some(dimmer) = initial_state.dimmer() {
        println!("Initial dimmer: {dimmer}");
    }

    // Build a wakeup routine: start dim, gradually increase brightness
    println!("\nBuilding wakeup routine...");
    let wakeup_routine = Routine::builder()
        .set_dimmer(Dimmer::new(10)?)
        .power_on(PowerIndex::one())
        .delay(Duration::from_secs(2))
        .set_dimmer(Dimmer::new(30)?)
        .delay(Duration::from_secs(2))
        .set_dimmer(Dimmer::new(50)?)
        .delay(Duration::from_secs(2))
        .set_dimmer(Dimmer::new(75)?)
        .delay(Duration::from_secs(2))
        .set_dimmer(Dimmer::new(100)?)
        .build()?;

    println!("\nExecuting wakeup routine (10% -> 30% -> 50% -> 75% -> 100%)...");

    match device.run(&wakeup_routine).await {
        Ok(resp) => {
            println!("Routine executed successfully!");
            println!("Response fields:");
            for (key, value) in resp.iter() {
                println!("  {key}: {value}");
            }
        }
        Err(e) => println!("Routine error: {e}"),
    }

    // Wait for the routine to complete visually
    println!("\nWaiting 10 seconds for routine to complete...");
    tokio::time::sleep(Duration::from_secs(10)).await;

    // Turn off
    println!("Turning off...");
    match device.power_off().await {
        Ok(resp) => println!("Power OFF: {resp:?}"),
        Err(e) => println!("Power OFF error: {e}"),
    }

    // Clean disconnect
    println!("Disconnecting...");
    device.disconnect().await;
    broker.disconnect().await?;

    println!("Done!");
    Ok(())
}
