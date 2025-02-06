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
//! - `enum`: enums, no matter their data; for [`Option`], variant 0 is [`None`]
//!   and variant 1 is [`Some`]
//! - `map`: map-like sequences, f.e. [`HashMap`](std::collections::HashMap)
//!
//! When deserializing from a byte slice, deserializing borrowed data is
//! supported.
//!
//! The following things should be considered when attributing your types:
//!
//! - Field, struct, and variant names are omitted.
//! - `#[serde(default)]` and `#[serde(flatten)]` are effectively ignored.
//! - The `#[serde(skip_*)]` attributes will break deserialization.
//!   `#[serde(skip)]` is fine because it's symmetrical, but adding/removing it
//!   will change the format.
//! - `#[serde(untagged)]` is unsupported.
//!
//! ## Future Proofing
//!
//! Because this format isn't self-describing and doesn't store anything such as
//! field names, extending data types in a way that still allows deserializing
//! old data is tricky.
//!
//! Here are the following considerations:
//!
//! - Changing a tuple struct into a normal struct or vice-versa is compatible
//!   as long as the declared field order stays the same
//! - Adding ZST fields is compatible (as long as they serialize as units).
//! - `#[serde(flatten)]` has no effect in practice.
//! - Inlining structs or factoring out fields into structs is compatible as
//!   long as the overall declared field order stays the same in the final
//!   struct.
//! - Swapping out structs/enums with ones that have equivalent representations
//!   is compatible.
//! - Adding enum variants at the end is always compatible.
//! - Changing to bigger integer types is allowed, as long as the source type
//!   isn't [`u8`] or [`i8`] and the sign does not change (i.e. [`u16`] to
//!   [`i32`] isn't allowed).
//! - Changing a type to a new-type wrapper or vice versa is always allowed.
//!
//! While adding new enum variants is trivial, adding non-ZST fields to structs
//! will break deserialization.
//!
//! Some reasonable options, each with their own downsides, remain:
//!
//! 1. Include a placeholder field that is serialized to a fixed value (f.e.
//!    [`Option<Infallible>`]). This is later replaced by an [`Option`] with a
//!    struct that holds the new data and a new placeholder. This is simple, but
//!    will lead to excessive nesting when many changes are done and leads to
//!    very impractical field access. (F.e. `self.v2.v3.v4.v5.field`).
//!
//! 2. Serialize the struct as an enum whose variant describes the version and
//!    convert old versions on deserialization. on the other hand is a lot more
//!    complex (introducing new wire-only types on every version with conversion
//!    code), but is also a lot more flexible and doesn't lead to the same
//!    awkward tree structures. It also allows removal of fields or general
//!    restructuring as needed.
//!
//! 3. Give up and reject old data. This is very easy but also does not allow
//!    interoperability with older versions. In this case, including a
//!    [`VersionTag`](compat::VersionTag) for verification may make sense.
//!
//! ## Attribution
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

pub mod compat;
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
