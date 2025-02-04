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
//! When deserializing from a byte slice, deserializing borrowed data is
//! supported.
//!
//! [bare]: <https://baremessages.org/>
//! [^bare]: No, I did not really read the spec and the output likely isn't compatible.

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
