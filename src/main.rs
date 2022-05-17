use std::env;
use std::fs::File;
use std::io::BufReader;
use std::io::Read;

use crate::payload::Payload;


mod config;
// mod observer;
mod payload;


fn main() {
    let args: Vec<String> = env::args().collect();

    let filename= config::parse_config(&args);
    println!("reading file: {}", filename);

    let f = File::open(filename).expect("error opening file");
    let mut reader = BufReader::new(f);

    const CHUNK_SIZE: usize = 2048;
    let mut d: [u8; CHUNK_SIZE] = [0; CHUNK_SIZE];

    loop {
        let mut bytes_read = match reader.read(&mut d) {
            Ok(n) => n,
            Err(e) => 0,
        };
        if bytes_read <= 0 {
            break;
        }

        let mut input: &[u8] = &d;
        let mut bytes_remaining = bytes_read;
        loop {
            let input_len = std::mem::size_of_val(input);
            let mut p = Payload::default();
            (input, p) = payload::parse_stream_to_payload(&input).expect("error parsing");
            println!("{:?}", p.data);
            
            bytes_remaining -= input_len - std::mem::size_of_val(input);
            if std::mem::size_of_val(input) < std::mem::size_of::<Payload>() ||
                bytes_remaining < std::mem::size_of::<Payload>() {
                
                break;
            }
        }
    }
}
