// SPDX-License-Identifier: MPL-2.0

//! Uptime retrieval example.
//!
//! Demonstrates three methods to retrieve device uptime as `std::time::Duration`:
//!
//! 1. **HTTP (punctual)**: Query device status via HTTP request
//! 2. **MQTT (punctual)**: Query device state via MQTT command
//! 3. **MQTT (subscription)**: Receive uptime via telemetry callback
//!
//! # Usage
//!
//! ```bash
//! # HTTP mode
//! cargo run --example uptime -- http <device_ip> [username] [password]
//!
//! # MQTT punctual mode
//! cargo run --example uptime -- mqtt <broker_host> <device_topic> [username] [password]
//!
//! # MQTT subscription mode (waits for telemetry)
//! cargo run --example uptime -- subscribe <broker_host> <device_topic> [username] [password]
//! ```
//!
//! # Examples
//!
//! ```bash
//! # Query uptime via HTTP (no auth)
//! cargo run --example uptime -- http 192.168.1.100
//!
//! # Query uptime via HTTP (with auth)
//! cargo run --example uptime -- http 192.168.1.100 admin password
//!
//! # Query uptime via MQTT
//! cargo run --example uptime -- mqtt 192.168.1.50 tasmota_plug
//!
//! # Subscribe and wait for telemetry uptime (runs for 5 minutes)
//! cargo run --example uptime -- subscribe 192.168.1.50 tasmota_plug user pass
//! ```

use std::env;
use std::time::Duration;
use tasmor_lib::state::DeviceState;
use tasmor_lib::subscription::Subscribable;
use tasmor_lib::{Device, MqttBroker};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 3 {
        print_usage(&args[0]);
        std::process::exit(1);
    }

    let mode = &args[1];

    match mode.as_str() {
        "http" => run_http_mode(&args).await,
        "mqtt" => run_mqtt_punctual_mode(&args).await,
        "subscribe" => run_mqtt_subscription_mode(&args).await,
        _ => {
            eprintln!("Unknown mode: {mode}");
            print_usage(&args[0]);
            std::process::exit(1);
        }
    }
}

fn print_usage(program: &str) {
    eprintln!("Usage:");
    eprintln!("  {program} http <device_ip> [username] [password]");
    eprintln!("  {program} mqtt <broker_host> <device_topic> [username] [password]");
    eprintln!("  {program} subscribe <broker_host> <device_topic> [username] [password]");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  {program} http 192.168.1.100");
    eprintln!("  {program} http 192.168.1.100 admin password");
    eprintln!("  {program} mqtt 192.168.1.50 tasmota_plug");
    eprintln!("  {program} subscribe 192.168.1.50 tasmota_plug user pass");
}

/// Formats a Duration in a human-readable format.
fn format_uptime(duration: Duration) -> String {
    let total_secs = duration.as_secs();
    let days = total_secs / 86400;
    let hours = (total_secs % 86400) / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if days > 0 {
        format!("{days}d {hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    }
}

/// Prints system info from a `DeviceState`.
fn print_system_info(state: &DeviceState, prefix: &str) {
    match state.uptime() {
        Some(uptime) => {
            println!("{prefix}Uptime: {}", format_uptime(uptime));
            println!("{prefix}  (raw: {} seconds)", uptime.as_secs());
        }
        None => {
            println!("{prefix}Uptime: not available");
        }
    }

    // Show additional system info if available
    if let Some(info) = state.system_info()
        && let Some(rssi) = info.wifi_rssi()
    {
        println!("{prefix}WiFi RSSI: {rssi} dBm");
    }
}

/// HTTP mode: Query uptime via HTTP request.
///
/// This is the simplest method - a direct HTTP request to the device.
/// No MQTT broker required, but no real-time updates.
async fn run_http_mode(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 3 {
        eprintln!("HTTP mode requires: <device_ip> [username] [password]");
        std::process::exit(1);
    }

    let device_ip = &args[2];

    println!("=== HTTP Uptime Query ===");
    println!("Device: {device_ip}");
    println!();

    // Create HTTP device builder
    let mut builder = Device::http(device_ip);

    // Add credentials if provided
    if args.len() >= 5 {
        builder = builder.with_credentials(&args[3], &args[4]);
    }

    // Build and query state (includes uptime)
    let (device, state) = builder.build().await?;

    // Print system info including uptime
    print_system_info(&state, "");

    drop(device);
    Ok(())
}

/// MQTT punctual mode: Query uptime via MQTT command.
///
/// This sends a status command to the device via MQTT and waits for the response.
/// Requires an MQTT broker.
///
/// The library automatically aggregates the multiple STATUS* messages that Tasmota
/// sends in response to Status 0, providing the same complete information as HTTP.
async fn run_mqtt_punctual_mode(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 4 {
        eprintln!("MQTT mode requires: <broker_host> <device_topic>");
        std::process::exit(1);
    }

    let broker_host = &args[2];
    let device_topic = &args[3];

    println!("=== MQTT Punctual Uptime Query ===");
    println!("Broker: {broker_host}");
    println!("Device: {device_topic}");
    println!();

    // Build broker connection
    let mut broker_builder = MqttBroker::builder().host(broker_host);

    if args.len() >= 6 {
        broker_builder = broker_builder.credentials(&args[4], &args[5]);
    }

    let broker = broker_builder.build().await?;

    // Build device - uptime is available from the aggregated Status 0 response
    let (device, state) = broker.device(device_topic).build().await?;

    // Print system info including uptime
    print_system_info(&state, "");

    // Clean disconnect
    device.disconnect().await;
    broker.disconnect().await?;

    Ok(())
}

/// MQTT subscription mode: Receive uptime via telemetry.
///
/// This subscribes to device events and waits for telemetry messages.
/// The uptime is updated periodically by Tasmota via `tele/<topic>/STATE`.
///
/// Note: Currently, the library exposes uptime via `on_connected` callback
/// (which receives the initial `DeviceState`) and via `query_state()`.
/// For continuous telemetry monitoring, consider using `on_state_changed`.
async fn run_mqtt_subscription_mode(args: &[String]) -> Result<(), Box<dyn std::error::Error>> {
    if args.len() < 4 {
        eprintln!("Subscribe mode requires: <broker_host> <device_topic>");
        std::process::exit(1);
    }

    let broker_host = &args[2];
    let device_topic = &args[3];

    println!("=== MQTT Subscription Mode ===");
    println!("Broker: {broker_host}");
    println!("Device: {device_topic}");
    println!();
    println!("Waiting for device connection and telemetry...");
    println!("(Running for 5 minutes, or press Ctrl+C to exit)");
    println!();

    // Build broker connection
    let mut broker_builder = MqttBroker::builder().host(broker_host);

    if args.len() >= 6 {
        broker_builder = broker_builder.credentials(&args[4], &args[5]);
    }

    let broker = broker_builder.build().await?;

    // Build device
    let (device, initial_state) = broker.device(device_topic).build().await?;

    // Show initial uptime if available
    if let Some(uptime) = initial_state.uptime() {
        println!(
            "[Initial] Uptime: {} ({} seconds)",
            format_uptime(uptime),
            uptime.as_secs()
        );
    }

    // Subscribe to connection events - receives DeviceState with SystemInfo
    device.on_connected(|state| {
        if let Some(uptime) = state.uptime() {
            println!(
                "[Connected] Uptime: {} ({} seconds)",
                format_uptime(uptime),
                uptime.as_secs()
            );
        }
        if let Some(info) = state.system_info()
            && let Some(rssi) = info.wifi_rssi()
        {
            println!("[Connected] WiFi RSSI: {rssi} dBm");
        }
    });

    // Subscribe to disconnection events
    device.on_disconnected(|| {
        println!("[Disconnected] Device offline");
    });

    // Subscribe to reconnection events
    let device_clone = device.clone();
    device.on_reconnected(move || {
        println!("[Reconnected] Connection restored, querying fresh state...");

        // Clone device for the async block
        let dev = device_clone.clone();
        tokio::spawn(async move {
            match dev.query_state().await {
                Ok(state) => {
                    if let Some(uptime) = state.uptime() {
                        println!(
                            "[Refreshed] Uptime: {} ({} seconds)",
                            format_uptime(uptime),
                            uptime.as_secs()
                        );
                    }
                }
                Err(e) => println!("[Refreshed] Failed to query state: {e}"),
            }
        });
    });

    // Periodic uptime refresh every 60 seconds to demonstrate punctual queries
    let device_periodic = device.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        interval.tick().await; // Skip initial tick

        loop {
            interval.tick().await;
            match device_periodic.query_state().await {
                Ok(state) => {
                    if let Some(uptime) = state.uptime() {
                        println!(
                            "[Periodic] Uptime: {} ({} seconds)",
                            format_uptime(uptime),
                            uptime.as_secs()
                        );
                    }
                }
                Err(e) => println!("[Periodic] Query failed: {e}"),
            }
        }
    });

    // Wait for 5 minutes (or until process is killed)
    tokio::time::sleep(Duration::from_secs(300)).await;
    println!();
    println!("Shutting down...");

    // Clean disconnect
    device.disconnect().await;
    broker.disconnect().await?;

    Ok(())
}
