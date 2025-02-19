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

/// Derivable trait for BSON model document structs.
///
/// The derive macro emits a struct for each associated type on
/// [`ModelDocument`].
///
/// A basic model might look like this:
///
/// ```
/// use bson::oid::ObjectId;
/// use bson_model::{Filter, ModelDocument};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Deserialize, Serialize, ModelDocument)]
/// struct User {
///     #[serde(rename = "_id")]
///     id: ObjectId,
///     name: String,
///     email: String,
///     age: i32,
/// }
///
/// let filter = User::filter()
///     .age(Filter::Gte(18))
///     .into_document()?;
///
/// let update = User::update()
///     .set(|u| u.age(21))
///     .into_document()?;
/// # Ok::<_, bson::ser::Error>(())
/// ```
///
/// The emitted types implement [`Default`] and [`serde::Serialize`] no matter
/// what. If nothing is explicitly specified, [`Debug`] and [`Clone`] are also
/// derived, however you may specify a custom set of traits to derive for them:
///
/// ```
/// # use bson_model::{Filter, ModelDocument};
/// # use serde::{Deserialize, Serialize};
/// #[derive(Debug, Clone, PartialEq, Hash, Deserialize, Serialize, ModelDocument)]
/// #[model(derive(Debug, Clone, PartialEq, Eq, Hash))]
/// # struct MyModel {}
/// # _ = stringify! {
/// struct MyModel {
///     ...
/// }
/// # };
/// ```
///
/// An empty `#[model(derive())]` is allowed to suppress the [`Debug`] and
/// [`Clone`] auto-derives without adding additional ones.
///
/// [`serde::Deserialize`] is not supported at the current time and attempting
/// to use a derived implementation for it on the builder types will lead to
/// inconsistent deserialization.
///
/// Currently the [`ModelDocument::Sort`] type has fixed derived traits due to
/// its implementation.
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
