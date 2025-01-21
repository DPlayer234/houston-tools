use std::fmt;
use std::hint::black_box;
use std::sync::LazyLock;

use super::*;

static DATA: LazyLock<Vec<u8>> = LazyLock::new(|| {
    IntoIterator::into_iter(0..u16::MAX)
        .flat_map(|u| u.to_le_bytes())
        .collect()
});

#[test]
fn round_trip_b256() {
    round_trip_core(&DATA, b256::to_string, b256::from_str);
}

#[test]
fn round_trip_b65536_even() {
    round_trip_core(&DATA, b65536::to_string, b65536::from_str);
}

#[test]
fn round_trip_b65536_odd() {
    assert!(DATA.len() % 2 == 0, "data length should be even");
    round_trip_core(&DATA[1..], b65536::to_string, b65536::from_str);
}

#[test]
fn min_b256() {
    let encoded = black_box("#\u{0078}&");
    let back = b256::from_str(encoded).expect("decoding failed");

    assert_eq!(back.as_slice(), &[0x78]);
}

#[test]
fn min_b65536() {
    let encoded = black_box("&\u{1020}&");
    let back = b65536::from_str(encoded).expect("decoding failed");

    assert_eq!(back.as_slice(), &[0x20, 0x10]);
}

#[test]
fn invalid_char_b256_fails() {
    let encoded = black_box("#\u{0100}&");
    b256::from_str(encoded).expect_err("U+256 is out of range");
}

#[test]
fn invalid_char_b65536_fails() {
    let encoded = black_box("%\u{10800}&");
    b65536::from_str(encoded).expect_err("U+10800 is out of range");
}

fn round_trip_core<E: fmt::Debug>(
    bytes: &[u8],
    encode: impl FnOnce(&[u8]) -> String,
    decode: impl FnOnce(&str) -> Result<Vec<u8>, E>,
) {
    let encoded = black_box(encode(bytes));
    println!("encoded[{}]", encoded.chars().count());

    let back = decode(&encoded).expect("decoding failed");

    assert_eq!(back.as_slice(), bytes);
}
