// SPDX-License-Identifier: MPL-2.0

//! Energy monitoring example.
//!
//! Demonstrates how to query and display energy consumption data from
//! Tasmota devices with power monitoring (smart plugs, energy meters).
//!
//! The device builder automatically queries initial state, so energy data
//! is available immediately after connection.
//!
//! # Usage
//!
//! ```bash
//! cargo run --example energy_test -- <host> <topic> [username] [password]
//! ```
//!
//! # Example
//!
//! ```bash
//! cargo run --example energy_test -- 192.168.1.50 tasmota_plug
//! cargo run --example energy_test -- 192.168.1.50 tasmota_plug user pass
//! ```

use std::env;
use tasmor_lib::{CapabilitiesBuilder, MqttBroker};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <host> <topic> [username] [password]", args[0]);
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  cargo run --example energy_test -- 192.168.1.50 tasmota_plug");
        eprintln!("  cargo run --example energy_test -- 192.168.1.50 tasmota_plug user pass");
        std::process::exit(1);
    }

    let host = &args[1];
    let topic = &args[2];

    println!("=== Tasmota Energy Monitor ===");
    println!("Broker: {host}");
    println!("Device: {topic}");
    println!();

    // Build broker connection
    let mut broker_builder = MqttBroker::builder().host(host);

    if args.len() >= 5 {
        broker_builder = broker_builder.credentials(&args[3], &args[4]);
    }

    let broker = broker_builder.build().await?;

    // Build device with energy monitoring capability
    let capabilities = CapabilitiesBuilder::new().with_energy_monitoring().build();

    // Build returns (device, initial_state) - energy data is already available!
    let (device, state) = broker
        .device(topic)
        .with_capabilities(capabilities)
        .build_without_probe()
        .await?;

    // Display energy data from initial state
    println!();
    println!("┌─────────────────────────────────────┐");
    println!("│         Current Readings            │");
    println!("├─────────────────────────────────────┤");

    if let Some(voltage) = state.voltage() {
        println!("│  Voltage:     {voltage:>8.0} V            │");
    }
    if let Some(current) = state.current() {
        println!("│  Current:     {current:>8.3} A           │");
    }
    if let Some(power) = state.power_consumption() {
        println!("│  Power:       {power:>8.0} W            │");
    }
    if let Some(factor) = state.power_factor()
        && factor > 0.0
    {
        println!("│  Power Factor:{factor:>8.2}             │");
    }
    if let Some(frequency) = state.frequency() {
        println!("│  Frequency:   {frequency:>8.2} Hz           │");
    }

    println!("├─────────────────────────────────────┤");
    println!("│         Energy Consumption          │");
    println!("├─────────────────────────────────────┤");

    if let Some(today) = state.energy_today() {
        println!("│  Today:       {today:>8.3} kWh         │");
    }
    if let Some(yesterday) = state.energy_yesterday() {
        println!("│  Yesterday:   {yesterday:>8.3} kWh         │");
    }
    if let Some(total) = state.energy_total() {
        println!("│  Total:       {total:>8.3} kWh         │");
    }
    if let Some(start_time) = state.total_start_time() {
        println!("│  Since:       {start_time:<21} │");
    }

    println!("└─────────────────────────────────────┘");

    if state.power_consumption().is_none() {
        println!();
        println!("No energy data available.");
        println!("Make sure the device has energy monitoring capability.");
    }

    // Clean disconnect
    device.disconnect().await;
    broker.disconnect().await?;

    Ok(())
}
