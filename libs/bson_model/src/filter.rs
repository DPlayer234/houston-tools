use serde::Serialize;

/// Represents a MongoDB filter condition for a field.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Filter<T> {
    /// `$ne`
    #[serde(rename = "$ne")]
    Ne(T),
    /// `$gt`
    #[serde(rename = "$gt")]
    Gt(T),
    /// `$gte`
    #[serde(rename = "$gte")]
    Gte(T),
    /// `$lt`
    #[serde(rename = "$lt")]
    Lt(T),
    /// `$lte`
    #[serde(rename = "$lte")]
    Lte(T),
    /// `$in`
    #[serde(rename = "$in")]
    In(Vec<T>),
    /// `$nin`
    #[serde(rename = "$nin")]
    NotIn(Vec<T>),
    /// `$exists`
    #[serde(rename = "$exists")]
    Exists(bool),

    /// `$eq`
    ///
    /// Serialized as untagged.
    #[serde(untagged)]
    Eq(T),
}

impl<T> Filter<T> {
    /// `$in`
    pub fn in_(values: impl IntoIterator<Item = T>) -> Self {
        Self::In(Vec::from_iter(values))
    }

    /// `$nin`
    pub fn not_in(values: impl IntoIterator<Item = T>) -> Self {
        Self::NotIn(Vec::from_iter(values))
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
