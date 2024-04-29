#[macro_export]
macro_rules! context {
    ($val:expr; $($arg:tt)*) => {
        $val.with_context(|_| format!($($arg)*))
    };
}
