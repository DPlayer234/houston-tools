use std::fmt::{Arguments, Display};
use std::io::{Write, Result as IoResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Instant, SystemTime};

mod buf;

/// Creates an action builder with the given label.
macro_rules! action {
    ($($t:tt)*) => {
        $crate::log::ActionBuilder::new(std::format!($($t)*))
    };
}

/// Prints an info message while no action is active.
macro_rules! info {
    ($($t:tt)*) => {
        $crate::log::__info(std::format_args!($($t)*))
    };
}

pub(crate) use action;
pub(crate) use info;

/// When false, uses simplified output.
static USE_ANSI: AtomicBool = AtomicBool::new(false);

/// Sets whether colors are printed.
pub fn use_color(force: Option<bool>) {
    let value = force.unwrap_or_else(|| utils::term::supports_ansi_escapes(&std::io::stderr()));
    USE_ANSI.store(value, Ordering::Relaxed);
}

fn lock_output() -> impl Write {
    buf::buf_stderr()
}

fn only_ansi<F: FnOnce() -> Result<(), E>, E>(f: F) -> Result<(), E> {
    if USE_ANSI.load(Ordering::Relaxed) {
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

impl Display for Ansi {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        only_ansi(|| f.write_str(self.0))
    }
}

#[doc(hidden)]
pub fn __info(args: Arguments<'_>) {
    ioerr(writeln_args(lock_output(), args));
}

fn writeln_args<W: Write>(mut writer: W, args: Arguments<'_>) -> IoResult<()> {
    writeln!(
        writer,
        "{TIME_STYLE}[{}]{RESET} {}",
        humantime::format_rfc3339_seconds(SystemTime::now()),
        args,
    )
}

/// Panics if an [`Err`] variant is passed.
#[track_caller]
fn ioerr<T>(result: IoResult<T>) -> T {
    #[cold]
    #[track_caller]
    fn fail<T>(err: std::io::Error) -> T {
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

    pub fn print_info(&self, args: Arguments<'_>) {
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
    fn print_init(&self) -> IoResult<()> {
        let mut out = lock_output();
        writeln!(out, "{self}")
    }

    fn print_update(&self) -> IoResult<()> {
        only_ansi(|| {
            let mut out = lock_output();
            writeln!(out, "{UNDO_LINE}{self}")
        })
    }

    fn print_info(&self, args: Arguments<'_>) -> IoResult<()> {
        let mut out = lock_output();
        if USE_ANSI.load(Ordering::Relaxed) {
            write!(out, "{UNDO_LINE}")?;
            writeln_args(&mut out, args)?;
            writeln!(out, "{self}")
        } else {
            writeln_args(&mut out, args)
        }
    }

    fn finish(&self) -> IoResult<()> {
        let mut out = lock_output();
        writeln!(out, "{UNDO_LINE}{self} {DONE_STYLE}Done!{RESET}")
    }
}

impl Display for ActionInner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let Self { start, progress, name } = self;
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

impl Display for Start {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
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
    Bounded {
        total: usize,
    },
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

impl Display for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ProgressKind::NotApplicable => Ok(()),
            ProgressKind::Unbounded => write!(f, "{PROGRESS_STYLE}[{}{}]{RESET} ", self.current, self.suffix),
            ProgressKind::Bounded { total } => write!(f, "{PROGRESS_STYLE}[{}/{}{}]{RESET} ", self.current, total, self.suffix),
        }
    }
}

#[derive(Debug)]
pub struct ActionWrite<W, const C: usize = 0x20000> {
    action: Action,
    writer: W,
    total: usize,
    flush: usize,
}

impl<W: Write> ActionWrite<W> {
    pub fn new(action: Action, writer: W) -> Self {
        Self::with_chunk(action, writer)
    }

    pub fn with_chunk<const CHUNK: usize>(action: Action, writer: W) -> ActionWrite<W, CHUNK> {
        ActionWrite {
            action,
            writer,
            total: 0,
            flush: 0,
        }
    }
}

impl<W: Write, const CHUNK: usize> ActionWrite<W, CHUNK> {
    pub fn finish(mut self) {
        self.action.0.progress.current = self.total_kb();
        self.action.finish();
    }

    fn total_kb(&self) -> usize {
        self.total / 1024
    }

    fn update_count(&mut self, len: usize) {
        self.total += len;
        self.flush += len;
        if self.flush > CHUNK {
            self.flush = 0;
            self.action.update_amount(self.total_kb());
        }
    }
}

impl<W: Write, const CHUNK: usize> Write for ActionWrite<W, CHUNK> {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        let len = self.writer.write(buf)?;
        self.update_count(len);
        Ok(len)
    }

    fn flush(&mut self) -> IoResult<()> {
        self.flush = 0;
        self.action.update_amount(self.total_kb());
        self.writer.flush()
    }

    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> IoResult<usize> {
        let len = self.writer.write_vectored(bufs)?;
        self.update_count(len);
        Ok(len)
    }

    fn write_all(&mut self, buf: &[u8]) -> IoResult<()> {
        self.writer.write_all(buf)?;
        self.update_count(buf.len());
        Ok(())
    }
}
