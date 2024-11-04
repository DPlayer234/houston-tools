use std::fmt::{self, Display, Write as _};
use std::str::Chars;

/// Returns a type implementing [`Display`] that escapes a specified
/// the characters listed in `pat` through `escape_as`.
///
/// Can also be converted to an [`Iterator`] over [`char`].
pub fn escape_by_char<P, F, I>(source: &str, pat: P, escape_as: F) -> EscapeByChar<'_, P, F>
where
    P: Fn(char) -> bool,
    F: Fn(char) -> I,
    I: IntoIterator<Item = char>,
{
    EscapeByChar {
        source,
        pat,
        escape_as,
    }
}

/// Type returned by [`escape_by_char`].
#[derive(Debug, Clone, Copy)]
pub struct EscapeByChar<'a, P, F> {
    source: &'a str,
    pat: P,
    escape_as: F,
}

impl<'a, P, F> EscapeByChar<'a, P, F> {
    /// Produces an equivalent value with the `pat` and `escape_as` used by-ref.
    ///
    /// This is only necessary when the original value is not [`Copy`] and
    /// is consumed for some reason, f.e. by calling [`EscapeByChar::into_iter`].
    pub fn by_ref(&self) -> EscapeByChar<'a, &P, &F> {
        EscapeByChar {
            source: self.source,
            pat: &self.pat,
            escape_as: &self.escape_as,
        }
    }
}

impl<P, F, I> Display for EscapeByChar<'_, P, F>
where
    P: Fn(char) -> bool,
    F: Fn(char) -> I,
    I: IntoIterator<Item = char>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for part in self.source.split_inclusive(&self.pat) {
            let mut part_iter = part.chars();
            match part_iter.next_back() {
                Some(sep) if (self.pat)(sep) => {
                    f.write_str(part_iter.as_str())?;
                    for c in (self.escape_as)(sep) {
                        f.write_char(c)?;
                    }
                }
                _ => f.write_str(part)?,
            }
        }

        Ok(())
    }
}

impl<'a, P, F, I> IntoIterator for EscapeByChar<'a, P, F>
where
    P: Fn(char) -> bool,
    F: Fn(char) -> I,
    I: IntoIterator<Item = char>,
{
    type Item = char;
    type IntoIter = EscapeByCharIter<'a, P, F, I::IntoIter>;

    fn into_iter(self) -> Self::IntoIter {
        EscapeByCharIter {
            source: self.source.chars(),
            pat: self.pat,
            escape_as: self.escape_as,
            iter: None,
        }
    }
}

/// [`Iterator`] over the [`char`] values produced by [`escape_by_char`].
///
/// If you just want to [`collect`](Iterator::collect) it into a [`String`],
/// use [`to_string`](ToString) on [`EscapeByChar`] instead.
#[derive(Debug, Clone)]
#[must_use = "iterators are lazy and do nothing if not used"]
pub struct EscapeByCharIter<'a, P, F, I> {
    source: Chars<'a>,
    pat: P,
    escape_as: F,
    iter: Option<I>,
}

impl<P, F, I> EscapeByCharIter<'_, P, F, I::IntoIter>
where
    P: Fn(char) -> bool,
    F: Fn(char) -> I,
    I: IntoIterator<Item = char>,
{
    fn try_escape_as(&mut self, c: char) -> Option<I> {
        ((self.pat)(c)).then(move || (self.escape_as)(c))
    }
}

impl<P, F, I> Iterator for EscapeByCharIter<'_, P, F, I::IntoIter>
where
    P: Fn(char) -> bool,
    F: Fn(char) -> I,
    I: IntoIterator<Item = char>,
{
    type Item = char;

    fn next(&mut self) -> Option<char> {
        loop {
            if let Some(iter) = &mut self.iter {
                match iter.next() {
                    Some(c) => break Some(c),
                    None => self.iter = None,
                }
            }

            let next = self.source.next();
            if let Some(iter) = next.and_then(|c| self.try_escape_as(c)) {
                self.iter = Some(iter.into_iter());
                continue;
            }

            break next;
        }
    }
}

#[cfg(test)]
mod test {
    use std::hint::black_box;

    use super::escape_by_char;

    #[test]
    fn escape_str() {
        let source = black_box("**hello world!** it is a _great_ day.");
        let escaped = escape_by_char(source, |c| matches!(c, '*' | '_'), |c| ['\\', c]);

        assert_eq!(escaped.to_string(), r#"\*\*hello world!\*\* it is a \_great\_ day."#);
    }

    #[test]
    fn escape_str_iter() {
        let source = black_box("**hello world!** it is a _great_ day.");
        let escaped = escape_by_char(source, |c| matches!(c, '*' | '_'), |c| ['\\', c]).into_iter();

        assert_eq!(escaped.collect::<String>(), r#"\*\*hello world!\*\* it is a \_great\_ day."#);
    }
}
