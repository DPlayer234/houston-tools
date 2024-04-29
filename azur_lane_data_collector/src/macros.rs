#[macro_export]
macro_rules! context {
    ($val:expr; $($arg:tt)*) => {
        $val.with_context(|_| format!($($arg)*))
    };
}

#[macro_export]
macro_rules! from_const_json_str {
    ($text:expr) => {{
        let text: &str = $text;
        let mut slice = text.as_bytes().to_owned();
        simd_json::from_slice(&mut slice)
    }};
}
