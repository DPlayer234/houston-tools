use std::fmt::Debug;

pub mod fields;
pub mod mem;
pub mod prefix_map;
pub mod range;
pub mod str_as_data;
pub mod text;
pub mod time;

/// Convenience method to calculate the hash of a value with the [`std::hash::DefaultHasher`].
#[must_use]
#[inline]
pub fn hash_default<T: std::hash::Hash>(value: &T) -> u64 {
    hash(value, std::hash::DefaultHasher::new())
}

/// Convenience method to feed a value to a hasher and then return its value.
#[must_use]
#[inline]
pub fn hash<T: std::hash::Hash, H: std::hash::Hasher>(value: &T, mut hasher: H) -> u64 {
    value.hash(&mut hasher);
    hasher.finish()
}

/// Trait that allows discarding values.
pub trait Discard {
    /// Consumes and discards the value.
    /// May panic if debug assertions are enabled.
    fn discard(self);
}

impl<T, E: Debug> Discard for Result<T, E> {
    /// Consumes and discards the value.
    /// If debug assertions are enabled, panics if it holds an error.
    fn discard(self) {
        if cfg!(debug_assertions) {
            drop(self.unwrap());
        }
    }
}

#[macro_export]
macro_rules! define_simple_error {
    ($type:ident : $message:literal) => {
        #[derive(Debug, Clone)]
        #[must_use]
        pub struct $type;

        impl ::std::error::Error for $type {}

        impl ::std::fmt::Display for $type {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, $message)
            }
        }
    };
}

/// Joins multiple path segments into a [`PathBuf`](`std::path::PathBuf`).
///
/// An extension may be specified at the end. If specified, it will override the extension of the last segment.
///
/// This is equivalent to creating a [`PathBuf`](`std::path::PathBuf`) from the first segment and then repeatedly
/// calling `push`, then finishing with `set_extension` if an extension is specified.
///
/// # Example
///
/// ```
/// # use std::path::Path;
/// let path = utils::join_path!["C:\\", "Windows", "System32", "notepad"; "exe"];
/// # #[cfg(windows)]
/// assert_eq!(
///     &path,
///     Path::new(r#"C:\Windows\System32\notepad.exe"#)
/// )
/// ```
#[macro_export]
macro_rules! join_path {
    [$root:expr, $($parts:expr),* $(; $ext:expr)?] => {{
        let mut path = ::std::path::PathBuf::from($root);
        $(
            path.push($parts);
        )*
        $(
            path.set_extension($ext);
        )?
        path
    }};
}

/// Helper trait to define an async function within a trait or implementation,
/// allowing specifying additional bounds for the returned future by adding them to the `async` keyword.
///
/// It expands to an equivalent function without the async keyword, instead returning a [`Future`](`core::future::Future`).
///
/// # Example
///
/// ```no_run
/// # use utils::async_trait_fn;
/// # struct Data;
/// # struct Store;
///
/// // Specify that futures must be `Send`.
/// trait Async {
///     async_trait_fn! {
///         async + Send fn get(id: u64) -> Data;
///     }
///
///     async_trait_fn! {
///         async + Send fn store(id: u64, data: Data);
///     }
/// }
///
/// impl Async for Store {
///     // You may use raw `async fn` to implement these methods, but this may leak additional
///     // auto-traits to direct consumers of this implementation, which may be a semver hazard.
///     async fn get(id: u64) -> Data {
///         # let _ = stringify! {
///         ...
///         # };
///         # Data
///     }
///
///     // Alternatively, use the macro in the implementation too.
///     async_trait_fn! {
///         async + Send fn store(id: u64, data: Data) {
///             # let _ = stringify! {
///             ...
///             # };
///         }
///     }
/// }
/// ```
#[macro_export]
macro_rules! async_trait_fn {
    {
        $(#[$attr:meta])*
        $v:vis async $(+ $bounds:tt)* fn $name:ident ( $($args:tt)* ) $(-> $ret:ty)? ;
    } => {
        $(#[$attr])*
        $v fn $name ( $($args)* ) -> impl ::core::future::Future $(<Output = $ret>)? $(+ $bounds)* ;
    };
    {
        $(#[$attr:meta])*
        $v:vis async $(+ $bounds:tt)* fn $name:ident ( $($args:tt)* ) $(-> $ret:ty)? $body:block
    } => {
        $(#[$attr])*
        $v fn $name ( $($args)* ) -> impl ::core::future::Future $(<Output = $ret>)? $(+ $bounds)* {
            async $body
        }
    };
}
