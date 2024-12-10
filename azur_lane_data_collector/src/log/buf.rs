//! Buffered output writing.
//!
//! This is not necessary for _correctness_, however since this app updates
//! already printed lines, including clearing them, the output can be quite
//! flickery and slow.
//!
//! And while stdout is buffered by default, stderr seems like the more correct
//! option.

use std::fmt;
use std::io::{stderr, IoSlice, LineWriter, Result, Stderr, Write};
use std::sync::{Mutex, MutexGuard};

/// Provides a line-buffered wrapper around [`stderr`].
///
/// Re-entrant use will deadlock.
pub fn buf_stderr() -> BufStderr {
    static INSTANCE: Mutex<Option<LineWriter<Stderr>>> = Mutex::new(None);

    let mut inner = INSTANCE.lock().unwrap();
    inner.get_or_insert_with(|| LineWriter::new(stderr()));

    BufStderr { inner }
}

/// A line-buffered writer to [`Stderr`].
pub struct BufStderr {
    // safety note: must always be `Some`
    inner: MutexGuard<'static, Option<LineWriter<Stderr>>>,
}

impl BufStderr {
    fn inner(&mut self) -> &mut LineWriter<Stderr> {
        debug_assert!(self.inner.is_some(), "inner must be Some");

        // SAFETY: always constructed with `Some`.
        unsafe { self.inner.as_mut().unwrap_unchecked() }
    }
}

impl Write for BufStderr {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        self.inner().write(buf)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.inner().write_all(buf)
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        self.inner().write_vectored(bufs)
    }

    fn flush(&mut self) -> Result<()> {
        self.inner().flush()
    }

    fn write_fmt(&mut self, fmt: fmt::Arguments<'_>) -> Result<()> {
        self.inner().write_fmt(fmt)
    }
}
