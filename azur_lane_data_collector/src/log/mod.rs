use std::fmt;
use std::io::{self, Write as _};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime};

mod buf;
mod write;

/// Creates an action builder with the given label.
macro_rules! action {
    ($($t:tt)*) => {
        $crate::log::ActionBuilder::new(::std::format!($($t)*))
    };
}

/// Prints an info message while no action is active.
macro_rules! info {
    ($($t:tt)*) => {
        $crate::log::__info(::std::format_args!($($t)*))
    };
}

pub(crate) use write::ActionWrite;
pub(crate) use {action, info};

/// When false, uses simplified output.
static USE_ANSI: AtomicBool = AtomicBool::new(false);

/// Sets whether colors are printed.
pub fn use_color(force: Option<bool>) {
    let value = force.unwrap_or_else(|| utils::term::supports_ansi_escapes(&io::stderr()));
    USE_ANSI.store(value, Ordering::Release);
}

fn lock_output() -> impl io::Write {
    buf::buf_stderr()
}

fn only_ansi<F: FnOnce() -> Result<(), E>, E>(f: F) -> Result<(), E> {
    if USE_ANSI.load(Ordering::Acquire) {
        f()
    } else {
        Ok(())
    }
}

/// Escape sequence to be only printed when CI is false.
#[derive(Debug)]
#[repr(transparent)]
struct Ansi(&'static str);

const RESET: Ansi = Ansi(utils::term::style::RESET);
const TIME_STYLE: Ansi = Ansi(utils::term::style::GRAY);
const DONE_STYLE: Ansi = Ansi(utils::term::style::BRIGHT_GREEN);
const PROGRESS_STYLE: Ansi = Ansi(utils::term::style::BRIGHT_CYAN);
const UNDO_LINE: Ansi = Ansi("\x1b[1A\x1b[0K");

impl fmt::Display for Ansi {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        only_ansi(|| f.write_str(self.0))
    }
}

#[doc(hidden)]
pub fn __info(args: fmt::Arguments<'_>) {
    ioerr(writeln_args(lock_output(), args));
}

fn writeln_args<W: io::Write>(mut writer: W, args: fmt::Arguments<'_>) -> io::Result<()> {
    writeln!(
        writer,
        "{TIME_STYLE}[{}]{RESET} {}",
        humantime::format_rfc3339_seconds(SystemTime::now()),
        args,
    )
}

/// Panics if an [`Err`] variant is passed.
#[track_caller]
fn ioerr<T>(result: io::Result<T>) -> T {
    #[cold]
    #[track_caller]
    fn fail<T>(err: io::Error) -> T {
        panic!("failed writing to stderr: {err:?}");
    }

    result.unwrap_or_else(fail)
}

#[derive(Debug)]
pub struct Action(ActionInner);

impl Action {
    pub fn update(&self) {
        ioerr(self.0.print_update());
    }

    pub fn print_info(&self, args: fmt::Arguments<'_>) {
        ioerr(self.0.print_info(args));
    }

    pub fn update_amount(&mut self, amount: usize) {
        self.0.progress.current = amount;
        self.update();
    }

    pub fn inc_amount(&mut self) {
        self.0.progress.current += 1;
        self.update();
    }

    pub fn amount(&self) -> usize {
        self.0.progress.current
    }

    pub fn finish(self) {
        drop(self);
    }
}

impl Drop for Action {
    fn drop(&mut self) {
        ioerr(self.0.finish());
    }
}

#[derive(Debug)]
pub struct ActionBuilder(ActionInner);

impl ActionBuilder {
    pub fn new(name: String) -> Self {
        Self(ActionInner {
            name,
            progress: Progress::new(),
            start: Start::now(),
        })
    }

    pub fn unbounded(mut self) -> Self {
        self.0.progress.kind = ProgressKind::Unbounded;
        self
    }

    pub fn bounded_total(mut self, total: usize) -> Self {
        self.0.progress.kind = ProgressKind::Bounded { total };
        self
    }

    pub fn suffix(mut self, suffix: &'static str) -> Self {
        self.0.progress.suffix = suffix;
        self
    }

    pub fn start(self) -> Action {
        ioerr(self.0.print_init());
        Action(self.0)
    }
}

#[derive(Debug)]
struct ActionInner {
    name: String,
    progress: Progress,
    start: Start,
}

impl ActionInner {
    fn print_init(&self) -> io::Result<()> {
        let mut out = lock_output();
        writeln!(out, "{self}")
    }

    fn print_update(&self) -> io::Result<()> {
        only_ansi(|| {
            let mut out = lock_output();
            writeln!(out, "{UNDO_LINE}{self}")
        })
    }

    fn print_info(&self, args: fmt::Arguments<'_>) -> io::Result<()> {
        let mut out = lock_output();
        if USE_ANSI.load(Ordering::Acquire) {
            write!(out, "{UNDO_LINE}")?;
            writeln_args(&mut out, args)?;
            writeln!(out, "{self}")
        } else {
            writeln_args(&mut out, args)
        }
    }

    fn finish(&self) -> io::Result<()> {
        let mut out = lock_output();
        writeln!(out, "{UNDO_LINE}{self} {DONE_STYLE}Done!{RESET}")
    }
}

impl fmt::Display for ActionInner {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self {
            start,
            progress,
            name,
        } = self;
        write!(f, "{start} {progress}{name}")
    }
}

#[derive(Debug)]
struct Start {
    instant: Instant,
    local: SystemTime,
}

impl Start {
    fn now() -> Self {
        Self {
            instant: Instant::now(),
            local: SystemTime::now(),
        }
    }
}

impl fmt::Display for Start {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{TIME_STYLE}[{}] [{:>7.1?}]{RESET}",
            humantime::format_rfc3339_seconds(self.local),
            self.instant.elapsed(),
        )
    }
}

#[derive(Debug)]
struct Progress {
    current: usize,
    suffix: &'static str,
    kind: ProgressKind,
}

#[derive(Debug)]
enum ProgressKind {
    NotApplicable,
    Unbounded,
    Bounded { total: usize },
}

impl Progress {
    fn new() -> Self {
        Self {
            current: 0,
            suffix: "",
            kind: ProgressKind::NotApplicable,
        }
    }
}

impl fmt::Display for Progress {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            ProgressKind::NotApplicable => Ok(()),
            ProgressKind::Unbounded => write!(
                f,
                "{PROGRESS_STYLE}[{}{}]{RESET} ",
                self.current, self.suffix
            ),
            ProgressKind::Bounded { total } => write!(
                f,
                "{PROGRESS_STYLE}[{}/{}{}]{RESET} ",
                self.current, total, self.suffix
            ),
        }
    }
}
