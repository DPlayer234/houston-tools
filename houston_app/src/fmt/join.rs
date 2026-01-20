use std::fmt::{Display, Formatter, Result};

/// Allows joining formattable items.
#[derive(Debug, Clone, Copy)]
#[must_use]
pub struct Join<'a> {
    mid: &'a str,
    last: &'a str,
    once: &'a str,
}

impl<'a> Join<'a> {
    /// Join all elements with nothing between.
    pub const EMPTY: Self = Self::simple("");

    /// Join all elements with a comma and space: `,`
    pub const COMMA: Self = Self::simple(", ");

    /// Join all elements with a slash: `/`
    pub const SLASH: Self = Self::simple("/");

    /// Joins items with `and`, as if in natural speech.
    pub const AND: Self = Self::natural(", ", ", and ", " and ");

    /// Joins items with `or`, as if in natural speech.
    pub const OR: Self = Self::natural(", ", ", or ", " or ");

    /// Creates a joiner that joins every item with a constant string.
    pub const fn simple(join: &'a str) -> Self {
        Self::natural(join, join, join)
    }

    /// Creates a joiner that joins items like in natural speech.
    pub const fn natural(mid: &'a str, last: &'a str, once: &'a str) -> Self {
        Self { mid, last, once }
    }

    /// Returns [`Display`] value that formats joined items via a mapping
    /// function.
    pub fn display_as<T, F, D>(&'a self, items: &'a [T], map: F) -> JoinDisplayAs<'a, T, F>
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

/// Display item for [`Join::display_as`].
pub struct JoinDisplayAs<'a, T, F> {
    joiner: &'a Join<'a>,
    items: &'a [T],
    map: F,
}

impl<'a, T, F, D> Display for JoinDisplayAs<'a, T, F>
where
    F: Fn(&'a T) -> D,
    D: 'a + Display,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        let map = &self.map;
        let joiner = *self.joiner;

        match self.items {
            [] => Ok(()),
            [single] => map(single).fmt(f),
            [first, last] => {
                map(first).fmt(f)?;
                f.write_str(joiner.once)?;
                map(last).fmt(f)
            },
            [first, mid @ .., last] => {
                map(first).fmt(f)?;

                for mid in mid {
                    f.write_str(joiner.mid)?;
                    map(mid).fmt(f)?;
                }

                f.write_str(joiner.last)?;
                map(last).fmt(f)
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty() {
        let items: &[i32] = &[];
        assert_eq!(Join::AND.display_as(items, |i| *i).to_string(), "");
    }

    #[test]
    fn single() {
        let items = &[42];
        assert_eq!(Join::AND.display_as(items, |i| *i).to_string(), "42");
    }

    #[test]
    fn two() {
        let items = &[42, 69];
        assert_eq!(Join::AND.display_as(items, |i| *i).to_string(), "42 and 69");
    }

    #[test]
    fn three() {
        let items = &[42, 69, 420];
        assert_eq!(
            Join::AND.display_as(items, |i| *i).to_string(),
            "42, 69, and 420"
        );
    }

    #[test]
    fn more() {
        let items = &[1, 2, 3, 4, 5, 6, 7];
        assert_eq!(
            Join::AND.display_as(items, |i| *i).to_string(),
            "1, 2, 3, 4, 5, 6, and 7"
        );
    }
}
