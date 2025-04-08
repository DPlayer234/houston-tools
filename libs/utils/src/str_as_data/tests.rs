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

#[test]
fn round_trip_b20bit() {
    fn b20bit_range() -> impl IntoIterator<Item = u64> {
        0..=0xFFFFFu64
    }

    let data: Vec<u8> = b20bit_range()
        .into_iter()
        .flat_map(|u| *(u | (u << 20)).to_le_bytes().first_chunk::<5>().unwrap())
        .collect();

    round_trip_core(&data, b20bit::to_string, b20bit::from_str);
    round_trip_core(&data[1..], b20bit::to_string, b20bit::from_str);
    round_trip_core(&data[2..], b20bit::to_string, b20bit::from_str);
    round_trip_core(&data[3..], b20bit::to_string, b20bit::from_str);
    round_trip_core(&data[4..], b20bit::to_string, b20bit::from_str);
}

#[test]
fn round_trip_b20bit_all_chars() {
    use std::iter::once;

    fn test_range() -> impl IntoIterator<Item = char> {
        '\u{0}'..='\u{1007FF}'
    }

    // yes, this code quite literally constructs a string with _every valid unicode
    // character_ this exists as a sanity check for the behavior of
    // `char_to_code` and `code_to_char`, in particular in relation to the unsafe
    // code used. if the round trip succeeds, it must mean that there is a 1:1
    // relationship so we can't hit invalid cases there. and other tests already
    // check for out-of-range characters
    let encoded: String = once('A').chain(test_range()).chain(once('&')).collect();
    let decoded = b20bit::from_str(&encoded).expect("decoding must work");
    let reencoded = b20bit::to_string(&decoded);

    assert_eq!(encoded, reencoded);
    std::thread::sleep(std::time::Duration::from_secs(10));
}

#[test]
fn min_b20bit() {
    const CASES: &[(&str, &[u8])] = &[
        ("A\u{61820}\u{34850}&", &[0x20, 0x10, 0x36, 0x50, 0x40]),
        ("B\u{61820}\u{30850}&", &[0x20, 0x10, 0x36, 0x50]),
        ("C\u{61820}\u{30800}&", &[0x20, 0x10, 0x36]),
        ("B\u{1020}&", &[0x20, 0x10]),
        ("C\u{0020}&", &[0x20]),
        ("A&", &[]),
    ];

    for (input, output) in CASES {
        let back = b20bit::from_str(input).expect("decoding failed");
        assert_eq!(back.as_slice(), *output);
    }
}

#[test]
fn invalid_char_b20bit_fails() {
    b20bit::from_str("A\x00\u{100800}&").expect_err("invalid char code");
    b20bit::from_str("A\x00\u{1007FF}&").expect("this should be valid");
}

#[test]
fn invalid_len_b20bit_fails() {
    b20bit::from_str("A\0&").expect_err("odd count with zero trim");
    b20bit::from_str("B&").expect_err("odd count with empty str");
    b20bit::from_str("C&").expect_err("odd count with empty str");
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
