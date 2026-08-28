//! This module documents the wire format for STEPH.
//!
//! # Types
//!
//! The basic types are as follows:
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
//! In particular, a "sequence" simply means the bytes of the individual values
//! are concatenated without padding.
//!
//! The following syntax may be used for complex types:
//!
//! - `list[T]`: homogenous `list` where each value is `T`
//! - `list[?]`: `list` with unknown types
//! - `tuple[A..]`: homogenous `tuple` where all values are `A`
//! - `tuple[A, B]`: heteregenous `tuple` with values `A` and `B`
//! - `tuple[*]`: heteregenous `tuple` with unknown types
//! - `map[K, V]`: homogenous `map` where all keys are `K` and all values are
//!   `V`
//! - `map[?, ?]`: `map` with unknown types
//! - `enum(T)`: `enum` variant with variant data `T`
//!
//! The following types are equivalent:
//!
//! - `map[K, V]` = `list[tuple[K, V]]`
//! - `enum(T)` = `tuple[uint, T]`
//! - `struct` = `list[*]`
//! - `unit` = `tuple[]`
//!
//! # Serde Mapping
//!
//! | `serde` type            | STEPH type       | Notes |
//! | ----------------------- | ---------------- | ----- |
//! | `bool`, `u8`, `i8`      | `byte`           | Converted to `u8`. |
//! | `uN`                    | `uint`           | - |
//! | `iN`                    | `sint`           | - |
//! | `fN`                    | `tuple[byte..]`  | Encoded as raw little-endian bytes. |
//! | `char`                  | `uint`           | Converted to `u32`. |
//! | `bytes`                 | `list[byte]`     | - |
//! | `str`                   | `list[byte]`     | Encoded as UTF-8 bytes. |
//! | `none`                  | `enum(unit)`     | Variant index `0`. |
//! | `some<T>`               | `enum(T)`        | Variant index `1`. |
//! | `unit`, `unit_struct`   | `unit`           | - |
//! | `newtype_struct<T>`     | `T`              | Transparent as `T`. |
//! | `newtype_variant<T>`    | `enum(T)`        | - |
//! | `seq`                   | `list[*]`        | Types determined by [`Serialize`] & [`Deserialize`]. |
//! | `tuple`, `tuple_struct` | `tuple[*]`       | Types determined by [`Serialize`] & [`Deserialize`]. |
//! | `tuple_variant`         | `enum(tuple[*])` | Types determined by [`Serialize`] & [`Deserialize`]. |
//! | `map`                   | `map[?, ?]`      | Types determined by [`Serialize`] & [`Deserialize`]. |
//! | `struct`                | `struct`         | - |
//! | `struct_variant`        | `enum(struct)`   | - |
//!
//! # Additional Notes for the Types
//!
//! ## `uint`/`sint`
//!
//! These are encoded in the variable-length LEB128 encoding.
//!
//! In simple terms: Integers are encoded as one base-128 digit per byte, with
//! the highest bit set each if there is another byte to read. Signed integers
//! are encoded the same way via `f(x) = abs(x) << 1 | sign(x)`.
//!
//! See also: <https://en.wikipedia.org/wiki/LEB128>
//!
//! ## `list`
//!
//! The length prefix must match the real encoded amount of values.
//!
//! A `list` is expected to be _homogenous_, but this isn't restricted. It may
//! be clearer to encode a heterogenous `list` as a `struct`.
//!
//! ## `struct`
//!
//! The length prefix must match the real encoded amount of fields.
//!
//! When encoding `struct` fields, the names are ignored and _exclusively_ the
//! order is considered. [`Serialize`] and [`Deserialize`] must agree on the
//! field order.
//!
//! ## `tuple`
//!
//! Functionally, a `tuple` is a `struct` with no length prefix. Instead,
//! [`Serialize`] and [`Deserialize`] must agree on not just the field order,
//! but also the amount of fields.
//!
//! ## `enum`
//!
//! The variant prefix must match the data. [`Serialize`] and [`Deserialize`]
//! must agree on what data each variant prefix indicates.
//!
//! [`Serialize`]: serde_core::Serialize
//! [`Deserialize`]: serde_core::Deserialize
