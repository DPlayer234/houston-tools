//! Provides helper methods to work with displayed text.

/// Given a `SNAKE_CASE` string, converts it to title case (i.e. `Snake Case`).
///
/// # Examples
///
/// ```
/// let mut s = String::from("HELLO_NEW_WORLD");
/// utils::text::to_titlecase(&mut s);
/// assert_eq!(&s, "Hello New World");
/// ```
///
/// Or, with a byte string:
/// ```
/// let mut s = b"HELLO_NEW_WORLD".to_vec();
/// utils::text::to_titlecase(&mut s);
/// assert_eq!(&s, b"Hello New World");
/// ```
#[must_use]
pub fn to_titlecase<S: MutStrLike>(value: &mut S) {
	// SAFETY: `to_titlecase_u8` only transforms
	// ASCII characters into other ASCII characters.
	unsafe {
		let slice = value.as_bytes_mut();
		to_titlecase_u8(slice);
	}
}

/// Given an ASCII or UTF-8 [`u8`] slice representing a `SNAKE_CASE` string, converts it to title case (i.e. `Snake Case`).
/// The slice is mutated in-place.
fn to_titlecase_u8(slice: &mut [u8]) {
	let mut is_start = true;

	for item in slice.iter_mut() {
		(*item, is_start) = titlecase_transform(*item, is_start);
	}
}

#[must_use]
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
///
/// Or byte strings, if prefixed with `b:`:
/// ```
/// const TITLE: &[u8] = utils::titlecase!(b: b"HELLO_NEW_WORLD");
/// assert_eq!(TITLE, b"Hello New World");
/// ```
#[macro_export]
macro_rules! titlecase {
	($input:expr) => {{
		// Ensure input is a `&'static str`
		const INPUT_STR: &str = $input;

		// Transmute result back to a str.
		const BYTES: &[u8] = $crate::titlecase!(b: INPUT_STR.as_bytes());
        unsafe { ::std::str::from_utf8_unchecked(BYTES) }
	}};
	(b: $input:expr) => {{
		// Ensure input is a `&'static [u8]`
		const INPUT: &[u8] = $input;

		// Reusable const for byte length
		const N: usize = INPUT.len();

		// Include length in constant for next call.
        const CLONE: [u8; N] = *$crate::as_with_size(INPUT);
		const RESULT: [u8; N] = $crate::text::__private::to_titlecase_u8_array(CLONE);
		&RESULT
	}};
}

/// Joins an arbitrary amount of const [`str`] values.
///
/// Unlike the [`std::concat`] macro, the parameters don't have to be literals, but also aren't stringified.
///
/// # Examples
///
/// ```
/// const BASE: &str = "https://example.com/";
/// const PATH: &str = "cool_page.html";
/// const FRAGMENT: &str = "#best_part";
/// const QUERY: &str = "?bad_stuff=false";
/// const URL: &str = utils::join!(BASE, PATH, FRAGMENT, QUERY);
/// assert_eq!(URL, "https://example.com/cool_page.html#best_part?bad_stuff=false");
/// ```
#[macro_export]
macro_rules! join {
	($($str:expr),*) => {{
		const STRS: &[&str] = &[$($str),*];
		const N: usize = $crate::text::__private::count_str_const(STRS);
		const JOIN: [u8; N] = $crate::text::__private::join_str_const(STRS);
		const RESULT: &str = unsafe { ::std::str::from_utf8_unchecked(&JOIN) };
		RESULT
	}};
}

/// Ensures a string is at most `len` in size.
/// If it exceeds the size, it is truncated to the specified size, including appending ellipses at the end.
#[must_use]
pub fn truncate(str: impl Into<String>, len: usize) -> String {
	let str: String = str.into();
	if str.len() < len { return str; }

	str.chars().take(len - 1)
		.chain(std::iter::once('\u{2026}'))
		.collect()
}

/// Allows conversion of a type to a byte slice, indicating the bytes hold some sort of string data.
///
/// These byte slices do not have to hold UTF8 data, but replacing ASCII codes with other ASCII codes must not invalidate it.
///
/// This exists solely as support [`to_titlecase`].
#[doc(hidden)]
pub unsafe trait MutStrLike {
	unsafe fn as_bytes_mut(&mut self) -> &mut [u8];
}

// Ideally there'd be blanket implementations for DerefMut<Target = str> and DerefMut<Target = [u8]>
// but that's not currently allowed.

unsafe impl MutStrLike for String {
	unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
		self.as_mut_str().as_bytes_mut()
	}
}

unsafe impl MutStrLike for str {
	unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
		self.as_bytes_mut()
	}
}

unsafe impl MutStrLike for [u8] {
	unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
		self
	}
}

unsafe impl MutStrLike for Vec<u8> {
	unsafe fn as_bytes_mut(&mut self) -> &mut [u8] {
		self.as_mut_slice()
	}
}

#[doc(hidden)]
pub mod __private {
	/// Given an ASCII or UTF-8 [`u8`] array representing a `SNAKE_CASE` string, converts it to title case (i.e. `Snake Case`).
	///
	/// This function is generally not useful and exists primarily to support the [`titlecase`] macro.
	#[must_use]
	pub const fn to_titlecase_u8_array<const LEN: usize>(mut value: [u8; LEN]) -> [u8; LEN] {
		let mut is_start = true;

		let mut index = 0usize;
		while index < LEN {
			(value[index], is_start) = super::titlecase_transform(value[index], is_start);
			index += 1;
		}

		value
	}

	/// Counts the total length of all [`str`] slices.
	///
	/// # Panic
	///
	/// Panics if the total length of all slices overflows [`usize`].
	#[must_use]
	pub const fn count_str_const(slices: &[&str]) -> usize {
		let mut offset = 0usize;

		let mut slice_index = 0usize;
		while slice_index < slices.len() {
			offset = match offset.checked_add(slices[slice_index].len()) {
				Some(value) => value,
				None => panic!("total length overflows usize"),
			};
			slice_index += 1;
		}

		offset
	}

	/// Provides a way to join several [`str`] slices into a single UTF8 byte array.
	/// The resulting array is safe to transmute into a [`str`].
	///
	/// This function is generally not useful and exists primarily to support the [`join`] macro.
	///
	/// # Panic
	///
	/// Panics if `N` is not equal to sum of the length of all slices.
	#[must_use]
	pub const fn join_str_const<const N: usize>(slices: &[&str]) -> [u8; N] {
		let mut out = [0u8; N];
		let mut offset = 0usize;

		let mut slice_index = 0usize;
		while slice_index < slices.len() {
			let slice = slices[slice_index].as_bytes();

			let mut index = 0usize;
			while index < slice.len() {
				out[offset + index] = slice[index];
				index += 1;
			}

			offset += slice.len();
			slice_index += 1;
		}

		assert!(offset == N);
		out
	}
}
