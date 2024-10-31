use std::io::Write;

use env_logger::fmt::Formatter;
use log::{Level, Record};

use utils::term::style::*;

macro_rules! format_record {
    ($buf:expr, $record:expr => $subdued_style:expr, $level_style:expr, $reset:expr) => {
        writeln!(
            $buf,
            "{s}[{r}{} {l}{:<5}{r} {}{s}]{r} {}",
            ::humantime::format_rfc3339_seconds(::std::time::SystemTime::now()),
            $record.level(),
            $record.module_path().unwrap_or("<unknown>"),
            $record.args(),
            s = $subdued_style,
            l = $level_style,
            r = $reset,
        )
    };
}

pub fn format_styled(buf: &mut Formatter, record: &Record<'_>) -> std::io::Result<()> {
    let subdued = utils::join!(RESET, GRAY);
    let level_style = match record.level() {
        Level::Error => utils::join!(RED, BOLD),
        Level::Warn => YELLOW,
        Level::Info => GREEN,
        Level::Debug => BLUE,
        Level::Trace => CYAN,
    };

    format_record!(buf, record => subdued, level_style, RESET)
}

pub fn format_unstyled(buf: &mut Formatter, record: &Record<'_>) -> std::io::Result<()> {
    format_record!(buf, record => "", "", "")
}
