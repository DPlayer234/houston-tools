use std::fmt::{Write, Display};

use smallvec::SmallVec;

pub mod azur;
pub mod discord;

pub fn write_join<W, I>(mut f: W, mut iter: I, join: &str) -> std::fmt::Result
where
    W: Write,
    I: Iterator,
    I::Item: Display,
{
    if let Some(item) = iter.next() {
        write!(f, "{item}")?;
        for item in iter {
            f.write_str(join)?;
            write!(f, "{item}")?;
        }
    }

    Ok(())
}

#[must_use]
pub struct JoinNatural<'a> {
    data: SmallVec<[&'a str; 15]>,
    join: &'a JoinBlock<'a>,
}

struct JoinBlock<'a> {
    mid: &'a str,
    last: &'a str,
    once: &'a str
}

impl<'a> JoinNatural<'a> {
    #[inline]
    fn new(iter: impl IntoIterator<Item = &'a str>, join: &'a JoinBlock<'a>) -> Self {
        Self {
            data: iter.into_iter().collect(),
            join,
        }
    }

    #[inline]
    pub fn and(iter: impl IntoIterator<Item = &'a str>) -> Self {
        static JOIN: JoinBlock<'static> = JoinBlock {
            mid: ", ",
            last: ", and ",
            once: " and "
        };

        Self::new(iter, &JOIN)
    }

    #[inline]
    pub fn or(iter: impl IntoIterator<Item = &'a str>) -> Self {
        static JOIN: JoinBlock<'static> = JoinBlock {
            mid: ", ",
            last: ", or ",
            once: " or "
        };

        Self::new(iter, &JOIN)
    }
}

impl std::fmt::Display for JoinNatural<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.data.as_slice() {
            [] => Ok(()),
            [one] => f.write_str(one),
            [head, last] => {
                f.write_str(head)?;
                f.write_str(self.join.once)?;
                f.write_str(last)
            },
            [first, mid @ .., last] => {
                f.write_str(first)?;

                for part in mid {
                    f.write_str(self.join.mid)?;
                    f.write_str(part)?;
                }

                f.write_str(self.join.last)?;
                f.write_str(last)
            },
        }
    }
}
