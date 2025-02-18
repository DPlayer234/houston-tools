use bson::Bson;
use serde::Serialize;

/// The sort order for a BSON field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Sort {
    /// Ascending sort order: `1`
    Asc,
    /// Descending sort order: `-1`
    Desc,
}

impl Serialize for Sort {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self {
            Self::Asc => serializer.serialize_i32(1),
            Self::Desc => serializer.serialize_i32(-1),
        }
    }
}

impl From<Sort> for Bson {
    fn from(sort: Sort) -> Self {
        match sort {
            Sort::Asc => Self::Int32(1),
            Sort::Desc => Self::Int32(-1),
        }
    }
}
