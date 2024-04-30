/// Includes contextual information with a Lua error/result.
#[macro_export]
macro_rules! context {
    ($val:expr; $($arg:tt)*) => {
        $val.with_context(|_| format!($($arg)*))
    };
}
