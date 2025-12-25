// SPDX-License-Identifier: MPL-2.0

//! Energy monitoring example.
//!
//! Demonstrates how to query and display energy consumption data from
//! Tasmota devices with power monitoring (smart plugs, energy meters).
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
use tasmor_lib::command::EnergyCommand;
use tasmor_lib::response::EnergyResponse;
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

    let device = builder.build_without_probe().await?;

    // Query energy data (Status 10)
    println!("Querying energy data...");
    let cmd = EnergyCommand::Get;
    let response = device.send_command(&cmd).await?;

    // Parse the response using serde
    let energy_response: EnergyResponse = serde_json::from_str(response.body())?;

    if let Some(energy) = energy_response.energy() {
        println!();
        println!("┌─────────────────────────────────────┐");
        println!("│         Current Readings            │");
        println!("├─────────────────────────────────────┤");
        println!("│  Voltage:     {:>8} V            │", energy.voltage);
        println!("│  Current:     {:>8.3} A           │", energy.current);
        println!("│  Power:       {:>8} W            │", energy.power);
        if energy.factor > 0.0 {
            println!("│  Power Factor:{:>8.2}             │", energy.factor);
        }
        println!("├─────────────────────────────────────┤");
        println!("│         Energy Consumption          │");
        println!("├─────────────────────────────────────┤");
        println!("│  Today:       {:>8.3} kWh         │", energy.today);
        println!("│  Yesterday:   {:>8.3} kWh         │", energy.yesterday);
        println!("│  Total:       {:>8.3} kWh         │", energy.total);
        if let Some(start_time) = &energy.total_start_time {
            println!("│  Since:       {:<21} │", start_time);
        }
        println!("└─────────────────────────────────────┘");
    } else {
        println!("No energy data available.");
        println!("Make sure the device has energy monitoring capability.");
    }

    Ok(())
}
