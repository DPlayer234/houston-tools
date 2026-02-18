//! A variety of utility types and functions for use across the crates in this
//! repo.
#![warn(missing_docs)]

// for benchmarks
#[cfg(test)]
use criterion as _;

pub mod fuzzy;
pub mod iter;
mod macros;
pub mod mem;
mod private;
pub mod range;
pub mod str_as_data;
pub mod term;
pub mod text;
