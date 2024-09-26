use std::fmt::{Display, Write};

/// Returns a type implementing [`Display`] that escapes a specified
/// the characters listed in [`pat`] through [`escape_as`].
pub fn escape_by_char<P, F, I>(source: &str, pat: P, escape_as: F) -> EscapeByChar<P, F>
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
#[derive(Debug, Clone)]
pub struct EscapeByChar<'a, P, F> {
    source: &'a str,
    pat: P,
    escape_as: F,
}

impl<'a, P, F, I> Display for EscapeByChar<'a, P, F>
where
    P: Fn(char) -> bool,
    F: Fn(char) -> I,
    I: IntoIterator<Item = char>,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
}
