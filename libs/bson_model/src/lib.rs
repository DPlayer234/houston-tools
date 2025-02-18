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

/// Derivable trait for document model structs.
pub trait ModelDocument {
    /// The type of the partial model.
    type Partial;

    /// The type of the filter builder.
    type Filter;

    /// The type of the sort builder
    type Sort;

    /// Create an empty partial model.
    #[must_use]
    fn partial() -> Self::Partial;

    /// Create a new filter builder.
    #[must_use]
    fn filter() -> Self::Filter;

    /// Create a new sort builder.
    #[must_use]
    fn sort() -> Self::Sort;

    /// Create a new update builder.
    #[must_use]
    fn update() -> Update<Self::Partial> {
        Update::new()
    }
}
