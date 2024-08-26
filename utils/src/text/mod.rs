//! Provides helper methods to work with displayed text.

pub mod __private;
mod inline_str;

pub use inline_str::InlineStr;

/// Given a `SNAKE_CASE` string, converts it to title case (i.e. `Snake Case`).
///
/// # Examples
///
/// ```
/// let mut s = String::from("HELLO_NEW_WORLD");
/// utils::text::to_titlecase(&mut s);
/// assert_eq!(&s, "Hello New World");
/// ```
///
/// Or, with a byte string:
/// ```
/// let mut s = b"HELLO_NEW_WORLD".to_vec();
/// utils::text::to_titlecase(&mut s);
/// assert_eq!(&s, b"Hello New World");
/// ```
pub fn to_titlecase<S: MutStrLike + ?Sized>(value: &mut S) {
    // SAFETY: `to_titlecase_u8` only transforms
    // ASCII characters into other ASCII characters.
    unsafe {
        let slice = value.as_bytes_mut();
        to_titlecase_u8(slice);
    }
}

/// Given an ASCII or UTF-8 [`u8`] slice representing a `SNAKE_CASE` string, converts it to title case (i.e. `Snake Case`).
/// The slice is mutated in-place.
fn to_titlecase_u8(slice: &mut [u8]) {
    let mut is_start = true;

    for item in slice.iter_mut() {
        (*item, is_start) = titlecase_transform(*item, is_start);
    }
}

#[must_use]
const fn titlecase_transform(c: u8, is_start: bool) -> (u8, bool) {
    if c == b'_' {
        (b' ', true)
    } else if !is_start {
        (c.to_ascii_lowercase(), false)
    } else {
        (c.to_ascii_uppercase(), false)
    }
}

/// Transforms a const [`str`] in `SNAKE_CASE` format into titlecase version (i.e. `Snake Case`).
/// The resulting value is still const.
///
/// # Examples
///
/// ```
/// const TITLE: &str = utils::titlecase!("HELLO_NEW_WORLD");
/// assert_eq!(TITLE, "Hello New World");
/// ```
///
/// Also works with lower snake case:
/// ```
/// const TITLE: &str = utils::titlecase!("hello_new_world");
/// assert_eq!(TITLE, "Hello New World");
/// ```
///
/// Or byte strings, if prefixed with `b:`:
/// ```
/// const TITLE: &[u8] = utils::titlecase!(b: b"HELLO_NEW_WORLD");
/// assert_eq!(TITLE, b"Hello New World");
/// ```
#[macro_export]
macro_rules! titlecase {
    ($input:expr) => {{
        // Ensure input is a `&'static str`
        const INPUT_STR: &str = $input;

        // Transmute result back to a str.
        const BYTES: &[u8] = $crate::titlecase!(b: INPUT_STR.as_bytes());
        unsafe { ::std::str::from_utf8_unchecked(BYTES) }
    }};
    (b: $input:expr) => {{
        // Ensure input is a `&'static [u8]`
        const INPUT: &[u8] = $input;

        // Reusable const for byte length
        const N: usize = INPUT.len();

        // Include length in constant for next call.
        const CLONE: [u8; N] = *$crate::mem::with_size(INPUT);
        const RESULT: [u8; N] = $crate::text::__private::to_titlecase_u8_array(CLONE);
        &RESULT as &[u8]
    }};
}

/// Joins an arbitrary amount of const [`str`] values.
///
/// Unlike the [`std::concat`] macro, the parameters don't have to be literals, but also aren't stringified.
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
    ($str:expr) => {{
        const STR: &str = $str;
        STR
    }};
    ($($str:expr),*) => {{
        const STRS: &[&str] = &[$($str),*];
        const N: usize = $crate::text::__private::count_str_const(STRS);
        const JOIN: $crate::text::InlineStr<N> = $crate::text::__private::join_str_const(STRS);
        JOIN.as_str()
    }};
}

/// Ensures a string is at most `len` in size.
/// If it exceeds the size, it is truncated to the specified size, including appending ellipses at the end.
#[must_use]
pub fn truncate(str: impl Into<String>, len: usize) -> String {
    let str: String = str.into();
    if str.len() < len { return str; }

    str.chars().take(len - 1)
        .chain(std::iter::once('\u{2026}'))
        .collect()
}

/// Allows conversion of a type to a byte slice, indicating the bytes hold some sort of string data.
///
/// These byte slices do not have to hold UTF8 data, but replacing ASCII codes with other ASCII codes must not invalidate it.
///
/// This exists solely as support for [`to_titlecase`].
#[doc(hidden)]
pub unsafe trait MutStrLike {
    #[must_use]
    unsafe fn as_bytes_mut(&mut self) -> &mut [u8];
}

// Ideally there'd be blanket implementations for DerefMut<Target = str> and DerefMut<Target = [u8]>
// but that's not currently allowed.

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
