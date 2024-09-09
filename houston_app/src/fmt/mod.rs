use std::fmt::{Write, Display};

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
