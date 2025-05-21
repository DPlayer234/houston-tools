//! # Short Transport Encoding Pbinary Hformat
//!
//! _Can you tell this isn't supposed to be an acronym?_
//!
//! Custom binary serialization format. This format is not self-describing and
//! as such deserializing any is disallowed. However, it encodes the amount of
//! struct fields and as such additions may be backwards-compatible.
//!
//! The types in this serialization format are as follows:
//!
//! - `byte`: single output byte
//! - `uint`: unsigned LEB128 integer
//! - `sint`: signed LEB128 integer
//! - `list`: sequence of values with `uint` length prefix
//! - `struct`: sequence of fields with `uint` field count prefix
//! - `tuple`: sequence of values without length prefix but known length
//! - `enum`: variant index as `uint` followed by variant data
//! - `map`: `list` of key-value 2-tuples
//! - `unit`: empty `tuple`
//!
//! Rust types map to these as follows:
//!
//! - `byte`: [`u8`], [`i8`], and [`bool`] (as 0 or 1)
//! - `uint`: [`u16`], [`u32`], [`u64`], [`u128`], [`usize`], [`char`]
//! - `sint`: [`i16`], [`i32`], [`i64`], [`i128`], [`isize`]
//! - `list`: variable length sequences, f.e. slices, [`str`], and [`Vec`]
//! - `struct`: regular structs, even when empty
//! - `tuple`: arrays, tuples, tuple structs, and new-type structs
//! - `enum`: enums, no matter their data; for [`Option`], variant 0 is [`None`]
//!   and variant 1 is [`Some`]
//! - `map`: map-like sequences, f.e. [`HashMap`](std::collections::HashMap)
//! - `unit`: `()`, empty arrays, and unit structs
//!
//! When deserializing from a byte slice, deserializing borrowed slices is
//! supported.
//!
//! The following things should be considered when attributing your types:
//!
//! - Field, struct, and variant names are omitted.
//! - Attribute ZST marker fields with `#[serde(skip)]` so they are ignored in
//!   the format.
//! - `#[serde(default)]` on non-trailing fields is useless.
//! - The `#[serde(skip_*)]` attributes will break deserialization.
//!   `#[serde(skip)]` is fine because it's symmetrical, but adding/removing it
//!   will change the format.
//! - `#[serde(untagged)]` is unsupported because it omits data needed for
//!   deserialization later.
//! - `#[serde(flatten)]` is unsupported because it means the container does not
//!   provide all data needed during serialization.
//!
//! Additionally, the following should be considered for manual [`Serialize`]
//! and [`Deserialize`] implementations:
//!
//! - [`Serialize`] implementations must provide _accurate_ lengths to
//!   [`Serializer`]. Omitting them is not allowed either.
//! - [`Deserialize`] implementations must not call
//!   `Deserializer::deserialize_any` or
//!   `Deserializer::deserialize_ignored_any`.
//! - [`Deserialize`] implementations must read structs, tuples, sequences, and
//!   maps to completion.
//!
//! [`Serialize`]: serde::Serialize
//! [`Deserialize`]: serde::Deserialize
//!
//! ## Future Proofing
//!
//! Because this format isn't self-describing and doesn't store anything such as
//! field names, extending data types in a way that still allows deserializing
//! old data is tricky.
//!
//! However since it encodes the amount of fields in a struct, some
//! backwards-compatible changes can still be made.
//!
//! The following changes are format-compatible:
//!
//! - Adding fields to the end of a `struct`, if they are `#[serde(default)]`.
//!   Note that you must attribute [`Option`] fields too.
//! - Inlining a `tuple` into another `tuple`.
//! - Swapping out types with the same representation.
//! - Adding new enum variants to the end.
//! - Extending integer types (i.e. [`u16`] to [`u32`] is OK), unless the
//!   original type is `byte`.
//! - Replacing a type to a new-type wrapper around it or vice versa is
//!   compatible.
//!
//! The following changes are format-incompatible:
//!
//! - Adding fields in a `struct` anywhere other than the end or without
//!   `#[serde(default)]`.
//! - Adding non-`unit` fields to `tuple`.
//! - Changing the sign of an integer (i.e [`u16`] to [`i16`] is not OK). This
//!   would not break deserialization but will change the value, even when the
//!   old value would be in range for the new type.
//! - Adding or removing `#[serde(skip)]`.
//!
//! Do note that even the format-compatible changes aren't necessarily
//! _forward_-compatible. This is important to consider because it means that
//! old applications will not be able to handle new data, even if the inverse
//! works.
//!
//! ## Attribution
//!
//! This was initially inspired by the [`serde_bare`] crate, based on a bit of
//! the binary output (I did not read the [BARE] spec), and written because its
//! implementation seemed sub-optimal and the crate appeared unmaintained.
//!
//! I have also found out that [`postcard`] describes a similar format. As in
//! almost binary compatible to [BARE], not that it defines types in the same
//! way.
//!
//! STEPHs major difference to these formats is the serialization of structs:
//! The field count prefix allows it to retain backwards-compatability at the
//! cost of adding minor binary overhead. This is useful when you want to
//! allow your software to keep handling old versions of the serialized data.
//!
//! In a related note, I will also mention [CBOR] and the [`ciborium`] crate if
//! your concerns require something closer to a binary JSON.
//!
//! In the end, pick one that matches your needs ([`postcard`] supports
//! `#![no_std]`). Or the one you think has the funniest name.
//!
//! [BARE]: <https://baremessages.org/>
//! [CBOR]: <https://cbor.io/>
//! [`serde_bare`]: <https://git.sr.ht/~tdeo/serde_bare>
//! [`postcard`]: <https://github.com/jamesmunns/postcard/tree/main>
//! [`ciborium`]: <https://github.com/enarx/ciborium>

pub mod compat;
pub mod de;
mod error;
mod leb128;
mod read;
pub mod ser;
#[cfg(test)]
mod tests;

pub use de::{Deserializer, from_reader, from_slice};
pub use error::{Error, Result};
pub use ser::{Serializer, to_vec, to_writer};
