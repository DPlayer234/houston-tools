macro_rules! define_escapes {
    ($($name:ident = $lit:literal,)*) => {
        $(
            pub const $name: &str = concat!("\x1b[", $lit);
        )*
    };
}

define_escapes! {
    RESET = "0m",

    BOLD = "1m",
    ITALIC = "3m",
    UNDERLINE = "4m",

    BLACK = "30m",
    RED = "31m",
    GREEN = "32m",
    YELLOW = "33m",
    BLUE = "34m",
    MAGENTA = "35m",
    CYAN = "36m",
    WHITE = "37m",

    GRAY = "38;5;8m",
    BRIGHT_RED = "38;5;9m",
    BRIGHT_GREEN = "38;5;10m",
    BRIGHT_YELLOW = "38;5;11m",
    BRIGHT_BLUE = "38;5;12m",
    BRIGHT_MAGENTA = "38;5;13m",
    BRIGHT_CYAN = "38;5;14m",
    BRIGHT_WHITE = "38;5;15m",

    DEFAULT_COLOR = "39m",
}
