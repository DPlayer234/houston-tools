/// Given a `SNAKE_CASE` string, converts it to title case (i.e. `Snake Case`).
/// 
/// # Examples
/// 
/// ```
/// let s = String::from("HELLO_NEW_WORLD");
/// let s = utils::text::to_titlecase(s);
/// assert_eq!(&s, "Hello New World");
/// ```
pub fn to_titlecase(mut value: String) -> String {
	// SAFETY: `to_titlecase_u8` only transforms
	// ASCII characters into other ASCII characters.
	unsafe {
		let slice =  value.as_bytes_mut();
		to_titlecase_u8(slice);
	}
	value
}

/// Given an ASCII or UTF-8 [`u8`] slice representing a `SNAKE_CASE` string, converts it to title case (i.e. `Snake Case`).
/// The slice is mutated in-place.
/// 
/// # Examples
/// 
/// ```
/// let mut s = b"HELLO_NEW_WORLD".to_vec();
/// utils::text::to_titlecase_u8(&mut s);
/// assert_eq!(&s, b"Hello New World");
/// ```
pub fn to_titlecase_u8(slice: &mut [u8]) {
	let mut is_start = true;

	for item in slice.iter_mut() {
		(*item, is_start) = titlecase_transform(*item, is_start);
	}
}

/// Given an ASCII or UTF-8 [`u8`] array representing a `SNAKE_CASE` string, converts it to title case (i.e. `Snake Case`).
/// 
/// This function is generally not useful and exists primarily to support the [`titlecase`] macro.
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
/// 
/// # Examples
/// 
/// ```
/// const TITLE: &str = utils::titlecase!("HELLO_NEW_WORLD");
/// assert_eq!(TITLE, "Hello New World");
/// ```
/// 
/// Also works with lower snake case:
/// ```
/// const TITLE: &str = utils::titlecase!("hello_new_world");
/// assert_eq!(TITLE, "Hello New World");
/// ```
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

pub const unsafe fn join_const_unsafe<const N: usize>(a: &[u8], b: &[u8]) -> [u8; N] {
	const fn copy_slice<const N: usize>(mut out: [u8; N], slice: &[u8], offset: usize) -> [u8; N] {
		let mut index = 0usize;
		while index < slice.len() {
			out[offset + index] = slice[index];
			index += 1;
		}

		out
	}
	
	let out = [0u8; N];
	let out = copy_slice(out, a, 0);
	let out = copy_slice(out, b, a.len());
	out
}

#[macro_export]
macro_rules! join {
	($a:expr, $b:expr) => {{
		const A: &str = $a;
		const B: &str = $b;
		const N: usize = A.len() + B.len();
		const JOIN: [u8; N] = unsafe { $crate::text::join_const_unsafe(A.as_bytes(), B.as_bytes()) };
		const RESULT: &str = unsafe { ::std::str::from_utf8_unchecked(&JOIN) };
		RESULT
	}};
	($a:expr, $b:expr, $c:expr) => {{
		$crate::join!($crate::join!($a, $b), $c)
	}};
}

pub fn truncate(str: impl Into<String>, len: usize) -> String {
	let str: String = str.into();
	if str.len() > len { return str; }

	str.chars().take(len - 1)
		.chain(std::iter::once('\u{2026}'))
		.collect()
}
