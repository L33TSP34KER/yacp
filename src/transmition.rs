use soapysdr::Direction::{Rx, Tx};

use num_complex::Complex;

use crate::utils::iq_to_text;
use crate::utils::text_to_iq;

use std::io;
use std::io::Write;
use std::io::stdout;

pub fn emit(device: &soapysdr::Device, channel: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("Setting up transmitter...");

    if let Err(e) = device.set_gain(Tx, channel, 40.0) {
        println!("Warning: Could not set overall TX gain: {}", e);
    }

    let mut tx_stream = device.tx_stream(&[channel])?;

    print!("Enter message to transmit: ");
    let _ = stdout().flush();
    let mut message = String::new();
    io::stdin().read_line(&mut message)?;
    let message = message.trim();

    if message.is_empty() {
        println!("No message entered");
        return Ok(());
    }

    let payload = text_to_iq(message);
    println!("Transmitting: \"{}\" ({} samples)", message, payload.len());

    tx_stream.activate(None)?;

    std::thread::sleep(std::time::Duration::from_millis(200));
    println!("Transmitting...");
    let timeout_us = 10_000_000;
    for _ in 0..520 {
        match tx_stream.write(&[&payload[..]], Some(timeout_us), false, 100000) {
            Ok(written) => println!("Successfully transmitted {} samples", written),
            Err(e) => println!("Transmission error: {}", e),
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    tx_stream.deactivate(None)?;
    println!("Transmission complete");
    Ok(())
}

pub fn receive(
    device: &soapysdr::Device,
    channel: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Setting up receiver...");

    if let Err(e) = device.set_gain(Rx, channel, 40.0) {
        println!("Warning: Could not set overall RX gain: {}", e);
    }

    let mut rx_stream = device.rx_stream(&[channel])?;

    let buffer_size = 8192;
    let mut buf = vec![Complex::new(0.0, 0.0); buffer_size];

    println!("Listening");
    rx_stream.activate(None)?;

    let mut all_samples = Vec::new();
    let mut consecutive_timeouts = 0;

    for iteration in 0..200 {
        match rx_stream.read(&mut [&mut buf[..]], 200_000) {
            Ok(len) => {
                consecutive_timeouts = 0;
                if len > 0 {
                    let power: f32 =
                        buf[..len].iter().map(|s| s.norm_sqr()).sum::<f32>() / len as f32;

                    if iteration % 10 == 0 || power > 0.001 {
                        println!("Read {} samples, power: {:.6}", len, power);
                    }

                    all_samples.extend_from_slice(&buf[..len]);

                    if all_samples.len() > 5_000_000 {
                        break;
                    }
                }
            }
            Err(_) => {
                consecutive_timeouts += 1;
                if consecutive_timeouts >= 10 {
                    break;
                }
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    rx_stream.deactivate(None)?;

    if all_samples.is_empty() {
        println!("No samples received");
        return Ok(());
    }

    println!("Processing samples...");

    let total_power: f32 =
        all_samples.iter().map(|s| s.norm_sqr()).sum::<f32>() / all_samples.len() as f32;
    println!("Average signal power: {:.6}", total_power);

    let decoded = iq_to_text(&all_samples);

    match decoded {
        Err(e) => println!("Error: {:}\n", e.to_string()),
        Ok(text) => println!("Decoded result: \"{}\"", text),
    }

    Ok(())
}
