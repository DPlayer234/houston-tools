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

/// Transforms a const [`str`] in `SNAKE_CASE` format into titlecase version
/// (i.e. `Snake Case`). The resulting value is still const.
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
    ($input:expr) => {
        const {
            // Ensure input is a `&'static str`
            const __INPUT_STR: &::std::primitive::str = $input;

            // Transmute result back to a str.
    const __BYTES: &[::std::primitive::u8] = $crate::titlecase!(b: __INPUT_STR.as_bytes());

            // SAFETY: `titlecase!` does not affect UTF-8 validity and input was `&str`.
            unsafe { ::std::primitive::str::from_utf8_unchecked(__BYTES) }
        }
    };
    (b: $input:expr) => {
        const {
            // Ensure input is a `&'static [u8]`
            const __INPUT: &[::std::primitive::u8] = $input;

            // Reusable const for byte length
            const __N: ::std::primitive::usize = __INPUT.len();

            // Include length in constant for next call.
            const __RESULT: [::std::primitive::u8; __N] = {
                let mut value = *$crate::mem::as_sized(__INPUT);
                $crate::text::private::to_titlecase_u8(&mut value);
                value
            };
            &__RESULT as &[::std::primitive::u8]
        }
    };
}
