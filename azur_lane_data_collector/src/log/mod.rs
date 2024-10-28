use std::fmt::Arguments;
use std::io::{stdout, Write, Result as IoResult};
use std::time::{Instant, SystemTime};

macro_rules! action {
    ($($t:tt)*) => {
        $crate::log::ActionBuilder::new(std::format!($($t)*))
    };
}

macro_rules! info {
    ($($t:tt)*) => {
        $crate::log::__println(std::format_args!($($t)*));
    };
}

pub(crate) use action;
pub(crate) use info;

const CSI: &str = "\x1b[";
const CSI_FG: &str = "\x1b[38;";

#[doc(hidden)]
pub fn __println(args: Arguments<'_>) {
    writeln_args(stdout(), args);
}

fn writeln_args<W: Write>(mut writer: W, args: Arguments<'_>) {
    _ = writeln!(
        writer,
        "{CSI_FG}5;8m[{}]{CSI}0m {}",
        humantime::format_rfc3339_seconds(SystemTime::now()),
        args,
    );
}

macro_rules! undoln {
    ($writer:expr) => {
        write!($writer, "{CSI}1A{CSI}0K")
    };
}

#[derive(Debug)]
pub struct Action(ActionInner);

impl Action {
    pub fn update(&self) {
        self.0.print_update();
    }

    pub fn print_info(&self, args: Arguments<'_>) {
        self.0.print_info(args);
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
        self.0.finish();
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
        self.0.print_init();
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
    fn print_init(&self) {
        let mut stdout = stdout().lock();
        self.write_state(&mut stdout);
        _ = writeln!(stdout);
    }

    fn print_update(&self) {
        let mut stdout = stdout().lock();
        _ = undoln!(stdout);
        self.write_state(&mut stdout);
        _ = writeln!(stdout);
    }

    fn print_info(&self, args: Arguments<'_>) {
        let mut stdout = stdout().lock();
        _ = undoln!(stdout);
        writeln_args(&mut stdout, args);
        self.write_state(&mut stdout);
        _ = writeln!(stdout);
    }

    fn finish(&self) {
        let mut stdout = stdout().lock();
        _ = undoln!(stdout);
        self.write_state(&mut stdout);
        _ = writeln!(stdout, " {CSI_FG}5;10mDone!{CSI}0m");
    }

    fn write_state<W: Write>(&self, mut writer: W) {
        _ = write!(
            writer,
            "{} {}{}",
            self.start,
            self.progress,
            self.name,
        );
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

impl std::fmt::Display for Start {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{CSI_FG}5;8m[{}] [{:>7.1?}]{CSI}0m",
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

impl std::fmt::Display for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.kind {
            ProgressKind::NotApplicable => Ok(()),
            ProgressKind::Unbounded => write!(f, "{CSI_FG}5;14m[{}{}]{CSI}0m ", self.current, self.suffix),
            ProgressKind::Bounded { total } => write!(f, "{CSI_FG}5;14m[{}/{}{}]{CSI}0m ", self.current, total, self.suffix),
        }
    }
}

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
