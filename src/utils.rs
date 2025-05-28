use crate::protocols::ReceiveError;
use crate::constants::samples_per_bit;


use num_complex::Complex;
use soapysdr::Direction::{Rx, Tx};

pub fn init_driver_sdr(channel: usize, _num: usize, freq: f64) -> Option<soapysdr::Device> {
    for dev in soapysdr::enumerate("").unwrap() {
        println!("{}", dev);
        let dev = soapysdr::Device::new(dev).expect("Error opening device");
        println!("{:?}", dev.list_gains(Rx, channel));

        dev.set_frequency(soapysdr::Direction::Rx, channel, freq, ())
            .expect("Failed to set RX frequency");
        dev.set_frequency(soapysdr::Direction::Tx, channel, freq, ())
            .expect("Failed to set TX frequency");

        dev.set_sample_rate(Rx, channel, 1_000_000.0)
            .expect("Failed to set RX sample rate");
        dev.set_sample_rate(Tx, channel, 1_000_000.0)
            .expect("Failed to set TX sample rate");

        if let Err(e) = dev.set_bandwidth(Rx, channel, 100000.0) {
            println!("Warning: Could not set RX bandwidth: {}", e);
        }
        if let Err(e) = dev.set_bandwidth(Tx, channel, 100000.0) {
            println!("Warning: Could not set TX bandwidth: {}", e);
        }

        if let Err(e) = dev.set_gain_element(Rx, channel, "VGA", 20.0) {
            println!("Warning: Could not set RX VGA: {}", e);
        }
        if let Err(e) = dev.set_gain_element(Rx, channel, "LNA", 20.0) {
            println!("Warning: Could not set RX LNA: {}", e);
        }
        if let Err(e) = dev.set_gain_element(Tx, channel, "VGA", 20.0) {
            println!("Warning: Could not set TX VGA: {}", e);
        }

        println!("SDR initialized successfully");
        return Some(dev);
    }
    None
}

pub fn text_to_iq(text: &str) -> Vec<Complex<f32>> {
    let mut iq_data = Vec::new();

    for _ in 0..20 {
        for _ in 0..samples_per_bit {
            iq_data.push(Complex::new(0.8, 0.0));
        }
        for _ in 0..samples_per_bit {
            iq_data.push(Complex::new(0.0, 0.0));
        }
    }

    let sync_pattern = [1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 1, 0, 0, 0];
    for &bit in &sync_pattern {
        for _ in 0..samples_per_bit {
            if bit == 1 {
                iq_data.push(Complex::new(0.9, 0.0));
            } else {
                iq_data.push(Complex::new(0.0, 0.0));
            }
        }
    }

    for c in text.chars() {
        let byte = c as u8;
        println!("Encoding '{}' (0x{:02x})", c, byte);

        for i in 0..8 {
            let bit = (byte >> i) & 1;
            for _ in 0..samples_per_bit {
                if bit == 1 {
                    iq_data.push(Complex::new(0.8, 0.0));
                } else {
                    iq_data.push(Complex::new(0.0, 0.0));
                }
            }
        }

        for _ in 0..(samples_per_bit / 2) {
            iq_data.push(Complex::new(0.0, 0.0));
        }
    }

    for _ in 0..10 {
        for _ in 0..samples_per_bit {
            iq_data.push(Complex::new(0.0, 0.0));
        }
    }

    iq_data
}

pub fn iq_to_text(samples: &[Complex<f32>]) -> Result<String, ReceiveError> {
    println!("Decoding {} samples", samples.len());

    if samples.len() < 100 {
        return Err(ReceiveError::NotEnough);
    }

    let mut power: Vec<f32> = samples.iter().map(|s| s.norm()).collect();

    let max_power = power.iter().fold(0.0f32, |a, &b| a.max(b));
    if max_power < 0.01 {
        return Err(ReceiveError::Weak);
    }

    for p in &mut power {
        *p /= max_power;
    }

    let threshold = 0.3;
    let bits: Vec<u8> = power
        .iter()
        .map(|&p| if p > threshold { 1 } else { 0 })
        .collect();

    println!(
        "Converted to {} bits, looking for sync pattern...",
        bits.len()
    );

    let sync_pattern = [1, 0, 1, 0, 1, 1, 0, 0, 1, 1, 1, 0, 0, 0];
    let mut sync_found = false;
    let mut data_start = 0;

    for start in 1000..bits
        .len()
        .saturating_sub(sync_pattern.len() * samples_per_bit)
    {
        let mut matches = 0;
        for (i, &expected_bit) in sync_pattern.iter().enumerate() {
            let bit_start = start + i * samples_per_bit;
            let bit_end = (bit_start + samples_per_bit).min(bits.len());
            if bit_end <= bit_start {
                break;
            }

            let ones = bits[bit_start..bit_end].iter().sum::<u8>();
            let actual_bit = if ones > (samples_per_bit as u8 / 2) {
                1
            } else {
                0
            };

            if actual_bit == expected_bit {
                matches += 1;
            }
        }

        if matches >= sync_pattern.len() - 2 {
            sync_found = true;
            data_start = start + sync_pattern.len() * samples_per_bit;
            println!(
                "Sync pattern found at position {}, data starts at {}",
                start, data_start
            );
            break;
        }
    }

    if !sync_found {
        println!("No sync pattern found, trying to find data after preamble...");
        data_start = 2000;
    }

    let mut text = String::new();
    let mut pos = data_start;

    while pos + 8 * samples_per_bit < bits.len() {
        let mut byte = 0u8;
        let mut valid_bits = 0;

        for bit_pos in 0..8 {
            let bit_start = pos + bit_pos * samples_per_bit;
            let bit_end = (bit_start + samples_per_bit).min(bits.len());

            if bit_end <= bit_start {
                break;
            }

            let ones = bits[bit_start..bit_end].iter().sum::<u8>() as usize;
            let zeros = (bit_end - bit_start) - ones;

            if ones > zeros && ones > samples_per_bit / 3 {
                byte |= 1 << bit_pos;
                valid_bits += 1;
            } else if zeros > ones && zeros > samples_per_bit / 3 {
                valid_bits += 1;
            }
        }

        if valid_bits >= 6 {
            if byte >= 32 && byte <= 126 {
                text.push(byte as char);
                println!("Decoded: '{}' (0x{:02x})", byte as char, byte);
            } else if byte == 0 {
                break;
            }
        }

        pos += 8 * samples_per_bit + samples_per_bit / 2;

        if text.len() > 100 {
            break;
        }
    }

    if text.is_empty() {
        return Err(ReceiveError::NoValid);
    } else {
        return Ok(text);
    }
}
