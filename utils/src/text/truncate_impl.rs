use std::borrow::Cow;

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
/// For the `&T` case: Do note that various types implement [`AsRef<str>`], notably including [`str`] itself, [`String`], and [`Cow<str>`].
/// If you require an owned [`String`] after the truncation, call [`into_owned`](Cow::into_owned) on the return value.
///
/// If you're working with an owned [`String`] or [`Cow<str>`], prefer passing it by value or `&mut` to avoid redundant clones.
///
/// # Panics
///
/// Panics if `len` is zero. `len` must be at least 1.
///
/// # Examples
///
/// Type annotations in the examples are optional and provided only for clarity.
///
/// `&T` where `T`: [`AsRef<str>`], i.e. by immutable reference, here with [`&str`](str):
/// ```
/// // by-ref may clone the value if needed and returns a `Cow<str>`
/// # use std::borrow::Cow;
/// # use utils::text::truncate;
/// let text = "hello world";
/// let long: Cow<str> = truncate(text, 11);
/// let short: Cow<str> = truncate(text, 6);
/// assert!(matches!(long, Cow::Borrowed(text)));
/// assert!(short == "hello…");
/// ```
///
/// By value, here with [`String`]:
/// ```
/// # use std::borrow::Cow;
/// # use utils::text::truncate;
/// let text = String::from("hello world");
/// let long: String = truncate(text, 11);
/// assert!(long == "hello world");
/// let short: String = truncate(long, 6);
/// assert!(short == "hello…");
/// ```
///
/// By mutable reference, here with [`&mut String`](String):
/// ```
/// # use std::borrow::Cow;
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

// Note: allowing `&mut &mut str` seems tempting, but the ellipsis character takes 3 bytes in UTF-8.
// Furthermore, actually mutating a `&mut str` not only requires unsafe code, it would need to be
// unsafe to call because the caller has to make sure the source reference isn't used anymore as
// the full buffer may no longer even be valid UTF-8.
// Plus, I don't have a reason for it currently.

#[inline]
fn find_truncate_at(s: &str, len: usize) -> Option<usize> {
    assert!(len >= 1, "cannot truncate to less than 1 character");

    if s.len() <= len { return None; }

    let mut indices = s.char_indices();
    let (end_at, _) = indices.nth(len - 1)?;
    indices.next().and(Some(end_at))
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
        let this: &str = this.as_ref();
        Truncate::truncate(Cow::Borrowed(this), len)
    }
}

impl Truncate for Cow<'_, str> {
    type Output = Self;

    fn truncate(mut this: Self, len: usize) -> Self::Output {
        Truncate::truncate(&mut this, len);
        this
    }
}

impl Truncate for String {
    type Output = Self;

    fn truncate(mut this: Self, len: usize) -> Self::Output {
        Truncate::truncate(&mut this, len);
        this
    }
}

impl Truncate for &mut Cow<'_, str> {
    type Output = ();

    fn truncate(this: Self, len: usize) -> Self::Output {
        if let Some(end_at) = find_truncate_at(this, len) {
            let str = this.to_mut();
            str.truncate(end_at);
            str.push(ELLIPSIS);
        }
    }
}

impl Truncate for &mut String {
    type Output = ();

    fn truncate(this: Self, len: usize) -> Self::Output {
        if let Some(end_at) = find_truncate_at(this, len) {
            this.truncate(end_at);
            this.push(ELLIPSIS);
        }
    }
}

#[cfg(test)]
mod test {
    use std::borrow::Cow;

    use super::truncate;

    #[test]
    fn truncate_string() {
        let mut to_single = "hello".to_owned();
        let mut to_one_down = "hello".to_owned();
        let mut to_exact = "hello".to_owned();
        let mut too_much = "hello".to_owned();

        truncate(&mut to_single, 1);
        truncate(&mut to_one_down, 4);
        truncate(&mut to_exact, 5);
        truncate(&mut too_much, 10);

        assert!(to_single == "…" && to_single.chars().count() == 1);
        assert!(to_one_down == "hel…" && to_one_down.chars().count() == 4);
        assert!(to_exact == "hello" && to_exact.chars().count() == 5);
        assert!(too_much == "hello" && too_much.chars().count() == 5);
    }

    #[test]
    fn truncate_ref() {
        let text = "hello";

        let to_single = truncate(text, 1);
        let to_one_down = truncate(text, 4);
        let to_exact = truncate(text, 5);
        let too_much = truncate(text, 10);

        assert!(matches!(to_single, Cow::Owned(_)) && to_single == "…" && to_single.chars().count() == 1);
        assert!(matches!(to_one_down, Cow::Owned(_)) && to_one_down == "hel…" && to_one_down.chars().count() == 4);
        assert!(matches!(to_exact, Cow::Borrowed(_)) && to_exact == "hello" && to_exact.chars().count() == 5);
        assert!(matches!(too_much, Cow::Borrowed(_)) && too_much == "hello" && too_much.chars().count() == 5);
    }

    #[test]
    fn truncate_multi_byte() {
        let text = "ヴァンプライ";
        assert!(text.len() == 18 && text.chars().count() == 6);

        let to_single = truncate(text, 1);
        let to_one_down = truncate(text, 5);
        let to_exact = truncate(text, 6);
        let too_much = truncate(text, 7);

        assert!(matches!(to_single, Cow::Owned(_)) && to_single == "…" && to_single.chars().count() == 1);
        assert!(matches!(to_one_down, Cow::Owned(_)) && to_one_down == "ヴァンプ…" && to_one_down.chars().count() == 5);
        assert!(matches!(to_exact, Cow::Borrowed(_)) && to_exact == "ヴァンプライ" && to_exact.chars().count() == 6);
        assert!(matches!(too_much, Cow::Borrowed(_)) && too_much == "ヴァンプライ" && too_much.chars().count() == 6);
    }
}
