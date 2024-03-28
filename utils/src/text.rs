pub fn to_titlecase(mut value: String) -> String {
	let slice = unsafe { value.as_bytes_mut() };
	let mut is_start = true;

	for item in slice.iter_mut() {
		(*item, is_start) = titlecase_transform(*item, is_start);
	}

	value
}

pub const fn to_titlecase_u8_array<const LEN: usize>(mut value: [u8; LEN]) -> [u8; LEN] {
	let mut is_start = true;

	let mut index = 0usize;
	while index < LEN {
		(value[index], is_start) = titlecase_transform(value[index], is_start);
		index += 1;
	}

	value
}

const fn titlecase_transform(c: u8, is_start: bool) -> (u8, bool) {
	if c == b'_' {
		(b' ', true)
	} else if !is_start {
		(c.to_ascii_lowercase(), false)
	} else {
		(c.to_ascii_uppercase(), false)
	}
}

/// Transforms a const [`str`] in `SNAKE_CASE` format into titlecase version (i.e. `Snake Case`).
/// The resulting value is still const.
#[macro_export]
macro_rules! titlecase {
	($input:expr) => {{
		// Ensure input is a `&'static str`
		const INPUT: &str = $input;
		
		// Reusable const for byte length
		const N: usize = INPUT.len();

		// Include length in constant for next call.
		// This is also in part necessary to satisfy the borrow checker.
		// This value has to exist during the call to `from_utf8_unchecked`, and inlining it wouldn't allow that.
        const CLONE: [u8; N] = *$crate::as_with_size(INPUT.as_bytes());

		// Modify and convert back to str
        const RESULT: &str = unsafe { ::std::str::from_utf8_unchecked(&$crate::text::to_titlecase_u8_array(CLONE)) };
        RESULT
	}}
}

#[cfg(test)]
mod tests {
	#[test]
	fn titlecase_from_uppercase() {
		const TITLE: &str = titlecase!("HELLO_NEW_WORLD");
		assert_eq!(TITLE, "Hello New World");
	}

	#[test]
	fn titlecase_from_lowercase() {
		const TITLE: &str = titlecase!("hello_new_world");
		assert_eq!(TITLE, "Hello New World");
	}
}
