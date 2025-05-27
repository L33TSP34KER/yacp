mod utils;
mod protocols;
mod transmition;

use transmition::emit;
use transmition::receive;

use std::io::Write;
use std::io::stdout;
use std::io;

fn main() {
    let channel: usize = 0;
    let num: usize = 0;
    let freq = 433690000.0;

    match utils::init_driver_sdr(channel, num, freq) {
        Some(device) => {
            println!("SDR device ready. Type 'help' for commands.");
            loop {
                let mut usrinput = String::new();
                print!("yacp> ");
                let _ = stdout().flush();
                io::stdin()
                    .read_line(&mut usrinput)
                    .expect("Failed to read input");
                
                match usrinput.trim() {
                    "help" => {
                        println!("===== HELP =====");
                        println!(" - transmit  : Send a text message");
                        println!(" - receive   : Listen for messages");
                        println!(" - exit      : Quit program");
                    },
                    "exit" => {
                        println!("Goodbye!");
                        return;
                    },
                    "transmit" => {
                        if let Err(e) = emit(&device, channel) {
                            println!("Transmission failed: {}", e);
                        }
                    },
                    "receive" => {
                        if let Err(e) = receive(&device, channel) {
                            println!("Receive failed: {}", e);
                        }
                    },
                    _ => {
                        println!("Unknown command. Type 'help' for available commands.");
                    }
                }
            }
        }
        None => {
            println!("No SDR device found");
        }
    }
}
