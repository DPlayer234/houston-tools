//! Provides helper methods to work with displayed text.

mod escape;
mod from_fn_impl;
mod inline_str;
pub mod private;
mod titlecase_impl;
mod truncate_impl;
mod write_str;

pub use escape::{EscapeByChar, escape_by_char};
pub use from_fn_impl::{FromFn, from_fn};
pub use inline_str::InlineStr;
pub use titlecase_impl::to_titlecase;
pub use truncate_impl::truncate;
pub use write_str::WriteStr;

/// Joins an arbitrary amount of const [`str`] values.
///
/// Unlike the [`std::concat`] macro, the parameters don't have to be literals,
/// but also aren't stringified.
///
/// # Examples
///
/// ```
/// const BASE: &str = "https://example.com/";
/// const PATH: &str = "cool_page.html";
/// const FRAGMENT: &str = "#best_part";
/// const QUERY: &str = "?bad_stuff=false";
/// const URL: &str = utils::join!(BASE, PATH, FRAGMENT, QUERY);
/// assert_eq!(URL, "https://example.com/cool_page.html#best_part?bad_stuff=false");
/// ```
#[macro_export]
macro_rules! join {
    ($str:expr) => { const {
        const __STR: &::std::primitive::str = $str;
        __STR
    }};
    ($($str:expr),*) => { const {
        const __STRS: &[&::std::primitive::str] = &[$($str),*];
        const __N: ::std::primitive::usize = $crate::text::private::count_str_const(__STRS);
        const __JOIN: $crate::text::InlineStr<__N> = $crate::text::private::join_str_const(__STRS);
        __JOIN.as_str()
    }};
}

/// Allows conversion of a type to a byte slice, indicating the bytes hold some
/// sort of string data.
///
/// These byte slices do not have to hold UTF8 data, but replacing ASCII codes
/// with other ASCII codes must not invalidate it.
///
/// This exists solely as support for [`to_titlecase`].
#[doc(hidden)]
pub unsafe trait MutStrLike {
    #[must_use]
    unsafe fn as_bytes_mut(&mut self) -> &mut [u8];
}

// Ideally there'd be blanket implementations for DerefMut<Target = str> and
// DerefMut<Target = [u8]> but that's not currently allowed.

unsafe impl MutStrLike for String {
    unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { self.as_mut_str().as_bytes_mut() }
    }
}

unsafe impl MutStrLike for str {
    unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { self.as_bytes_mut() }
    }
}

unsafe impl MutStrLike for [u8] {
    unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        self
    }
}

unsafe impl MutStrLike for Vec<u8> {
    unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        self.as_mut_slice()
    }
}

unsafe impl<const N: usize> MutStrLike for InlineStr<N> {
    unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
        unsafe { self.as_bytes_mut() }
    }
}
