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
//! cargo run --example energy_test -- <broker> <topic> [username] [password]
//! ```
//!
//! # Example
//!
//! ```bash
//! cargo run --example energy_test -- mqtt://192.168.1.50:1883 tasmota_plug
//! cargo run --example energy_test -- mqtt://192.168.1.50:1883 tasmota_plug user pass
//! ```

use std::env;
use tasmor_lib::{CapabilitiesBuilder, Device};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        eprintln!("Usage: {} <broker> <topic> [username] [password]", args[0]);
        eprintln!();
        eprintln!("Examples:");
        eprintln!("  cargo run --example energy_test -- mqtt://192.168.1.50:1883 tasmota_plug");
        eprintln!(
            "  cargo run --example energy_test -- mqtt://192.168.1.50:1883 tasmota_plug user pass"
        );
        std::process::exit(1);
    }

    let broker = &args[1];
    let topic = &args[2];

    println!("=== Tasmota Energy Monitor ===");
    println!("Broker: {broker}");
    println!("Device: {topic}");
    println!();

    // Build device with energy monitoring capability
    let capabilities = CapabilitiesBuilder::new().with_energy_monitoring().build();

    let mut builder = Device::mqtt(broker, topic).with_capabilities(capabilities);

    if args.len() >= 5 {
        builder = builder.with_credentials(&args[3], &args[4]);
    }

    // Build returns (device, initial_state) - energy data is already available!
    let (_device, state) = builder.build_without_probe().await?;

    // Display energy data from initial state
    println!();
    println!("┌─────────────────────────────────────┐");
    println!("│         Current Readings            │");
    println!("├─────────────────────────────────────┤");

    if let Some(voltage) = state.voltage() {
        println!("│  Voltage:     {:>8.0} V            │", voltage);
    }
    if let Some(current) = state.current() {
        println!("│  Current:     {:>8.3} A           │", current);
    }
    if let Some(power) = state.power_consumption() {
        println!("│  Power:       {:>8.0} W            │", power);
    }
    if let Some(factor) = state.power_factor() {
        if factor > 0.0 {
            println!("│  Power Factor:{:>8.2}             │", factor);
        }
    }

    println!("├─────────────────────────────────────┤");
    println!("│         Energy Consumption          │");
    println!("├─────────────────────────────────────┤");

    if let Some(today) = state.energy_today() {
        println!("│  Today:       {:>8.3} kWh         │", today);
    }
    if let Some(yesterday) = state.energy_yesterday() {
        println!("│  Yesterday:   {:>8.3} kWh         │", yesterday);
    }
    if let Some(total) = state.energy_total() {
        println!("│  Total:       {:>8.3} kWh         │", total);
    }
    if let Some(start_time) = state.total_start_time() {
        println!("│  Since:       {:<21} │", start_time);
    }

    println!("└─────────────────────────────────────┘");

    if state.power_consumption().is_none() {
        println!();
        println!("No energy data available.");
        println!("Make sure the device has energy monitoring capability.");
    }

    Ok(())
}
