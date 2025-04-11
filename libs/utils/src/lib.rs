//! A variety of utility types and functions for use across the crates in this
//! repo.

// for benchmarks
#[cfg(test)]
use criterion as _;

pub mod fuzzy;
pub mod iter;
pub mod mem;
pub mod range;
pub mod str_as_data;
pub mod term;
pub mod text;

mod macros;
mod private;

#[expect(deprecated, reason = "to be removed in a future version")]
pub use private::hash::{hash, hash_default};
