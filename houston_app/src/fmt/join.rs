use std::fmt::{Display, Formatter, Result, Write};

/// Allows joining formattable items.
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct Join<'a> {
    mid: &'a str,
    last: &'a str,
    once: &'a str,
}

impl Join<'static> {
    /// Join all elements with a comma: `,`
    pub const COMMA: Self = Self::simple(", ");

    /// Joins items with `and`, as if in natural speech.
    pub const AND: Self = Self::natural(", ", ", and ", " and ");

    /// Joins items with `or`, as if in natural speech.
    pub const OR: Self = Self::natural(", ", ", or ", " or ");
}

impl<'a> Join<'a> {
    /// Creates a joiner that joins every item with a constant string.
    pub const fn simple(join: &'a str) -> Self {
        Self::natural(join, join, join)
    }

    /// Creates a joiner that joins items like in natural speech.
    pub const fn natural(mid: &'a str, last: &'a str, once: &'a str) -> Self {
        Self { mid, last, once }
    }

    /// Writes joined items to a writer with a format function.
    pub fn write_with<W, T, F>(&self, writer: &mut W, items: &'a [T], mut fmt: F) -> Result
    where
        W: Write,
        F: FnMut(&'a T, &mut W) -> Result,
    {
        match items {
            [] => Ok(()),
            [single] => fmt(single, writer),
            [first, last] => {
                fmt(first, writer)?;
                writer.write_str(self.once)?;
                fmt(last, writer)
            },
            [first, mid @ .., last] => {
                fmt(first, writer)?;

                for mid in mid {
                    writer.write_str(self.mid)?;
                    fmt(mid, writer)?;
                }

                writer.write_str(self.last)?;
                fmt(last, writer)
            },
        }
    }

    /// Returns a [`Display`] that formats joined items with a format function.
    pub fn display_with<T, F>(self, items: &'a [T], fmt: F) -> JoinDisplayWith<'a, T, F>
    where
        F: Fn(&'a T, &mut Formatter<'_>) -> Result,
    {
        JoinDisplayWith {
            joiner: self,
            items,
            fmt,
        }
    }

    /// Returns [`Display`] value that formats joined items via a mapping
    /// function.
    pub fn display_as<T, F, D>(self, items: &'a [T], map: F) -> JoinDisplayAs<'a, T, F>
    where
        F: Fn(&'a T) -> D,
        D: 'a + Display,
    {
        JoinDisplayAs {
            joiner: self,
            items,
            map,
        }
    }
}

/// Display item for [`Join::display_with`].
pub struct JoinDisplayWith<'a, T, F> {
    joiner: Join<'a>,
    items: &'a [T],
    fmt: F,
}

/// Display item for [`Join::display_as`].
pub struct JoinDisplayAs<'a, T, F> {
    joiner: Join<'a>,
    items: &'a [T],
    map: F,
}

impl<'a, T, F> Display for JoinDisplayWith<'a, T, F>
where
    F: Fn(&'a T, &mut Formatter<'_>) -> Result,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.joiner.write_with(f, self.items, &self.fmt)
    }
}

impl<'a, T, F, D> Display for JoinDisplayAs<'a, T, F>
where
    F: Fn(&'a T) -> D,
    D: 'a + Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        self.joiner
            .write_with(f, self.items, |i, f| (self.map)(i).fmt(f))
    }
}
