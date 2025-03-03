use std::borrow::Cow;

pub mod azur;
pub mod discord;
pub mod join;
pub mod time;

pub use join::Join;

/// If non-empty, turns the string into a [`Cow::Owned`].
///
/// Otherwise returns a [`Cow::Borrowed`] with the `default`.
pub fn written_or(string: String, default: &str) -> Cow<'_, str> {
    if string.is_empty() {
        Cow::Borrowed(default)
    } else {
        Cow::Owned(string)
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
    use utils::text::write_str::*;

    use super::replace_holes;

    #[test]
    fn replace_holes_ok() {
        let user = 12345;
        let role = 67890;
        let haystack = "Look, look! {user} reached {role}!";

        let result = replace_holes(haystack, |out, n| match n {
            "user" => write_str!(out, "<@{user}>"),
            "role" => write_str!(out, "<@&{role}>"),
            _ => {},
        });

        assert_eq!(result, "Look, look! <@12345> reached <@&67890>!");
    }
}
