//! Provides a typed interface to create filters, updates, and sorts for MongoDB
//! BSON documents via a derive macro.

mod filter;
#[doc(hidden)]
pub mod private;
mod sort;
mod update;

pub use ::bson_model_macros::ModelDocument;
pub use filter::Filter;
pub use sort::Sort;
pub use update::Update;
