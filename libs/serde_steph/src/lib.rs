//! # Short Transport Encoding PBinary HFormat
//!
//! _Can you tell this isn't supposed to be an acronym?_
//!
//! Custom binary serialization format. This format is not self-describing and
//! as such deserializing any is disallowed.
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
//! When deserializing from a byte slice, deserializing borrowed data is
//! supported.
//!
//! This was initially inspired by the [`serde_bare`] crate, based on a bit of
//! the binary output (I did not read the spec), and written because its
//! implementation seemed sub-optimal and the crate appeared unmaintained. I
//! have since read the [BARE] spec also and come to the realization that the
//! only difference to this format is that BARE supports fixed-width ints while
//! this format supports ints up to 128 bits.
//!
//! I have also found out that [`postcard`] describes an identical format. As in
//! fully binary compatible, not that it defines types in the same way.
//!
//! The broad differences between the formats are really limited to how they are
//! specced (not that STEPH has any real spec beyond this comment).
//!
//! So I guess pick one that matches your needs ([`postcard`] supports
//! `!#[no_std]`). Or the one you think has the funniest name.
//!
//! [BARE]: <https://baremessages.org/>
//! [`serde_bare`]: <https://git.sr.ht/~tdeo/serde_bare>
//! [`postcard`]: <https://github.com/jamesmunns/postcard/tree/main>

pub mod de;
mod error;
mod leb128;
mod read;
pub mod ser;
#[cfg(test)]
mod tests;

pub use de::{from_reader, from_slice, Deserializer};
pub use error::{Error, Result};
pub use ser::{to_vec, to_writer, Serializer};
