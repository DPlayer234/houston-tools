use std::fmt;
use std::hint::black_box;

use super::*;

static DATA: &[u8] = {
    const MAX: usize = u16::MAX as usize;
    const fn create_data() -> [u16; MAX] {
        let mut result = [0u16; MAX];
        let mut index = 0usize;

        #[allow(clippy::cast_possible_truncation)]
        while index < result.len() {
            result[index] = index as u16;
            index += 1;
        }

        result
    }

    unsafe {
        crate::mem::as_bytes(&create_data())
    }
};

#[test]
fn round_trip_b256() {
    round_trip_core(
        DATA,
        to_b256,
        from_b256
    );
}

#[test]
fn round_trip_b65536_even() {
    round_trip_core(
        DATA,
        to_b65536,
        from_b65536
    );
}

#[test]
fn round_trip_b65536_odd() {
    round_trip_core(
        &DATA[1..],
        to_b65536,
        from_b65536
    );
}

#[test]
fn min_b256() {
    let encoded = black_box("#\u{0078}&");
    let back = from_b256(encoded).expect("decoding failed");

    assert_eq!(back.as_slice(), &[0x78]);
}

#[test]
fn min_b65536() {
    let encoded = black_box("&\u{1020}&");
    let back = from_b65536(encoded).expect("decoding failed");

    assert_eq!(back.as_slice(), &[0x20, 0x10]);
}

#[test]
fn invalid_char_b256_fails() {
    let encoded = black_box("%\u{10800}&");
    from_b65536(encoded).expect_err("U+10800 is out of range");
}

#[test]
fn invalid_char_b65536_fails() {
    let encoded = black_box("#\u{0100}&");
    from_b256(encoded).expect_err("U+256 is out of range");
}

fn round_trip_core<E: fmt::Debug>(bytes: &[u8], encode: impl FnOnce(&[u8]) -> String, decode: impl FnOnce(&str) -> Result<Vec<u8>, E>) {
    let encoded = black_box(encode(bytes));
    println!("encoded[{}]", encoded.chars().count());

    let back = decode(&encoded).expect("decoding failed");

    assert_eq!(back.as_slice(), bytes);
}
