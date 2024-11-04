use std::io::{IoSlice, Result, Write};

use super::Action;

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
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        let len = self.writer.write(buf)?;
        self.update_count(len);
        Ok(len)
    }

    fn flush(&mut self) -> Result<()> {
        self.flush = 0;
        self.action.update_amount(self.total_kb());
        self.writer.flush()
    }

    fn write_vectored(&mut self, bufs: &[IoSlice<'_>]) -> Result<usize> {
        let len = self.writer.write_vectored(bufs)?;
        self.update_count(len);
        Ok(len)
    }

    fn write_all(&mut self, buf: &[u8]) -> Result<()> {
        self.writer.write_all(buf)?;
        self.update_count(buf.len());
        Ok(())
    }
}
