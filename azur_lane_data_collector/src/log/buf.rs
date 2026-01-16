//! Buffered output writing.
//!
//! This is not necessary for _correctness_, however since this app updates
//! already printed lines, including clearing them, the output can be quite
//! flickery and slow.
//!
//! And while stdout is buffered by default, stderr seems like the more correct
//! option.

use std::fmt;
use std::io::{IoSlice, LineWriter, Result, Stderr, Write, stderr};
use std::sync::{Mutex, MutexGuard, OnceLock, PoisonError};

/// Provides a line-buffered wrapper around [`stderr`].
///
/// Calling this function while a [`BufStderr`] is already in use will deadlock.
pub fn buf_stderr() -> BufStderr {
    // ideally we'd copy what stdout does, but `ReentrantLock` isn't stable yet, so
    // we instead use a `Mutex` and skip the `RefCell` to get a _similar_ effect.
    // note that this will deadlock instead of panic on recursive use.
    static STDERR: OnceLock<Mutex<LineWriter<Stderr>>> = OnceLock::new();

    let inner = STDERR
        .get_or_init(|| Mutex::new(LineWriter::new(stderr())))
        .lock()
        .unwrap_or_else(PoisonError::into_inner);

    BufStderr { inner }
}

/// A line-buffered writer to [`Stderr`].
pub struct BufStderr {
    inner: MutexGuard<'static, LineWriter<Stderr>>,
}

impl Write for BufStderr {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner.write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.inner.write_all(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        self.inner.write_vectored(bufs)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner.flush()
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> Result<()> {
        self.inner.write_fmt(fmt)
    }
}
