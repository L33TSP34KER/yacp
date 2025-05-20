use soapysdr::Direction::{Rx, Tx};
use num_complex::Complex;

fn init_driver_sdr(channel: usize, mut num: usize, mut freq: f64) -> soapysdr::Device {
    for dev in soapysdr::enumerate("").unwrap() {
        println!("{}", dev);
        let dev = soapysdr::Device::new(dev).expect("Error opening device");
        println!("{:?}", dev.list_gains(Rx, channel));
        dev.set_frequency(soapysdr::Direction::Rx, channel,  433690000.0, ()).expect("Failed to set frequency");
        match dev.set_gain_element(Rx, channel, "VGA", 32.0) {
            Ok(_) => println!("set VGA to 8"),
            Err(e) => {
                println!("error while setting up VGA {e}");
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
    }
    return dev;
}

fn main() {
    let channel: usize = 0;
    let num: usize = 0;
    let freq =  433690000.0;
    match init_driver_sdr(channel, num, freq) {

    }
}
