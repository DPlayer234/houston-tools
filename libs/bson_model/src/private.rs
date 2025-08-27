//! Internal details for use by the proc-macro expansion.

use std::convert::Infallible;
use std::marker::PhantomData;

pub use {bson, serde, serde_with};

/// Marker type, used to define an empty type with generic arguments.
pub struct Never<T: ?Sized>(Infallible, PhantomData<T>);
