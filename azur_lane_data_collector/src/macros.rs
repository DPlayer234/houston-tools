/// Essentially a lazy format format.
#[macro_export]
macro_rules! context {
    ($($arg:tt)*) => {{
        |_| format!($($arg)*)
    }};
}
