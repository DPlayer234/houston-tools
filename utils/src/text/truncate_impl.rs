use std::borrow::{Borrow, Cow};

/// Truncates a string to the given `len` (in terms of [`char`], not [`u8`]).
/// If a truncation happens, appends an ellipsis.
///
/// This function is supported for [`str`], [`String`], and [`Cow<str>`].
/// `&mut str` is not directly supported. Pass it as `&str` instead.
///
/// Given a _value_, it reuses its buffer and returns the modified value with the same type.
///
/// Given an _immutable reference_, it will return a [`Cow<str>`], either referencing the original or storing a modified copy.
///
/// Give a _mutable reference_, it will modify the value in place.
///
/// # Panics
///
/// Panics if `len` is zero. `len` must be at least 1.
///
/// # Examples
///
/// By value:
/// ```
/// # use std::borrow::Cow;
/// # use utils::text::truncate;
/// let text = String::from("hello world");
/// let long = truncate(text, 11);
/// assert!(long == "hello world");
/// let short = truncate(long, 6);
/// assert!(short == "hello…");
/// ```
///
/// By immutable reference:
/// ```
/// # use std::borrow::Cow;
/// # use utils::text::truncate;
/// let text = "hello world";
/// let long = truncate(text, 11);
/// let short = truncate(text, 6);
/// assert!(matches!(long, Cow::Borrowed(text)));
/// assert!(short == "hello…");
/// ```
///
/// By mutable reference:
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

#[inline]
fn find_truncate_at(s: &str, len: usize) -> Option<usize> {
    assert!(len >= 1, "cannot truncate to less than 1 character");

    if s.len() <= len { return None; }

    let mut indices = s.char_indices();
    let (end_at, _) = indices.nth(len - 1)?;
    indices.next().and(Some(end_at))
}

/// Not public API.
#[doc(hidden)]
pub trait Truncate {
    type Output;

    fn truncate(this: Self, len: usize) -> Self::Output;
}

impl<'a> Truncate for Cow<'a, str> {
    type Output = Self;

    fn truncate(mut this: Self, len: usize) -> Self::Output {
        Truncate::truncate(&mut this, len);
        this
    }
}

impl<'a, 'b> Truncate for &'b Cow<'a, str> {
    type Output = Cow<'b, str>;

    fn truncate(this: Self, len: usize) -> Self::Output {
        <&str as Truncate>::truncate(this.borrow(), len)
    }
}

impl<'a, 'b> Truncate for &'b mut Cow<'a, str> {
    type Output = ();

    fn truncate(this: Self, len: usize) -> Self::Output {
        if let Some(end_at) = find_truncate_at(this, len) {
            let str = this.to_mut();
            str.truncate(end_at);
            str.push('\u{2026}');
        }
    }
}

impl<'a> Truncate for &'a str {
    type Output = Cow<'a, str>;

    fn truncate(this: Self, len: usize) -> Self::Output {
        Truncate::truncate(Cow::Borrowed(this), len)
    }
}

impl Truncate for String {
    type Output = Self;

    fn truncate(mut this: Self, len: usize) -> Self::Output {
        Truncate::truncate(&mut this, len);
        this
    }
}

impl<'a> Truncate for &'a String {
    type Output = Cow<'a, str>;

    fn truncate(this: Self, len: usize) -> Self::Output {
        Truncate::truncate(Cow::Borrowed(this.as_str()), len)
    }
}

impl<'a> Truncate for &'a mut String {
    type Output = ();

    fn truncate(this: Self, len: usize) -> Self::Output {
        if let Some(end_at) = find_truncate_at(this, len) {
            this.truncate(end_at);
            this.push('\u{2026}');
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
