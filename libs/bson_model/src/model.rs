use std::{fmt, hint};

use bson::Bson;

use crate::update::Update;

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
/// #[model(derive_partial(Debug, Clone, PartialEq, Eq, Hash))]
/// #[model(derive_filter(Debug, Clone, PartialEq))]
/// # struct MyModel { _p: () }
/// # _ = stringify! {
/// struct MyModel {
///     ...
/// }
/// # };
/// ```
///
/// An empty `#[model(derive_partial())]` or `#[model(derive_filter())]` is
/// allowed to suppress the [`Debug`] and [`Clone`] auto-derives without adding
/// additional ones.
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

    /// The field accessors type.
    type Fields;

    /// Create an empty partial model.
    #[must_use]
    fn partial() -> Self::Partial;

    /// Create a new filter builder.
    #[must_use]
    fn filter() -> Self::Filter;

    /// Create a new sort builder.
    #[must_use]
    fn sort() -> Self::Sort;

    /// Gets the fields.
    #[must_use]
    fn fields() -> Self::Fields;

    /// Create a new update builder.
    #[must_use]
    fn update() -> Update<Self::Partial> {
        Update::new()
    }
}

/// Represents a model's field name.
///
/// When used in a [`bson`](bson::bson!) or [`doc`](bson::doc!) in place of a
/// field name, resolves to the field name. When used in a value position,
/// instead resolves to the expression value (i.e. a string prefixed with `$`).
///
/// This can be useful to manually build up more complex queries, like aggregate
/// `$group` or `$project` clauses.
///
/// Obtain instances of this type via [`ModelDocument::fields()`].
///
/// # Examples
///
/// ```
/// use bson::oid::ObjectId;
/// use bson_model::{Filter, ModelDocument};
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Deserialize, Serialize, ModelDocument)]
/// struct Pizza {
///     #[serde(rename = "_id")]
///     id: ObjectId,
///     name: String,
///     size: String,
///     quantity: i32,
/// }
///
/// #[derive(Debug, Clone, Deserialize, ModelDocument)]
/// struct Totals {
///     #[serde(rename = "_id")]
///     name: ObjectId,
///     total_quantity: i32,
/// }
///
/// // count the amount of each type of medium-size pizzas
/// let steps = [
///     bson::doc! {
///         "$match": Pizza::filter()
///             .size("medium".to_owned())
///             .into_document()?,
///     },
///     bson::doc! {
///         "$group": {
///             // a $group stage must have an _id field -- the Totals::name here
///             Totals::fields().name(): Pizza::fields().name(),
///             Totals::fields().total_quantity(): {
///                 "$sum": Pizza::fields().quantity(),
///             }
///         }
///     },
/// ];
/// # Ok::<_, bson::ser::Error>(())
/// ```
#[derive(Clone, Copy)]
#[must_use]
pub struct ModelField {
    // safety invariant: begins with `$`
    expr: &'static str,
    _unsafe: (),
}

impl fmt::Display for ModelField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.expr, f)
    }
}

impl fmt::Debug for ModelField {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.expr, f)
    }
}

impl ModelField {
    /// Creates a new value from the expression name.
    ///
    /// # Panics
    ///
    /// Panics if `expr` does not begin with `$`. No further validation is done.
    pub const fn new(expr: &'static str) -> Self {
        assert!(
            matches!(expr.as_bytes(), [b'$', ..]),
            "expr must start with $"
        );
        Self { expr, _unsafe: () }
    }

    /// Gets the name of this field.
    pub const fn name(self) -> &'static str {
        // essentially `self.expr.get_unchecked(1..)` but const
        let [_, bytes @ ..] = self.expr.as_bytes() else {
            // SAFETY: string always begins with '$', so it can't be empty
            unsafe { hint::unreachable_unchecked() };
        };

        // SAFETY: string always begins with '$', so the tail after is valid UTF-8
        unsafe { std::str::from_utf8_unchecked(bytes) }
    }

    /// Gets the expression of this field. That is, the name of the field
    /// prefixed with `$`.
    pub const fn expr(self) -> &'static str {
        self.expr
    }
}

impl From<ModelField> for Bson {
    fn from(value: ModelField) -> Self {
        Self::String(value.expr().to_owned())
    }
}

impl From<ModelField> for String {
    fn from(value: ModelField) -> Self {
        value.name().to_owned()
    }
}
