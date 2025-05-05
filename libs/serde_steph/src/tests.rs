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
    ]);

    assert_all_equal([
        round_trip(&(3u32, 87654321i32, 54321u16)),
        round_trip(&Enum::Tuple(87654321i32, 54321u16)),
    ]);
}

#[test]
fn round_trip_struct() {
    assert_all_equal([
        round_trip(&(2usize, 87654321i32, 54321u16)),
        round_trip(&Struct {
            a: 87654321i32,
            b: 54321u16,
        }),
    ]);

    assert_all_equal([
        round_trip(&(2u32, 2usize, 87654321i32, 54321u16)),
        round_trip(&Enum::Struct {
            a: 87654321i32,
            b: 54321u16,
        }),
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
fn round_trip_addition() {
    #[derive(Debug, Serialize, Deserialize)]
    struct V1 {
        hello: String,
        code: u32,
    }

    #[derive(Debug, PartialEq, Serialize, Deserialize)]
    struct V2 {
        hello: String,
        code: u32,
        #[serde(default)]
        reason: Option<String>,
    }

    let buf_v1 = to_vec(&V1 {
        hello: "hello world".to_owned(),
        code: 974,
    })
    .expect("must serialize");

    let as_v2: V2 = from_slice(&buf_v1).expect("must deserialize");
    assert_eq!(
        as_v2,
        V2 {
            hello: "hello world".to_owned(),
            code: 974,
            reason: None,
        }
    );
}

#[test]
fn error_eof() {
    assert!(
        matches!(
            from_slice::<Vec<u8>>(&[5, 1, 2, 3, 4]),
            Err(Error::Io(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof
        ),
        "expected eof error"
    );
}

#[test]
fn de_borrowed() {
    let slice = *b"\x04abcd";
    let res: &str = from_slice(&slice).expect("must deserialize");

    assert_eq!(res, "abcd", "expected match");

    // is comparing addresses like this guaranteed to be stable?
    assert_eq!(
        res.as_ptr().addr(),
        slice[1..].as_ptr().addr(),
        "expected borrow from slice"
    );
}

#[test]
fn from_slice_excess() {
    let slice = *b"\x03abcd";
    let res = from_slice::<String>(&slice).expect_err("must be excess");

    assert!(
        matches!(res, Error::TrailingBytes),
        "must be trailing bytes error"
    );
}

#[test]
fn collect_str() {
    use serde::ser::Serializer as _;
    let mut ser = Serializer::from_writer(Vec::new());
    ser.collect_str(&format_args!("hello world {}", 16))
        .expect("must be able to write");

    assert_eq!(
        ser.as_writer().as_slice(),
        b"\x0Ehello world 16",
        "must have written string"
    );
}

#[test]
fn round_trip_io_read() {
    use std::io::Cursor;

    mod force_byte_buf {
        use std::fmt;

        use serde::{Deserializer, Serializer, de};

        pub fn serialize<S>(value: &[u8], serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            serializer.serialize_bytes(value)
        }

        pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
        where
            D: Deserializer<'de>,
        {
            struct ByteVisitor;

            impl de::Visitor<'_> for ByteVisitor {
                type Value = Vec<u8>;

                fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                    formatter.write_str("bytes")
                }

                fn visit_byte_buf<E>(self, v: Vec<u8>) -> Result<Self::Value, E>
                where
                    E: de::Error,
                {
                    Ok(v)
                }
            }

            deserializer.deserialize_byte_buf(ByteVisitor)
        }
    }

    #[derive(Debug, PartialEq, Deserialize, Serialize)]
    struct Test<'a> {
        int: u32,
        #[serde(borrow)]
        cow: Cow<'a, [u8]>,
        #[serde(with = "force_byte_buf")]
        vec: Vec<u8>,
    }

    let source = Test {
        int: 0x780,
        cow: Cow::Borrowed(b"abcde"),
        vec: b"ABCDEFG".to_vec(),
    };

    let buf = to_vec(&source).unwrap();
    let buf = buf.as_slice();

    let from_slice: Test<'_> = from_slice(buf).unwrap();
    let from_reader: Test<'_> = Deserializer::from_reader(Cursor::new(buf))
        .read_to_end()
        .unwrap();

    assert_eq!(from_slice, source);
    assert_eq!(from_reader, source);

    assert_eq!(from_slice.int, 0x780);
    assert!(matches!(from_slice.cow, Cow::Borrowed(b"abcde")));
    assert_eq!(from_slice.vec, b"ABCDEFG");

    assert_eq!(from_reader.int, 0x780);
    assert!(matches!(from_reader.cow, Cow::Owned(_)));
    assert_eq!(*from_reader.cow, *b"abcde");
    assert_eq!(from_reader.vec, b"ABCDEFG");
}

#[test]
fn de_faulty_seq_visitor() {
    use std::fmt;
    use std::result::Result;

    use serde::{Deserializer as _, de};

    let seq = Deserializer::from_slice(&[3, 1, 2, 3]).deserialize_seq(Faulty);
    let map = Deserializer::from_slice(&[3, 1, 2, 3, 4, 5, 6]).deserialize_map(Faulty);
    let tuple = Deserializer::from_slice(&[1, 2, 3]).deserialize_tuple(3, Faulty);

    assert!(
        matches!(seq, Err(Error::ShortSeqRead)),
        "seq length must be wrong"
    );
    assert!(
        matches!(map, Err(Error::ShortSeqRead)),
        "map length must be wrong"
    );
    assert!(
        matches!(tuple, Err(Error::ShortSeqRead)),
        "tuple length must be wrong"
    );

    /// Visitor that always reads 2x [`u32`].
    pub struct Faulty;

    impl<'de> de::Visitor<'de> for Faulty {
        type Value = ();

        fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
            formatter.write_str("sequence")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: de::SeqAccess<'de>,
        {
            seq.next_element::<u32>()?;
            seq.next_element::<u32>()?;
            Ok(())
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            map.next_entry::<u32, u32>()?;
            Ok(())
        }
    }
}

#[test]
fn ser_faulty_len() {
    use std::result::Result;

    use serde::ser::{SerializeMap as _, SerializeSeq as _, SerializeStruct as _};

    let mut buf = [0u8; 256];

    let seq_short = to_writer(buf.as_mut_slice(), &Seq(2, 3));
    let seq_long = to_writer(buf.as_mut_slice(), &Seq(3, 2));
    let map_short = to_writer(buf.as_mut_slice(), &Map(2, 3));
    let map_long = to_writer(buf.as_mut_slice(), &Map(3, 2));
    let struct_short = to_writer(buf.as_mut_slice(), &Struct(2, 3));
    let struct_long = to_writer(buf.as_mut_slice(), &Struct(3, 2));

    assert!(
        matches!(seq_short, Err(Error::LengthIncorrect)),
        "seq is too short"
    );
    assert!(
        matches!(seq_long, Err(Error::LengthIncorrect)),
        "seq is too long"
    );
    assert!(
        matches!(map_short, Err(Error::LengthIncorrect)),
        "map is too short"
    );
    assert!(
        matches!(map_long, Err(Error::LengthIncorrect)),
        "map is too long"
    );
    assert!(
        matches!(struct_short, Err(Error::LengthIncorrect)),
        "struct is too short"
    );
    assert!(
        matches!(struct_long, Err(Error::LengthIncorrect)),
        "struct is too long"
    );

    struct Seq(usize, usize);

    impl Serialize for Seq {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut seq = serializer.serialize_seq(Some(self.0))?;
            for _ in 0..self.1 {
                seq.serialize_element(&1usize)?;
            }
            seq.end()
        }
    }

    struct Map(usize, usize);

    impl Serialize for Map {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut map = serializer.serialize_map(Some(self.0))?;
            for _ in 0..self.1 {
                map.serialize_entry(&1usize, &1usize)?;
            }
            map.end()
        }
    }

    struct Struct(usize, usize);

    impl Serialize for Struct {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            let mut tuple = serializer.serialize_struct("Struct", self.0)?;
            for _ in 0..self.1 {
                tuple.serialize_field("hi", &1usize)?;
            }
            tuple.end()
        }
    }
}
