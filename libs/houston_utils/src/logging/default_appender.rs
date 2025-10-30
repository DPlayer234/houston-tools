//! Provides a simple console logger.
//!
//! While [`log4rs`] already provides something like this, it doesn't buffer
//! `stderr` output.
//!
//! This appender type is available as `"default"` in the configuration.

use std::io::{self, Write as _};

use arrayvec::ArrayVec;
use log::Record;
use log4rs::append::Append;
use log4rs::config::{Deserialize, Deserializers};
use log4rs::encode::{self, Encode, EncoderConfig, Style};

use super::WRITE_BUF_SIZE;

#[derive(Debug)]
pub struct DefaultAppender {
    encoder: Box<dyn Encode>,
    color: bool,
}

impl Append for DefaultAppender {
    fn append(&self, record: &Record<'_>) -> anyhow::Result<()> {
        let mut writer = ConsoleWriter {
            color: self.color,
            buf: ArrayVec::new_const(),
        };
        self.encoder.encode(&mut writer, record)?;
        Ok(writer.flush()?)
    }

    fn flush(&self) {
        _ = io::stderr().flush();
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct DefaultAppenderConfig {
    color: Option<bool>,
    encoder: EncoderConfig,
}

pub struct DefaultAppenderDeserializer;

impl Deserialize for DefaultAppenderDeserializer {
    type Trait = dyn Append;
    type Config = DefaultAppenderConfig;

    fn deserialize(
        &self,
        config: Self::Config,
        deserializers: &Deserializers,
    ) -> anyhow::Result<Box<Self::Trait>> {
        let encoder = deserializers.deserialize(&config.encoder.kind, config.encoder.config)?;
        let color = config
            .color
            .unwrap_or_else(|| utils::term::supports_ansi_escapes(&io::stderr()));

        Ok(Box::new(DefaultAppender { encoder, color }))
    }
}

/// Stack-buffered writer.
///
/// If a write exceeds the capacity, its buffer is flushed to stderr first.
#[derive(Debug)]
struct ConsoleWriter {
    color: bool,
    buf: ArrayVec<u8, WRITE_BUF_SIZE>,
}

impl io::Write for ConsoleWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if self.buf.remaining_capacity() < buf.len() {
            self.flush()?;
        }

        if buf.len() > self.buf.capacity() {
            io::stderr().write(buf)
        } else {
            self.buf.write(buf)
        }
    }

    fn write_all(&mut self, buf: &[u8]) -> io::Result<()> {
        if self.buf.remaining_capacity() < buf.len() {
            self.flush()?;
        }

        if buf.len() > self.buf.capacity() {
            io::stderr().write_all(buf)
        } else {
            self.buf.write_all(buf)
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut stderr = io::stderr().lock();
        stderr.write_all(&self.buf)?;
        self.buf.clear();
        stderr.flush()?;
        Ok(())
    }
}

impl encode::Write for ConsoleWriter {
    fn set_style(&mut self, style: &Style) -> io::Result<()> {
        use log4rs::encode::Color;
        use utils::term::style::*;

        if self.color {
            self.write_all(RESET.as_bytes())?;

            if let Some(text) = style.text {
                match text {
                    Color::Black => self.write_all(BLACK.as_bytes()),
                    Color::Red => self.write_all(RED.as_bytes()),
                    Color::Green => self.write_all(GREEN.as_bytes()),
                    Color::Yellow => self.write_all(YELLOW.as_bytes()),
                    Color::Blue => self.write_all(BLUE.as_bytes()),
                    Color::Magenta => self.write_all(MAGENTA.as_bytes()),
                    Color::Cyan => self.write_all(CYAN.as_bytes()),
                    Color::White => self.write_all(WHITE.as_bytes()),
                }?;
            }

            if let Some(background) = style.background {
                match background {
                    Color::Black => self.write_all(BLACK_BG.as_bytes()),
                    Color::Red => self.write_all(RED_BG.as_bytes()),
                    Color::Green => self.write_all(GREEN_BG.as_bytes()),
                    Color::Yellow => self.write_all(YELLOW_BG.as_bytes()),
                    Color::Blue => self.write_all(BLUE_BG.as_bytes()),
                    Color::Magenta => self.write_all(MAGENTA_BG.as_bytes()),
                    Color::Cyan => self.write_all(CYAN_BG.as_bytes()),
                    Color::White => self.write_all(WHITE_BG.as_bytes()),
                }?;
            }

            if style.intense == Some(true) {
                self.write_all(BOLD.as_bytes())?;
            }
        }

        Ok(())
    }
}
