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
    text.bytes()
        .flat_map(|b| {
            // Each byte becomes 8 samples (bit by bit)
            (0..8).map(move |i| {
                let bit = (b >> (7 - i)) & 1;
                // Bit 1 => Complex { re: 1.0 }, Bit 0 => Complex { re: -1.0 }
                if bit == 1 {
                    Complex::new(1.0, 0.0)
                } else {
                    Complex::new(-1.0, 0.0)
                }
            })
        })
        .collect()
}

fn iq_to_text(iq_samples: &[Complex<f32>]) -> String {
    // Collect the bits from the IQ samples
    let bits: Vec<u8> = iq_samples
        .iter()
        .map(|sample| {
            // Check if the real part of the sample is positive or negative
            if sample.re > 0.0 {
                1
            } else {
                0
            }
        })
        .collect();

    // Convert the bits back into bytes and then to a string
    let mut result = Vec::new();
    for chunk in bits.chunks(8) {
        // Ensure we only deal with complete bytes (8 bits)
        let byte = chunk.iter().fold(0, |acc, &bit| (acc << 1) | bit);
        result.push(byte);
    }

    // Convert bytes back into characters and then into a string
    String::from_utf8(result).unwrap()
}

fn calc_power(samples: &[Complex<f32>]) -> f32 {
    samples.iter()
        .map(|s| s.re * s.re + s.im * s.im)
        .sum::<f32>() / samples.len() as f32
}

fn emit(device: &soapysdr::Device, tx_stream: &mut TxStream<Complex<f32>>, channel: usize) {
                                                       //
    let text: &str = "A";   // Gen Z vibe
    let mut payload: Vec<Complex<f32>> = text_to_iq(text);
    let written = tx_stream
        .write(&[&payload[..]], Some(1_000_000), true, 100000)
        .expect("TX write failed");
    println!("Sent {} bytes of text", written);
}

fn receive(device: soapysdr::Device, channel: usize) {
    let mut rx_stream = device.rx_stream::<Complex<f32>>(&[channel])
        .expect("Failed to create RX stream");

    let mut buf = vec![Complex::new(0.0, 0.0); rx_stream.mtu().unwrap()];

    rx_stream.activate(None).expect("Failed to activate RX stream");

    for _ in 0..10 {
        let read_len = buf.len();
        let len = rx_stream.read(&mut [&mut buf[..read_len]], 1_000_000)
            .expect("Read failed");
        let power = calc_power(&buf[..len]);
        println!("Power: {}", power);

        if power > 0.001 {
            let text = iq_to_text(&buf[..len]); // your decoder func from earlier
            println!(" Received: {}", text);
        } else {
            println!(" Low power, skipping...");
        }
    }

    rx_stream.deactivate(None).expect("Failed to deactivate RX stream");
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
                    let mut tx_stream = device.tx_stream::<Complex<f32>>(&[channel])
                        .expect("Failed to create TX stream");
                    tx_stream.activate(None).expect("Failed to activate TX");
                    std::thread::sleep_ms(1000);
                    emit(&device, &mut tx_stream, channel);
                    std::thread::sleep_ms(1000);
                    tx_stream.deactivate(None).expect("Failed to deactivate TX");
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
