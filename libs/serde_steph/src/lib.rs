//! # Short Transport Encoding PBinary HFormat
//!
//! _Can you tell this isn't supposed to be an acronym?_
//!
//! Custom binary serialization format, vaguely inspired by [BARE][^bare]. This
//! format is not self-describing and as such deserializing any is disallowed.
//!
//! The types in this serialization format are as follows:
//!
//! - `byte`: single output byte
//! - `uint`: unsigned LEB128 integer
//! - `sint`: signed LEB128 integer
//! - `list`: sequence of values with `uint`-length prefix
//! - `tuple`: sequence of values without length prefix but known length
//! - `enum`: variant index as `uint` followed by variant data
//! - `map`: `list` of key-value 2-tuples
//!
//! Rust types map to these as follows:
//!
//! - `byte`: [`u8`], [`i8`], and [`bool`] (as 0 or 1)
//! - `uint`: [`u16`], [`u32`], [`u64`], [`u128`], [`usize`], [`char`]
//! - `sint`: [`i16`], [`i32`], [`i64`], [`i128`], [`isize`]
//! - `list`: variable length sequences, f.e. slices, [`str`], and [`Vec`]
//! - `tuple`: arrays, tuples, regular and tuple structs, and enum variant data.
//!   empty tuples, structs, and arrays will be 0-tuples.
//! - `enum`: enums, no matter their data
//! - `map`: map-like sequences, f.e. [`HashMap`](std::collections::HashMap)
//!
//! [bare]: <https://baremessages.org/>
//! [^bare]: No, I did not really read the spec and the output likely isn't compatible.

pub mod de;
mod leb128;
pub mod ser;

pub use de::{from_reader, from_slice};
pub use ser::{to_vec, to_writer};

#[cfg(test)]
mod tests {
    // some tests that ensure round-tripping values works as expected and
    // additionally checks that types that should have the same binary
    // representation actually do
    use std::borrow::Cow;
    use std::fmt;

    use serde::{Deserialize, Serialize};

    use super::*;

    fn round_trip<T>(value: &T) -> Vec<u8>
    where
        T: PartialEq + fmt::Debug + Serialize + serde::de::DeserializeOwned,
    {
        let buf = to_vec(value).expect("serializing must work");
        let rev: T = from_slice(&buf).expect("deserializing must work");
        assert_eq!(*value, rev, "serialization messed up data");
        buf
    }

    fn assert_all_equal(iter: impl IntoIterator<Item = Vec<u8>>) {
        let peek = iter.into_iter();
        let mut peek = peek.peekable();

        while let Some(item) = peek.next() {
            if let Some(next) = peek.peek() {
                assert_eq!(item, *next, "all serialized forms must be equal");
            }
        }
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Unit;

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct NewType(u64);

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Struct {
        a: i32,
        b: u16,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct Tuple(i32, u16);

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    enum Enum {
        Unit,
        NewType(u64),
        Struct { a: i32, b: u16 },
        Tuple(i32, u16),
    }

    #[test]
    fn round_trip_unit() {
        assert_all_equal([round_trip(&()), round_trip(&[0u64; 0]), round_trip(&Unit)]);
        round_trip(&Enum::Unit);
    }

    #[test]
    fn round_trip_one_tuple() {
        assert_all_equal([
            round_trip(&(87654321u64,)),
            round_trip(&[87654321u64]),
            round_trip(&NewType(87654321u64)),
        ]);
        assert_all_equal([
            round_trip(&(1u32, 87654321u64)),
            round_trip(&Enum::NewType(87654321u64)),
        ]);
    }

    #[test]
    fn round_trip_tuple() {
        assert_all_equal([
            round_trip(&(87654321, 54321)),
            round_trip(&[87654321, 54321]),
        ]);

        assert_all_equal([
            round_trip(&(87654321i32, 54321u16)),
            round_trip(&Tuple(87654321i32, 54321u16)),
            round_trip(&Struct {
                a: 87654321i32,
                b: 54321u16,
            }),
        ]);

        assert_all_equal([
            round_trip(&(2u32, 87654321i32, 54321u16)),
            round_trip(&Enum::Struct {
                a: 87654321i32,
                b: 54321u16,
            }),
        ]);

        assert_all_equal([
            round_trip(&(3u32, 87654321i32, 54321u16)),
            round_trip(&Enum::Tuple(87654321i32, 54321u16)),
        ]);
    }

    #[test]
    fn round_trip_list() {
        assert_all_equal([
            round_trip(&(3usize, 87654321, 54321, 321)),
            round_trip(&Cow::Borrowed(&[87654321, 54321, 321][..])),
            round_trip(&vec![87654321, 54321, 321]),
        ]);
    }

    #[test]
    fn round_trip_string() {
        assert_all_equal([round_trip(b"\x04abcd"), round_trip(&"abcd".to_owned())]);
    }

    #[test]
    fn round_trip_map() {
        use indexmap::IndexMap;

        assert_all_equal([
            round_trip(&(
                3u32,
                ("a".to_owned(), 'A'),
                ("b".to_owned(), 'B'),
                ("c".to_owned(), 'C'),
            )),
            round_trip(&IndexMap::<String, char>::from_iter([
                ("a".to_owned(), 'A'),
                ("b".to_owned(), 'B'),
                ("c".to_owned(), 'C'),
            ])),
        ]);
    }

    #[test]
    fn error_eof() {
        let res = from_slice::<Vec<u8>>(&[5, 1, 2, 3, 4]).expect_err("too short");
        match res {
            de::Error::Io(e) => assert_eq!(
                e.kind(),
                std::io::ErrorKind::UnexpectedEof,
                "expected eof error"
            ),
            _ => panic!("incorrect error kind: {res:?}"),
        }
    }
}
