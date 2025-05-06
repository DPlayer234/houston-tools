use std::borrow::Cow;

pub mod azur;
pub mod discord;
pub mod join;
pub mod time;

pub use join::Join;

/// Extension methods for [`String`].
pub trait StringExt {
    /// Gets the written [`String`] if it isn't empty.
    ///
    /// Returns [`None`] if it is empty.
    fn or_none(self) -> Option<String>;

    /// Gets the written [`String`] if it isn't empty, or returns the `default`.
    ///
    /// Returns [`Cow::Owned`] with the written value if it isn't empty,
    /// otherwise returns [`Cow::Borrowed`] with `default`.
    fn or_default(self, default: &str) -> Cow<'_, str>;
}

impl StringExt for String {
    fn or_none(self) -> Option<Self> {
        (!self.is_empty()).then_some(self)
    }

    fn or_default(self, default: &str) -> Cow<'_, str> {
        if self.is_empty() {
            Cow::Borrowed(default)
        } else {
            Cow::Owned(self)
        }
    }
}

pub fn replace_holes<F>(mut haystack: &str, mut f: F) -> String
where
    F: FnMut(&mut String, &str),
{
    let mut out = String::with_capacity(haystack.len());

    while let Some(start) = haystack.find('{') {
        let (l, r) = haystack.split_at(start);
        out.push_str(l);

        if let Some(end) = r.find('}') {
            let (l, r) = r.split_at(end + 1);
            debug_assert!(l.len() >= 2, "must be at least 2 bytes long");

            // SAFETY: we must have at least 2 bytes here now, `{` and `}`.
            // `end` is within the range (due to successful split), and must be >=1.
            let name = unsafe { l.get_unchecked(1..end) };

            // call user append function
            f(&mut out, name);

            // update haystack to be the remainder
            haystack = r;
        } else {
            // no closing found, just push the rest and exit
            out.push_str(r);
            return out;
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
            _ => {},
        });

        assert_eq!(result, "Look, look! <@12345> reached <@&67890>!");
    }
}
