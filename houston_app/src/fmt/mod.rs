use std::borrow::Cow;

pub mod azur;
pub mod discord;
pub mod join;
pub mod time;

pub use join::Join;

/// Extension methods for [`String`].
pub trait StringExt<'this>: Sized {
    /// Gets the written [`String`] if it isn't empty.
    ///
    /// Returns [`None`] if it is empty.
    fn or_none(self) -> Option<Self>;

    /// Gets the written [`String`] if it isn't empty, or returns the `default`.
    ///
    /// Returns [`Cow::Owned`] with the written value if it isn't empty,
    /// otherwise returns [`Cow::Borrowed`] with `default`.
    fn or_default(self, default: &'this str) -> Cow<'this, str>;
}

impl<'this> StringExt<'this> for String {
    fn or_none(self) -> Option<Self> {
        (!self.is_empty()).then_some(self)
    }

    fn or_default(self, default: &'this str) -> Cow<'this, str> {
        if self.is_empty() {
            Cow::Borrowed(default)
        } else {
            Cow::Owned(self)
        }
    }
}

impl<'this> StringExt<'this> for Cow<'this, str> {
    fn or_none(self) -> Option<Self> {
        (!self.is_empty()).then_some(self)
    }

    fn or_default(self, default: &'this str) -> Self {
        if self.is_empty() {
            Self::Borrowed(default)
        } else {
            self
        }
    }
}

pub fn replace_holes<F>(mut haystack: &str, mut f: F) -> String
where
    F: FnMut(&mut String, &str),
{
    let mut out = String::with_capacity(haystack.len());

    while let Some(start) = haystack.find('{') {
        let (raw, rest) = haystack.split_at(start);
        out.push_str(raw);

        if let Some(end) = rest.find('}') {
            let (hole, rest) = rest.split_at(end + 1);
            debug_assert!(hole.len() >= 2, "must be at least 2 bytes long");
            debug_assert!(end >= 1, "end must be >= 1");
            debug_assert!(
                hole.is_char_boundary(1) && hole.is_char_boundary(end),
                "must have a char boundaries at index 1 and {end}"
            );

            // SAFETY: we must have at least 2 bytes here now, `{` and `}`.
            // `end` is within the range (due to successful split), and must be >=1.
            // `{` is a 1-byte char, so 1 must be a char boundary.
            let name = unsafe { hole.get_unchecked(1..end) };

            // call user append function
            f(&mut out, name);

            // update haystack to be the remainder
            haystack = rest;
        } else {
            // no closing found, mark the rest to be pushed and break out
            haystack = rest;
            break;
        }
    }

    // push the remainder and return
    out.push_str(haystack);
    out
}

#[cfg(test)]
mod tests {
    use utils::text::WriteStr as _;

    use super::replace_holes;

    #[test]
    fn replace_holes_ok() {
        let user = 12345;
        let role = 67890;
        let haystack = "Look, look! {user} reached {role}!";

        let result = replace_holes(haystack, |out, n| match n {
            "user" => write!(out, "<@{user}>"),
            "role" => write!(out, "<@&{role}>"),
            _ => unreachable!(),
        });

        assert_eq!(result, "Look, look! <@12345> reached <@&67890>!");
    }

    #[test]
    fn replace_holes_odd() {
        let haystack = "{start} Here is an empty hole: {} {end}";
        let result = replace_holes(haystack, |out, n| match n {
            "" => out.push_str("<empty>"),
            "start" => out.push('^'),
            "end" => out.push('$'),
            _ => unreachable!(),
        });

        assert_eq!(result, "^ Here is an empty hole: <empty> $");
    }

    #[test]
    fn replace_holes_all() {
        let haystack = "{this is a singular huge hole to fill}";
        let result = replace_holes(haystack, |out, n| match n {
            "this is a singular huge hole to fill" => out.push_str("hello world"),
            _ => unreachable!(),
        });

        assert_eq!(result, "hello world");
    }
}
