use std::fmt::Arguments;
use std::io::{stdout, Write};
use std::time::{Instant, SystemTime};

macro_rules! action {
    ($($t:tt)*) => {
        $crate::log::ActionBuilder::new(std::format!($($t)*))
    };
}

macro_rules! println {
    ($($t:tt)*) => {
        $crate::log::__println(std::format_args!($($t)*));
    };
}

pub(crate) use action;
pub(crate) use println;

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

pub struct Action(ActionInner);

impl Action {
    pub fn update(&self) {
        self.0.print_update();
    }

    pub fn run_busy<F, T>(&self, f: F) -> T
    where
        F: FnOnce() -> T + Send,
        T: Send,
    {
        use std::thread::{scope, sleep};
        use std::time::Duration;

        scope(|scope| {
            let handle = scope.spawn(f);

            while !handle.is_finished() {
                self.update();
                sleep(Duration::from_millis(100));
            }

            handle.join().unwrap()
        })
    }

    pub fn print_info(&self, args: Arguments<'_>) {
        self.0.print_info(args);
    }

    pub fn update_amount(&mut self, amount: usize) {
        if let Some(old_amount) = self.0.progress.current_mut() {
            *old_amount = amount;
        }

        self.update();
    }

    pub fn inc_amount(&mut self) {
        if let Some(old_amount) = self.0.progress.current_mut() {
            *old_amount += 1;
        }

        self.update();
    }

    pub fn amount(&self) -> usize {
        self.0.progress.current().copied().unwrap_or(0)
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

pub struct ActionBuilder(ActionInner);

impl ActionBuilder {
    pub fn new(name: String) -> Self {
        Self(ActionInner {
            name,
            progress: Progress::NotApplicable,
            start: Start::now(),
        })
    }

    pub fn unbounded(mut self) -> Self {
        self.0.progress = Progress::Unbounded { current: 0 };
        self
    }

    pub fn bounded_total(mut self, total: usize) -> Self {
        self.0.progress = Progress::Bounded { current: 0, total };
        self
    }

    pub fn start(self) -> Action {
        self.0.print_init();
        Action(self.0)
    }
}

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
        _ = write!(stdout, "{CSI}1A{CSI}0K");
        self.write_state(&mut stdout);
        _ = writeln!(stdout);
    }

    fn print_info(&self, args: Arguments<'_>) {
        let mut stdout = stdout().lock();
        _ = write!(stdout, "{CSI}1A{CSI}0K");
        writeln_args(&mut stdout, args);
        self.write_state(&mut stdout);
        _ = writeln!(stdout);
    }

    fn finish(&self) {
        let mut stdout = stdout().lock();
        _ = write!(stdout, "{CSI}1A{CSI}0K");
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

enum Progress {
    NotApplicable,
    Unbounded {
        current: usize,
    },
    Bounded {
        current: usize,
        total: usize,
    },
}

impl Progress {
    fn current(&self) -> Option<&usize> {
        match self {
            Self::NotApplicable => None,
            Self::Unbounded { current } => Some(current),
            Self::Bounded { current, .. } => Some(current),
        }
    }

    fn current_mut(&mut self) -> Option<&mut usize> {
        match self {
            Self::NotApplicable => None,
            Self::Unbounded { current } => Some(current),
            Self::Bounded { current, .. } => Some(current),
        }
    }
}

impl std::fmt::Display for Progress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotApplicable => Ok(()),
            Self::Unbounded { current } => write!(f, "{CSI_FG}5;14m[{}/?]{CSI}0m ", current),
            Self::Bounded { current, total } => write!(f, "{CSI_FG}5;14m[{}/{}]{CSI}0m ", current, total),
        }
    }
}
