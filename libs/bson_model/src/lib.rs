//! Provides an experimental typed interface to create filters, updates, and
//! sorts for MongoDB BSON documents via a derive macro.
//!
//! Alternatively also provides a way to get the _actual_ field names, including
//! in expression form, in a way that allows reasonably easy ways to build up
//! queries with [`bson::doc!`] if the typed system is unsufficient.
//!
//! See the documentation on the [`ModelDocument`] trait, which can be derived.

pub use bson_model_macros::ModelDocument;

mod filter;
mod model;
mod multi;
#[doc(hidden)]
pub mod private;
mod sort;
mod update;

pub use filter::Filter;
pub use model::{ModelDocument, ModelField};
pub use multi::Multi;
pub use sort::Sort;
pub use update::Update;
