use soapysdr::Direction::{Rx, Tx};
use num_complex::Complex;

fn main() {
    let channel: usize = 0;
    let mut num: usize = 0;
    let mut freq =  433690000.0;
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
        for i in 0..20{
            dev.set_frequency(soapysdr::Direction::Rx, channel,  freq, ()).expect("Failed to set frequency");
            for i in 0..10{
                let read_size =buf.len();
                let len = stream.read(&mut [&mut buf[..read_size]], 1_000_000).expect("read failed");
                let power: f32 = buf.iter().map(|s| s.re * s.re + s.im * s.im).sum::<f32>() / buf.len() as f32;
                println!("Average power: {}", power);
            }
        }
        for data in buf {
            let magnitude = (data.re * data.re + data.im * data.im).sqrt();
            println!("{:X}", (magnitude * 10000.0) as u32);
        }
        stream.deactivate(None).expect("failed to deactivate");
    }
}
