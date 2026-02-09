use super::MutStrLike;
use crate::iter::ConstIter;

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

/// Given an ASCII or UTF-8 [`u8`] slice representing a `SNAKE_CASE` string,
/// converts it to title case (i.e. `Snake Case`). The slice is mutated
/// in-place.
pub const fn to_titlecase_u8(slice: &mut [u8]) {
    let mut is_start = true;

    let mut iter = ConstIter::new(slice);
    while let Some(item) = iter.next() {
        (*item, is_start) = titlecase_transform(*item, is_start);
    }
}

#[must_use]
pub const fn titlecase_transform(c: u8, is_start: bool) -> (u8, bool) {
    if c == b'_' {
        (b' ', true)
    } else if !is_start {
        (c.to_ascii_lowercase(), false)
    } else {
        (c.to_ascii_uppercase(), false)
    }
}

/// Transforms a const `&[u8]` in `SNAKE_CASE` format into titlecase version
/// (i.e. `Snake Case`). The resulting value is still const.
///
/// For [`&str`](str), use [`titlecase`](crate::titlecase) instead.
///
/// # Examples
///
/// ```
/// const TITLE: &[u8] = utils::titlecase_u8!(b"HELLO_NEW_WORLD");
/// assert_eq!(TITLE, b"Hello New World");
/// ```
///
/// Also works with lower snake case:
/// ```
/// const TITLE: &[u8] = utils::titlecase_u8!(b"hello_new_world");
/// assert_eq!(TITLE, b"Hello New World");
/// ```
#[macro_export]
macro_rules! titlecase_u8 {
    ($input:expr) => {
        // const-block to force compile-time eval and hide temporary named consts.
        // result is turned into a &[u8], with the size hidden again.
        &const {
            // Ensure input is a `&'static [u8]`
            const __INPUT: &[::std::primitive::u8] = $input;

            // Reusable const for byte length
            const __N: ::std::primitive::usize = __INPUT.len();

            // Include length in constant for next call.
            let mut value = *__INPUT
                .as_array::<__N>()
                .expect("must be same size as output");

            $crate::text::private::to_titlecase_u8(&mut value);
            value
        } as &[::std::primitive::u8]
    };
}

/// Transforms a const [`&str`](str) in `SNAKE_CASE` format into titlecase
/// version (i.e. `Snake Case`). The resulting value is still const.
///
/// For `&[u8]`, use [`titlecase_u8`](crate::titlecase_u8) instead.
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
#[macro_export]
macro_rules! titlecase {
    ($input:expr) => {
        // SAFETY: `titlecase!` does not affect UTF-8 validity and input was `&str`.
        unsafe {
            ::std::primitive::str::from_utf8_unchecked($crate::titlecase_u8!(
                // Ensure input is a `&'static str`
                ::std::primitive::str::as_bytes($input)
            ))
        }
    };
    (b: $input:expr) => {
        const {
            #[deprecated = "`titlecase!(b: ..)` should be replaced with `titlecase_u8!(..)`"]
            const __TITLECASE_B_DEPRECATED: &[u8] = $crate::titlecase_u8!($input);
            __TITLECASE_B_DEPRECATED
        }
    };
}
