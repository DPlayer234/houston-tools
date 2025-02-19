use serde::Serialize;

/// Represents a MongoDB filter condition for a field.
#[derive(Debug, Clone, PartialEq)]
pub enum Filter<T> {
    /// `$eq`
    ///
    /// Serialized as untagged.
    Eq(T),

    /// `$ne`
    Ne(T),
    /// `$gt`
    Gt(T),
    /// `$gte`
    Gte(T),
    /// `$lt`
    Lt(T),
    /// `$lte`
    Lte(T),
    /// `$in`
    In(Vec<T>),
    /// `$nin`
    NotIn(Vec<T>),
    /// `$exists`
    Exists(bool),
}

impl<T> Filter<T> {
    /// `$in`
    pub fn in_(values: impl IntoIterator<Item = T>) -> Self {
        Self::In(values.into_iter().collect())
    }

    /// `$nin`
    pub fn not_in(values: impl IntoIterator<Item = T>) -> Self {
        Self::NotIn(values.into_iter().collect())
    }

    /// Maps the filter to by-ref values.
    pub fn as_ref(&self) -> Filter<&T> {
        match self {
            Self::Ne(v) => Filter::Ne(v),
            Self::Gt(v) => Filter::Gt(v),
            Self::Gte(v) => Filter::Gte(v),
            Self::Lt(v) => Filter::Lt(v),
            Self::Lte(v) => Filter::Lte(v),
            Self::In(v) => Filter::In(v.iter().collect()),
            Self::NotIn(v) => Filter::NotIn(v.iter().collect()),
            Self::Exists(b) => Filter::Exists(*b),
            Self::Eq(v) => Filter::Eq(v),
        }
    }

    /// Maps the filter variants via a conversion function.
    pub fn map<U>(self, mut f: impl FnMut(T) -> U) -> Filter<U> {
        match self {
            Self::Ne(v) => Filter::Ne(f(v)),
            Self::Gt(v) => Filter::Gt(f(v)),
            Self::Gte(v) => Filter::Gte(f(v)),
            Self::Lt(v) => Filter::Lt(f(v)),
            Self::Lte(v) => Filter::Lte(f(v)),
            Self::In(v) => Filter::In(v.into_iter().map(f).collect()),
            Self::NotIn(v) => Filter::NotIn(v.into_iter().map(f).collect()),
            Self::Exists(b) => Filter::Exists(b),
            Self::Eq(v) => Filter::Eq(f(v)),
        }
    }
}

impl<T> From<T> for Filter<T> {
    fn from(value: T) -> Self {
        Self::Eq(value)
    }
}

impl<T: Serialize> Serialize for Filter<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        crate::private::serialize_filter_with(self, serializer, ())
    }
}
