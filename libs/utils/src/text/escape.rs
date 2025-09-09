use std::cell::Cell;
use std::fmt::{self, Display};

/// Returns a type implementing [`Display`] that escapes by characters.
///
/// The `escape_as` closure should return [`None`] if the character isn't a
/// match. If it is a match, it should return [`Some`] with an [iterable] of
/// characters to replace it with; the original is not included by default so it
/// must be part of your output if it should be included.
///
/// # Examples
///
/// This example escapes `*` and `_` characters in a string by putting a `\` in
/// front of each one.
///
/// ```
/// # use utils::text::escape_by_char;
/// let source = "**hello world!** it is a _great_ day.";
/// let escaped = escape_by_char(source, |c| matches!(c, '*' | '_').then_some(['\\', c]));
///
/// assert_eq!(
///     escaped.to_string(),
///     r#"\*\*hello world!\*\* it is a \_great\_ day."#
/// );
/// ```
///
/// [iterable]: IntoIterator
pub fn escape_by_char<F, I>(source: &str, escape_as: F) -> EscapeByChar<'_, F>
where
    F: Fn(char) -> Option<I>,
    I: IntoIterator<Item = char>,
{
    EscapeByChar { source, escape_as }
}

/// Type returned by [`escape_by_char`].
#[derive(Debug, Clone, Copy)]
pub struct EscapeByChar<'a, F> {
    source: &'a str,
    escape_as: F,
}

impl<'a, F> EscapeByChar<'a, F> {
    /// Produces an equivalent value with `escape_as` used by-ref.
    ///
    /// This is only necessary when the original value is not [`Copy`] and is
    /// consumed for some reason.
    pub fn by_ref(&self) -> EscapeByChar<'a, &F> {
        EscapeByChar {
            source: self.source,
            escape_as: &self.escape_as,
        }
    }
}

impl<F, I> EscapeByChar<'_, F>
where
    F: Fn(char) -> Option<I>,
    I: IntoIterator<Item = char>,
{
    fn write_to(&self, mut f: impl fmt::Write) -> fmt::Result {
        let to_escape = Cell::new(None);
        let escape_as = |c: char| match (self.escape_as)(c) {
            i @ Some(_) => {
                to_escape.set(i);
                true
            },
            None => false,
        };

        for part in self.source.split(escape_as) {
            f.write_str(part)?;
            if let Some(iter) = to_escape.take() {
                for c in iter {
                    f.write_char(c)?;
                }
            }
        }

        Ok(())
    }
}

impl<F, I> Display for EscapeByChar<'_, F>
where
    F: Fn(char) -> Option<I>,
    I: IntoIterator<Item = char>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.write_to(f)
    }
}

impl<'a, F, I> FromIterator<EscapeByChar<'a, F>> for String
where
    F: Fn(char) -> Option<I>,
    I: IntoIterator<Item = char>,
{
    fn from_iter<T>(iter: T) -> Self
    where
        T: IntoIterator<Item = EscapeByChar<'a, F>>,
    {
        let mut s = Self::new();
        for part in iter {
            part.write_to(&mut s)
                .expect("writing to String cannot fail");
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPECTED: &str = r#"\*\*hello world!\*\* it is a \_great\_ day."#;

    fn escape_as(c: char) -> Option<[char; 2]> {
        matches!(c, '*' | '_').then_some(['\\', c])
    }

    #[test]
    fn to_string() {
        let source = "**hello world!** it is a _great_ day.";
        let escaped = escape_by_char(source, escape_as);

        let output = escaped.to_string();
        assert_eq!(output, EXPECTED);
    }

    #[test]
    fn from_iter_single() {
        let source = "**hello world!** it is a _great_ day.";
        let escaped = escape_by_char(source, escape_as);

        let from_iter = String::from_iter([escaped]);
        assert_eq!(from_iter, EXPECTED);
    }

    #[test]
    fn from_iter_many() {
        let source = ["**hello world", "!*", "* it is a _", "great_ day."];
        let escaped = source.map(|s| escape_by_char(s, escape_as));

        let from_iter = String::from_iter(escaped);
        assert_eq!(from_iter, EXPECTED);
    }
}
