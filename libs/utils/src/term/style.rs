//! Support for styling terminal text via ANSI escape sequences.
//!
//! There isn't any magic here, just [`str`] constants with the escape
//! sequences, so you will need to ensure support yourself or via
//! [`super::supports_ansi_escapes`].
//!
//! # Examples
//!
//! ```
//! use utils::term::style::*;
//!
//! // make "very important" bold and red
//! println!("This is {BOLD}{RED}very important{RESET}.");
//! ```

macro_rules! define_escapes {
    ($($(#[$attr:meta])* $name:ident = $lit:literal,)*) => {
        $(
            $(#[$attr])*
            pub const $name: &str = concat!("\x1b[", $lit);
        )*
    };
}

macro_rules! define_color_escapes {
    ($($label:literal $name:ident / $name_bg:ident = $lit:literal,)*) => {
        $(
            #[doc = concat!("Change the foreground color to ", $label, ".")]
            pub const $name: &str = concat!("\x1b[38;5;", $lit);
        )*
        $(
            #[doc = concat!("Change the background color to ", $label, ".")]
            pub const $name_bg: &str = concat!("\x1b[48;5;", $lit);
        )*
    };
}

define_escapes! {
    /// Resets all styles and colors.
    RESET = "0m",

    /// Bold text.
    BOLD = "1m",
    /// Italic text.
    ITALIC = "3m",
    /// Underlined text.
    UNDERLINE = "4m",

    /// Change the foreground color to the terminal's default.
    DEFAULT_COLOR = "39m",
    /// Change the background color to the terminal's default.
    DEFAULT_COLOR_BG = "49m",
}

define_color_escapes! {
    "black" BLACK / BLACK_BG = "0m",
    "red" RED / RED_BG = "1m",
    "green" GREEN / GREEN_BG = "2m",
    "yellow" YELLOW / YELLOW_BG = "3m",
    "blue" BLUE / BLUE_BG = "4m",
    "magenta" MAGENTA / MAGENTA_BG = "5m",
    "cyan" CYAN / CYAN_BG = "6m",
    "white" WHITE / WHITE_BG = "7m",

    "gray" GRAY / GRAY_BG = "8m",
    "bright red" BRIGHT_RED / BRIGHT_RED_BG = "9m",
    "bright green" BRIGHT_GREEN / BRIGHT_GREEN_BG = "10m",
    "bright yellow" BRIGHT_YELLOW / BRIGHT_YELLOW_BG = "11m",
    "bright blue" BRIGHT_BLUE / BRIGHT_BLUE_BG = "12m",
    "bright magenta" BRIGHT_MAGENTA / BRIGHT_MAGENTA_BG = "13m",
    "bright cyan" BRIGHT_CYAN / BRIGHT_CYAN_BG = "14m",
    "bright white" BRIGHT_WHITE / BRIGHT_WHITE_BG = "15m",
}
