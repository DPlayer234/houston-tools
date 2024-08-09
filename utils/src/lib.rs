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

/// Defines a simple, public error type with an error message.
///
/// The resulting type will, by default, only implement [`Error`](std::error::Error), [`Debug`](std::fmt::Debug), and [`Display`](std::fmt::Display).
/// Additional derives may be added as needed.
///
/// # Examples
///
/// The simplest type is declared with just a type name and error message:
///
/// ```no_run
/// utils::define_simple_error!(MyError: "error happened");
/// println!("{:?}", MyError);
/// ```
///
/// You can further add fields and include their values in the error message:
///
/// ```no_run
/// utils::define_simple_error!(MyError(String): s => "error '{}' happened", s.0);
/// println!("{:?}", MyError("user fault".into()));
/// ```
///
/// Named fields are also supported:
///
/// ```no_run
/// utils::define_simple_error!(MyError { code: u8 }: s => "error '{}' happened", s.code);
/// println!("{:?}", MyError { code: 42 });
/// ```
///
/// You may also use attributes on the error type:
///
/// ```no_run
/// utils::define_simple_error!(
///     #[derive(Clone)]
///     MyError(u8):
///     s => "error '{}' happened", s.0
/// );
/// ```
#[macro_export]
macro_rules! define_simple_error {
    ($(#[$attr:meta])* $type:ident : $($message:tt)*) => {
        $(#[$attr])*
        #[derive(Debug)]
        pub struct $type;
        $crate::define_simple_error!(@main $type: $($message)*);
    };
    ($(#[$attr:meta])* $type:ident { $($body:tt)* } : $($message:tt)*) => {
        $(#[$attr])*
        #[derive(Debug)]
        pub struct $type { $($body)* }
        $crate::define_simple_error!(@main $type: $($message)*);
    };
    ($(#[$attr:meta])* $type:ident ( $($body:tt)* ) : $($message:tt)*) => {
        $(#[$attr])*
        #[derive(Debug)]
        pub struct $type ( $($body)* );
        $crate::define_simple_error!(@main $type: $($message)*);
    };
    (@main $type:ident: $message:expr) => {
        impl ::std::error::Error for $type {}
        impl ::std::fmt::Display for $type {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                write!(f, $message)
            }
        }
    };
    (@main $type:ident: $s:ident => $($message:tt)*) => {
        impl ::std::error::Error for $type {}
        impl ::std::fmt::Display for $type {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                let $s = self;
                write!(f, $($message)*)
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
