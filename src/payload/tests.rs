// use super::*;

extern crate hex_literal;

#[cfg(test)]
mod payload_tests {
    use crate::payload::{FRAME_START, Payload, parse_stream_to_payload};
    // use std::mem::size_of;
    use hex_literal::hex;


    const FRAME_SIZE: usize = 32;
    // test data
    const ONE_GOOD_FRAME: [u8; FRAME_SIZE] = hex!(
        "424d 001c 0004 0006 0008 0004 0006 0008" 
        "0324 00ea 0036 0008 0002 0000 9700 02b7"
    );

    // const BAD_FRAME_32: [u8; FRAME_SIZE] = hex!(
    //     "BAAD F00D 0102 0304 0506 0708 090A 0B0C"
    //     "2D1E FFFE FDFC FBFA F9F8 F7F6 F5F4 1234"
    // );


    #[test]
    fn parser_returns_payload() {
        // let p = parse_stream_to_Payload(&ONE_GOOD_FRAME);
        let (_, p) = parse_stream_to_payload(&ONE_GOOD_FRAME).unwrap();
        assert!(std::mem::size_of_val(&p) == std::mem::size_of::<Payload>());

        // check payload has a start sequence
        assert!(p.start == FRAME_START);

        // the length should always be 0x1C == 32 - 4 == 28
        //      2 bytes for start; 2 fo length, so 28 bytes to be read
        assert!(p.len as usize == (0x20 - 2*std::mem::size_of::<u16>()));

        // data matches input
        let d_size_in_bytes = 12*std::mem::size_of::<u16>();
        let d: Vec<u16> = (&ONE_GOOD_FRAME[4..(4 + d_size_in_bytes)])
            .chunks_exact(2)
            .into_iter()
            .map(|x| u16::from_be_bytes([x[0], x[1]]))
            .collect();

        assert_eq!(p.data.to_vec(), d);

        // reserved data -- no validation, just existence
        assert_eq!(std::mem::size_of_val(&p.reserved_data), std::mem::size_of::<u16>());

        // checksum
        assert_eq!(p.check, u16::from_be_bytes([ONE_GOOD_FRAME[30], ONE_GOOD_FRAME[31]]));
    }

    #[test]
    fn fails_on_checksum_errors() {
        let mut f = ONE_GOOD_FRAME.clone();
        f[4] += 1;
        
        assert!(returns_error(&f));
    }

    #[test]
    fn panics_without_frame_start() {
        let mut f = ONE_GOOD_FRAME.clone();
        f[0] = 0xBA; f[1] = 0xFD;           // could be any error values, just removing 'BM'

        let r = std::panic::catch_unwind(|| 
            parse_stream_to_payload(&f)
        );

        assert!(r.is_err());
    }

    #[test]
    fn panics_with_no_length() {
        let mut f = ONE_GOOD_FRAME.clone();
        f[2] = 0x00; f[3] = 0x00;

        assert!(returns_error(&f));
    }

    #[test]
    fn ns_error_for_too_long() {
        let mut f = ONE_GOOD_FRAME.clone();
        f[2] = 0x01; f[3] = 0x00;       // try to read 256 bytes from 28 byte remainder

        assert!(returns_error(&f));
    }

    fn returns_error(f: &[u8]) -> bool {
        match parse_stream_to_payload(&f) {
            Ok(_b) => return false,
            Err(e) => {
                eprintln!("{:?}", e);
                return true;
            }
        }
    }


}