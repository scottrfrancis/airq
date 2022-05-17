#[cfg(test)]
extern crate nom;

mod tests;

use nom::{
    Err,
    IResult,
    bytes::streaming::tag, 
    bytes::streaming::take,
    number::streaming::be_u16
};


// reading from PMS5003 particle sensor
//  see datasheet:  https://cdn-shop.adafruit.com/product-files/3686/plantower-pms5003-manual_v2-3.pdf
#[derive(Default, Debug)]
pub struct Payload {
    start: u16,     // is always 'BM'
    len: u16,           // length of payload -- constant -- 0x1C

    pub data: [u16; 12],    // 12 16-bit readings for various PM concentrations
    reserved_data: u16, // seems to always be 0x9700 -- but no doc

    check: u16
}

pub const FRAME_START: u16 = 0x424D;        // 'BM'

fn start_tag_parser(s: &[u8]) -> IResult<&[u8], &[u8]> {
    tag(FRAME_START.to_be_bytes())(s)
}
fn u16_parser(s: &[u8]) -> IResult<&[u8], u16> {
    be_u16(s)
}
fn take_n_bytes(s: &[u8], n: usize) -> IResult<&[u8], &[u8]> {
    take(n)(s)
}
pub fn parse_stream_to_payload(input: &[u8]) -> IResult<&[u8], Payload> {
    let mut p = Payload::default();

    let (body, _) = start_tag_parser(input)
        .expect("Frame Start not Found");
    p.start = FRAME_START;

    let (body, len) = u16_parser(body)
        .expect("can't read frame length");
    if len != 0x1C {
        eprintln!("bad length");
        return Err(Err::Error(nom::error::Error { input: body, code: nom::error::ErrorKind::LengthValue }));
    }
    p.len = len;
    // len is number of bytes *remaining* in body to read
    let d_size = std::mem::size_of_val(&p.data);
    let (body, d) = take_n_bytes(body, d_size)
        .expect("can't read data");

    let vd: Vec<u16> = d
        .chunks_exact(std::mem::size_of::<u16>())
        .into_iter()
        .map(|w| u16::from_be_bytes([w[0],w[1]]))
        .collect();
    p.data = vd.try_into()
        .expect("error reading data");
    
    // copy the reserved field
    let (body, reserved) = u16_parser(body)
        .expect("can't read reserved word");
    p.reserved_data = reserved;

    // compute checksum
    p.check = checksum(&p);
    let (body, check) = u16_parser(body)
        .expect("can't read checksum");
    if p.check != check {
        eprintln!("checksums don't match!");
        return Err(Err::Error(nom::error::Error { input: body, code: nom::error::ErrorKind::Fail }));
    }

    Ok((body, p))
}

// checksum is a byte-wise checksum... need to split words
fn checksum(p: &Payload) -> u16 {
    let mut sum = 0;
    sum += add_hi_lo_bytes(p.start);
    sum += add_hi_lo_bytes(p.len);
    for x in p.data.iter() {
        sum += add_hi_lo_bytes(*x);
    }
    sum += add_hi_lo_bytes(p.reserved_data);

    sum
}

fn add_hi_lo_bytes(x: u16) -> u16 {
    ((x & 0xFF00) >> 8) + (x & 0x00FF)
}