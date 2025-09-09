use std::borrow::Cow;

use crate::private::str::Indices;

const ELLIPSIS: char = '\u{2026}';

/// Truncates a string to the given `len` (in terms of [`char`], not [`u8`]).
/// If a truncation happens, appends an ellipsis.
///
/// The following types are accepted for `str`, with slightly varying behavior:
///
/// | Input                                           | Return                                                                                  |
/// |:----------------------------------------------- |:--------------------------------------------------------------------------------------- |
/// | `&T` where `T`: [`AsRef<str>`]                  | [`Cow<str>`], either borrowing the source [`str`] or owning a truncated clone.          |
/// | [`String`], [`Cow<str>`]                        | The input type, either the input value or truncated, attempting to use the same buffer. |
/// | [`&mut String`](String), [`&mut Cow<str>`](Cow) | No return value. The value is truncated in-place, attempting to use the same buffer.    |
///
/// For the `&T` case: Do note that various types implement [`AsRef<str>`],
/// notably including [`str`] itself, [`String`], and [`Cow<str>`].
/// If you require an owned [`String`] after the truncation, call
/// [`into_owned`](Cow::into_owned) on the return value.
///
/// If you're working with an owned [`String`] or [`Cow<str>`], prefer passing
/// it by value or `&mut` to avoid redundant clones.
///
/// # Panics
///
/// Panics if `len` is zero. `len` must be at least 1.
///
/// # Examples
///
/// Type annotations in the examples are optional and provided only for clarity.
///
/// `&T` where `T`: [`AsRef<str>`], i.e. by immutable reference, here with
/// [`&str`](str):
///
/// ```
/// // by-ref may clone the value if needed and returns a `Cow<str>`
/// # use std::borrow::Cow;
/// # use utils::text::truncate;
/// let text = "hello world";
/// let long: Cow<'_, str> = truncate(text, 11);
/// let short: Cow<'_, str> = truncate(text, 6);
/// assert!(matches!(long, Cow::Borrowed(text)));
/// assert!(short == "hello…");
/// ```
///
/// By value, here with [`String`]:
///
/// ```
/// # use utils::text::truncate;
/// let text = String::from("hello world");
/// let long: String = truncate(text, 11);
/// assert!(long == "hello world");
/// let short: String = truncate(long, 6);
/// assert!(short == "hello…");
/// ```
///
/// By mutable reference, here with [`&mut String`](String):
///
/// ```
/// # use utils::text::truncate;
/// let mut text = String::from("hello world");
/// truncate(&mut text, 11);
/// assert!(text == "hello world");
/// truncate(&mut text, 6);
/// assert!(text == "hello…");
/// ```
pub fn truncate<T: Truncate>(str: T, len: usize) -> T::Output {
    T::truncate(str, len)
}

// Note: allowing `&mut &mut str` may be useful, but experiments show that it
// ends up being very hard to call due to limitations between the borrow checker
// and trait implementations on `&mut T`.

/// If the input is longer than `len` _characters_, returns [`Some`] with a
/// _byte index_ to truncate the input at such that, if another character is
/// appended after the returned index point, that the total string is `len`
/// characters long.
///
/// In effect, this returns the byte index of the character at position `len -
/// 1` if it has more than `len` characters.
///
/// Returns [`None`] if the string is at most `len` characters long and doesn't
/// need to be truncated.
///
/// # Panics
///
/// Panics if `len` is less than 1.
#[inline]
fn find_truncate_at(s: &str, len: usize) -> Option<usize> {
    assert!(len >= 1, "cannot truncate to less than 1 character");

    if s.len() <= len {
        return None;
    }

    let mut indices = Indices::new(s);
    let end_at = indices.nth(len - 1)?;
    indices.next()?;
    Some(end_at)
}

/// Exists to support the [`truncate`] function.
///
/// Not public API.
#[doc(hidden)]
pub trait Truncate {
    type Output;

    fn truncate(this: Self, len: usize) -> Self::Output;
}

impl<'a, S: AsRef<str> + ?Sized> Truncate for &'a S {
    type Output = Cow<'a, str>;

    fn truncate(this: Self, len: usize) -> Self::Output {
        // non-generic shared code
        fn inner(this: &str, len: usize) -> Cow<'_, str> {
            if let Some(end_at) = find_truncate_at(this, len) {
                Cow::Owned(to_truncated_at(this, end_at))
            } else {
                Cow::Borrowed(this)
            }
        }

        inner(this.as_ref(), len)
    }
}

macro_rules! delegate_to_by_mut {
    ($($Ty:ty),*) => { $(
        impl Truncate for $Ty {
            type Output = Self;

            fn truncate(mut this: Self, len: usize) -> Self::Output {
                <&mut Self as Truncate>::truncate(&mut this, len);
                this
            }
        }
    )* };
}

delegate_to_by_mut!(Cow<'_, str>, String);

impl Truncate for &mut Cow<'_, str> {
    type Output = ();

    fn truncate(this: Self, len: usize) -> Self::Output {
        if let Some(end_at) = find_truncate_at(this, len) {
            match this {
                Cow::Borrowed(src) => *this = Cow::Owned(to_truncated_at(src, end_at)),
                Cow::Owned(buf) => truncate_at(buf, end_at),
            }
        }
    }
}

impl Truncate for &mut String {
    type Output = ();

    fn truncate(this: Self, len: usize) -> Self::Output {
        if let Some(end_at) = find_truncate_at(this, len) {
            truncate_at(this, end_at);
        }
    }
}

/// Creates a [`String`] with the content of `src` truncated at `end_at`.
///
/// The capacity of the result should be the minimum needed.
fn to_truncated_at(src: &str, end_at: usize) -> String {
    debug_assert!(src.len() > end_at, "must actually truncate");
    debug_assert!(src.is_char_boundary(end_at), "must truncate on boundary");

    let new_len = end_at + ELLIPSIS.len_utf8();
    let mut buf = String::with_capacity(new_len);
    buf.push_str(&src[..end_at]);
    buf.push(ELLIPSIS);
    buf
}

/// Truncates a [`String`] at `end_at`.
///
/// When truncating near the end, avoids overallocating the buffer.
fn truncate_at(buf: &mut String, end_at: usize) {
    debug_assert!(buf.len() > end_at, "must actually truncate");
    debug_assert!(buf.is_char_boundary(end_at), "must truncate on boundary");

    buf.truncate(end_at);
    buf.reserve_exact(ELLIPSIS.len_utf8());
    buf.push(ELLIPSIS);
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use super::{Truncate, truncate};

    #[expect(clippy::ptr_arg)]
    fn is_borrowed(t: &Cow<'_, str>) -> bool {
        matches!(t, Cow::Borrowed(_))
    }

    #[expect(clippy::ptr_arg)]
    fn is_owned(t: &Cow<'_, str>) -> bool {
        matches!(t, Cow::Owned(_))
    }

    #[expect(clippy::ptr_arg)]
    fn is_owned_exact_cap(t: &Cow<'_, str>) -> bool {
        matches!(t, Cow::Owned(x) if x.len() == x.capacity())
    }

    #[test]
    fn truncate_string() {
        let to_single = truncate("hello".to_owned(), 1);
        let to_one_down = truncate("hello".to_owned(), 4);
        let to_exact = truncate("hello".to_owned(), 5);
        let too_much = truncate("hello".to_owned(), 10);

        assert!(
            to_single == "…" && to_single.chars().count() == 1,
            "\"{to_single}\" == \"…\""
        );
        assert!(
            to_one_down == "hel…" && to_one_down.chars().count() == 4,
            "\"{to_one_down}\" == \"hel…\""
        );
        assert!(
            to_exact == "hello" && to_exact.chars().count() == 5,
            "\"{to_exact}\" == \"hello\""
        );
        assert!(
            too_much == "hello" && too_much.chars().count() == 5,
            "\"{too_much}\" == \"hello\""
        );
    }

    #[test]
    fn truncate_ref() {
        truncate_into_cow(&|| "hello", &is_owned_exact_cap, &is_borrowed);
    }

    #[test]
    fn truncate_cow_borrowed() {
        truncate_into_cow(
            &|| Cow::Borrowed("hello"),
            &is_owned_exact_cap,
            &is_borrowed,
        );
    }

    #[test]
    fn truncate_cow_owned() {
        truncate_into_cow(
            &|| Cow::Owned("hello".to_owned()),
            &is_owned,
            &is_owned_exact_cap,
        );
    }

    fn truncate_into_cow<'a, T>(
        text: &dyn Fn() -> T,
        match_trunc: &dyn Fn(&Cow<'a, str>) -> bool,
        match_no_trunc: &dyn Fn(&Cow<'a, str>) -> bool,
    ) where
        T: 'a + Truncate<Output = Cow<'a, str>>,
    {
        let to_single = truncate(text(), 1);
        let to_one_down = truncate(text(), 4);
        let to_exact = truncate(text(), 5);
        let too_much = truncate(text(), 10);

        assert!(match_trunc(&to_single) && to_single == "…" && to_single.chars().count() == 1);
        assert!(
            // if the initial capacity was 5, this actually needs to reserve more capacity
            match_trunc(&to_one_down) && to_one_down == "hel…" && to_one_down.chars().count() == 4
        );
        assert!(match_no_trunc(&to_exact) && to_exact == "hello" && to_exact.chars().count() == 5);
        assert!(match_no_trunc(&too_much) && too_much == "hello" && too_much.chars().count() == 5);
    }

    #[test]
    fn truncate_multi_byte() {
        let text = "ヴァンプライ";
        assert!(text.len() == 18 && text.chars().count() == 6);

        let to_single = truncate(text, 1);
        let to_one_down = truncate(text, 5);
        let to_exact = truncate(text, 6);
        let too_much = truncate(text, 7);

        assert!(
            matches!(to_single, Cow::Owned(_))
                && to_single == "…"
                && to_single.chars().count() == 1
        );
        assert!(
            matches!(to_one_down, Cow::Owned(_))
                && to_one_down == "ヴァンプ…"
                && to_one_down.chars().count() == 5
        );
        assert!(
            matches!(to_exact, Cow::Borrowed(_))
                && to_exact == "ヴァンプライ"
                && to_exact.chars().count() == 6
        );
        assert!(
            matches!(too_much, Cow::Borrowed(_))
                && too_much == "ヴァンプライ"
                && too_much.chars().count() == 6
        );
    }
}
