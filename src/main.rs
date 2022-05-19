use itertools::join;
use std::env;
use std::fs::File;
use std::io;
use std::io::Read;


use crate::payload::Payload;

mod config;
mod payload;


const CHUNK_SIZE: usize = 64;
fn main() -> io::Result<()> {
    let args: Vec<String> = env::args().collect();

    let mut f = File::open(config::parse_config(&args))?;
    let mut d = [0; 2*CHUNK_SIZE];

    let mut total_read = 0;
    loop {
        total_read += f.read(&mut d[total_read..])?;

        if total_read < CHUNK_SIZE {
            continue;
        }

        let mut p;
        let found;
        (_, p, found) = match payload::parse_stream_to_payload(&d) {
            Ok((i, p)) => (i, p, true),
            Err(_e) => (&d[..], Payload::default(), false),
        };

        if found {
            println!("{}", join(&*&mut p.data[0..3], ","));
            break;
        }
    }

    Ok(())
}
