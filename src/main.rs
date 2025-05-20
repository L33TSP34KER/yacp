use std::io::Write;

use std::io::stdout;

use soapysdr::TxStream;
use std::io;
use soapysdr::Direction::{Rx, Tx};
use num_complex::Complex;

fn init_driver_sdr(channel: usize, mut num: usize, mut freq: f64) -> Option<soapysdr::Device> {
    for dev in soapysdr::enumerate("").unwrap() {
        println!("{}", dev);
        let dev = soapysdr::Device::new(dev).expect("Error opening device");
        println!("{:?}", dev.list_gains(Rx, channel));
        dev.set_frequency(soapysdr::Direction::Rx, channel,  freq, ()).expect("Failed to set frequency");
        match dev.set_gain_element(Rx, channel, "VGA", 32.0) {
            Ok(_) => println!("set VGA to 8"),
            Err(e) => {
                println!("error while setting up VGA {e}");
                break;
            }
        }
        match dev.set_gain_element(Tx, channel, "VGA", 32.0) {
            Ok(_) => println!("set VGA to 8"),
            Err(e) => {
                println!("error while setting up VGA {e}");
                break;
            }
        }
        match dev.set_gain_element(Tx, channel, "LNA", 32.0) {
            Ok(_) => println!("set LNA to 8"),
            Err(e) => {
                println!("error while setting up LNA {e}");
                break;
            }
        }
        match dev.set_bandwidth(Rx, channel, 100000.0) {
            Ok(_) => println!("set bandwidth"),
            Err(e) => {
                println!("error while setting up bandwidth {e}");
                break;
            }
        }
        match dev.set_gain_element(Rx, channel, "LNA", 32.0) {
            Ok(_) => println!("set LNA to 8"),
            Err(e) => {
                println!("error while setting up LNA {e}");
                break;
            }
        }
        dev.set_sample_rate(Rx, channel, 2_400_000.0).expect("Failed to set sample rate");
        let mut stream = dev.rx_stream::<Complex<f32>>(&[channel]).unwrap();
        let mut buf = vec![Complex::new(0.0, 0.0); stream.mtu().unwrap()];
        stream.activate(None).expect("failed to activate stream");
        dev.set_frequency(soapysdr::Direction::Rx, channel,  freq, ()).expect("Failed to set frequency");
        //stream.deactivate(None).expect("failed to deactivate");
        return Some(dev);
    }
    None
}
fn text_to_iq(text: &str) -> Vec<Complex<f32>> {
    let mut iq_data = Vec::with_capacity(text.len() * 8); // More samples per character for better reception
    
    for c in text.chars() {
        let byte = c as u8;
        // Generate more distinct IQ patterns for each character
        for i in 0..8 {
            let bit = (byte >> i) & 1;
            if bit == 1 {
                // Signal for bit 1
                iq_data.push(Complex::new(0.7, 0.7));
                iq_data.push(Complex::new(-0.7, 0.7));
                iq_data.push(Complex::new(-0.7, -0.7));
                iq_data.push(Complex::new(0.7, -0.7));
            } else {
                // Signal for bit 0
                iq_data.push(Complex::new(0.7, -0.7));
                iq_data.push(Complex::new(0.7, 0.7));
                iq_data.push(Complex::new(-0.7, 0.7));
                iq_data.push(Complex::new(-0.7, -0.7));
            }
        }
        
        // Add separator between characters
        iq_data.push(Complex::new(0.0, 0.0));
        iq_data.push(Complex::new(0.0, 0.0));
    }
    
    // Add start/end markers
    let preamble = vec![
        Complex::new(1.0, 1.0), Complex::new(1.0, -1.0),
        Complex::new(-1.0, -1.0), Complex::new(-1.0, 1.0),
        Complex::new(1.0, 1.0), Complex::new(1.0, -1.0),
        Complex::new(-1.0, -1.0), Complex::new(-1.0, 1.0),
    ];
    
    let mut result = preamble.clone();
    result.extend(iq_data);
    result.extend(preamble);
    
    result
}

fn iq_to_text(samples: &[Complex<f32>]) -> String {
    // First find the preamble
    if samples.len() < 16 {
        return String::from("[Signal too short]");
    }
    
    // Look for preamble pattern
    let mut start_idx = 0;
    'outer: for i in 0..samples.len()-8 {
        for j in 0..8 {
            if (samples[i+j].re.abs() < 0.5) || (samples[i+j].im.abs() < 0.5) {
                continue 'outer;
            }
        }
        start_idx = i + 8; // Skip preamble
        break;
    }
    
    // If we couldn't find a preamble
    if start_idx == 0 {
        return String::from("[No preamble detected]");
    }
    
    // Find the end preamble
    let mut end_idx = samples.len();
    'outer: for i in (start_idx+32..samples.len()-8).rev() {
        for j in 0..8 {
            if (samples[i+j].re.abs() < 0.5) || (samples[i+j].im.abs() < 0.5) {
                continue 'outer;
            }
        }
        end_idx = i;
        break;
    }
    
    // Process the data between preambles
    let data_samples = &samples[start_idx..end_idx];
    let mut text = String::new();
    let mut current_byte = 0u8;
    let mut bit_count = 0;
    
    let mut i = 0;
    while i < data_samples.len() {
        // Skip separator zeroes
        if data_samples[i].re.abs() < 0.3 && data_samples[i].im.abs() < 0.3 {
            i += 1;
            continue;
        }
        
        // Need at least 4 samples for a bit
        if i + 4 > data_samples.len() {
            break;
        }
        
        // Detect bit pattern (simplified)
        let bit = if data_samples[i].re > 0.0 && data_samples[i].im > 0.0 { 1 } else { 0 };
        
        current_byte |= bit << bit_count;
        bit_count += 1;
        
        // Move to next bit
        i += 4;
        
        // Complete byte
        if bit_count == 8 {
            text.push(current_byte as char);
            current_byte = 0;
            bit_count = 0;
        }
    }
    
    text
}

fn calc_power(samples: &[Complex<f32>]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    
    let sum: f32 = samples.iter()
        .map(|s| s.norm_sqr())
        .sum();
    
    sum / samples.len() as f32
}

fn emit(device: &soapysdr::Device, channel: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("Setting up transmitter...");
    let mut tx_stream = device.tx_stream(&[channel])?;
    
    // Set up buffer with test message
    let text = "HELLO WORLD! THIS IS A TEST MESSAGE";
    let payload = text_to_iq(text);
    
    println!("Transmitting message: \"{}\"", text);
    
    // Give the receiver time to start up
    std::thread::sleep(std::time::Duration::from_millis(500));
    
    // Send with generous timeout
    tx_stream.activate(None)?;
    
    // Try multiple transmissions for reliability
    for attempt in 1..=3 {
        println!("Transmission attempt {}...", attempt);
        let written = tx_stream.write(&[&payload[..]], Some(5_000_000), false, 500000)?;
        println!("Sent {} IQ samples ({} text characters)", written, text.len());
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    
    tx_stream.deactivate(None)?;
    Ok(())
}

fn receive(device: &soapysdr::Device, channel: usize) -> Result<(), Box<dyn std::error::Error>> {
    println!("Setting up receiver...");
    let mut rx_stream = device.rx_stream(&[channel])?;
    
    // Use a larger buffer for the receive side
    let mtu = rx_stream.mtu().unwrap_or(16384);
    let mut buf = vec![Complex::new(0.0, 0.0); mtu];
    
    println!("Activating receiver...");
    rx_stream.activate(None)?;
    
    println!("Listening for transmissions...");
    let mut received_data = Vec::new();
    
    // Listen for longer to catch the transmission
    for _ in 0..30 {
        match rx_stream.read(&mut [&mut buf[..]], 1_000_000) {
            Ok(len) => {
                let power = calc_power(&buf[..len]);
                println!("Received {} samples, power: {:.6}", len, power);
                
                // Only process if signal power is meaningful
                if power > 0.05 {
                    // Store all samples above threshold for later processing
                    received_data.extend_from_slice(&buf[..len]);
                    println!("Captured data chunk (power: {:.6})", power);
                }
            },
            Err(e) => {
                println!("Read error: {}", e);
            }
        }
        
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
    
    // Now process all the collected data
    if !received_data.is_empty() {
        let text = iq_to_text(&received_data);
        println!("Decoded message: \"{}\"", text);
    } else {
        println!("No significant signal detected");
    }
    
    rx_stream.deactivate(None)?;
    Ok(())
}


fn main() {
    let channel: usize = 0;
    let num: usize = 0;
    let freq =  433690000.0;

    match init_driver_sdr(channel, num, freq) {
        Some(device) => {
            loop {
                let mut usrinput = String::new();
                print!("yacp> ");
                let _ = stdout().flush();
                io::stdin()
                    .read_line(&mut usrinput)
                    .expect("?? wtf");
                if usrinput.trim() == "help" {
                    println!("===== HELP =====");
                    println!(" - transmit");
                    println!(" - receive");
                    println!(" - exit");
                }
                if usrinput.trim() == "exit" {
                    return;
                }

                if usrinput.trim() == "transmit" {
                    device.set_gain(Tx, channel, 51.0).ok();
                    std::thread::sleep_ms(1000);
                    emit(&device, channel);
                    std::thread::sleep_ms(1000);
                } else if usrinput.trim() == "receive" {
                    println!("receive");
                }
            }
        }
        None => {
            println!("No Device found");
        }
    }
}
