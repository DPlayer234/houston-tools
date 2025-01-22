use std::fmt;

use super::*;

#[test]
fn round_trip_b256() {
    let data: Vec<u8> = IntoIterator::into_iter(0..u8::MAX).collect();

    round_trip_core(&data, b256::to_string, b256::from_str);
}

#[test]
fn round_trip_b65536() {
    let data: Vec<u8> = IntoIterator::into_iter(0..u16::MAX)
        .flat_map(|u| u.to_le_bytes())
        .collect();

    round_trip_core(&data, b65536::to_string, b65536::from_str);
    round_trip_core(&data[1..], b65536::to_string, b65536::from_str);
}

#[test]
fn min_b256() {
    let encoded = "#\u{0078}&";
    let back = b256::from_str(encoded).expect("decoding failed");

    assert_eq!(back.as_slice(), &[0x78]);
}

#[test]
fn min_b65536() {
    let encoded = "&\u{1020}&";
    let back = b65536::from_str(encoded).expect("decoding failed");

    assert_eq!(back.as_slice(), &[0x20, 0x10]);
}

#[test]
fn invalid_char_b256_fails() {
    b256::from_str("#\u{0100}&").expect_err("U+256 is out of range");
}

#[test]
fn invalid_char_b65536_fails() {
    b65536::from_str("%\u{10800}&").expect_err("U+10800 is out of range");
}

#[test]
fn invalid_len_b65536_fails() {
    b65536::from_str("%&").expect_err("odd count with empty str");
}

fn round_trip_core<E: fmt::Debug>(
    bytes: &[u8],
    encode: impl FnOnce(&[u8]) -> String,
    decode: impl FnOnce(&str) -> Result<Vec<u8>, E>,
) {
    let encoded = encode(bytes);
    println!("encoded[{} in {}]", encoded.chars().count(), encoded.len());

    let back = decode(&encoded).expect("decoding failed");

    assert_eq!(back.as_slice(), bytes);
}
