pub mod style;

/// Performs automatic detection of whether ANSI escape codes are supported.
pub fn supports_ansi_escapes<T: std::io::IsTerminal>(stream: &T) -> bool {
    use anstyle_query::*;

    let clicolor = clicolor();
    if no_color() {
        false
    } else if clicolor_force() {
        true
    } else if clicolor == Some(false) {
        false
    } else {
        stream.is_terminal() &&
        (term_supports_color() || clicolor == Some(true) || is_ci())
    }
}
