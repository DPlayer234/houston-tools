//! Provides utilities for dealing with terminals.

use std::io;

pub mod style;

/// Performs automatic detection of whether ANSI escape codes are supported.
pub fn supports_ansi_escapes<T: io::IsTerminal>(stream: &T) -> bool {
    use anstyle_query as a;

    let clicolor = a::clicolor();
    if a::no_color() {
        false
    } else if a::clicolor_force() {
        true
    } else if clicolor == Some(false) {
        false
    } else {
        stream.is_terminal() && (a::term_supports_color() || clicolor == Some(true) || a::is_ci())
    }
}
