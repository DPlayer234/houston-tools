use bson::Document;
use serde::Serialize;

/// Provides a builder for document updates.
///
/// `T` should be a partial type for the document you want to update and will be
/// serialized as-is for each field of this type.
///
/// This provides only a small amount of update operators that can be reasonably
/// provided. Updates into nested objects are not supported, unless the `T`
/// provided provides a flat representation of the nested object tree.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[non_exhaustive]
pub struct Update<T> {
    #[serde(rename = "$set")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set: Option<T>,
    #[serde(rename = "$setOnInsert")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub set_on_insert: Option<T>,
    #[serde(rename = "$inc")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inc: Option<T>,
    #[serde(rename = "$max")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<T>,
    #[serde(rename = "$min")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<T>,
}

impl<T> Update<T> {
    /// Creates a new empty update.
    #[must_use]
    pub fn new() -> Self {
        Self {
            set: None,
            set_on_insert: None,
            inc: None,
            max: None,
            min: None,
        }
    }
}

macro_rules! update_fn {
    ($(#[$attr:meta])* $T:ty, $field:ident) => {
        $(#[$attr])*
        #[must_use]
        pub fn $field(mut self, set: impl FnOnce($T) -> $T) -> Self {
            self.$field = Some(set(self.$field.take().unwrap_or_default()));
            self
        }
    };
}

impl<T: Default> Update<T> {
    update_fn!(
        /// Set the `$set` field of the update.
        T, set
    );
    update_fn!(
        /// Set the `$setOnInsert` field of the update.
        T, set_on_insert
    );
    update_fn!(
        /// Set the `$inc` field of the update.
        T, inc
    );
    update_fn!(
        /// Set the `$max` field of the update.
        T, max
    );
    update_fn!(
        /// Set the `$min` field of the update.
        T, min
    );
}

impl<T> Default for Update<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Serialize> Update<T> {
    /// Tries to serialize this value into a BSON document.
    pub fn into_document(self) -> bson::ser::Result<Document> {
        bson::to_document(&self)
    }
}

impl<T: Serialize> TryFrom<Update<T>> for Document {
    type Error = bson::ser::Error;

    fn try_from(value: Update<T>) -> Result<Self, Self::Error> {
        value.into_document()
    }
}
