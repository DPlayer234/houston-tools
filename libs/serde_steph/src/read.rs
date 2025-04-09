//! Exposes a specialized reader trait.

use std::io;

use crate::error::{Error, Result};

/// Returns an [`io::Error`] with kind [`io::ErrorKind::UnexpectedEof`].
fn eof() -> Error {
    // this doesn't have quite the right error message, but it doesn't allocate and
    // honestly who cares whether it says it's eof or that it couldn't fill a buffer
    io::Error::from(io::ErrorKind::UnexpectedEof).into()
}

/// Specialized reader trait for use with [`Deserializer`](super::Deserializer).
///
/// By default, this is implemented for [`SliceRead`], [`IoRead`] and mutable
/// references to [`Read`] implementations.
///
/// This trait also allows access to borrowed data if supported at runtime.
/// `'de` represents that borrowed lifetime and is otherwise unused.
pub trait Read<'de>: io::Read {
    /// Reads a constant size chunk of bytes.
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N]>;

    /// Reads a chunk of bytes, possibly borrowed from the reader for the
    /// duration of the call.
    fn read_byte_view<F, T>(&mut self, len: usize, access: F) -> Result<T>
    where
        F: FnOnce(&[u8]) -> Result<T>;

    /// Reads a chunk of bytes, returning it as a newly allocated [`Vec`].
    fn read_byte_vec(&mut self, len: usize) -> Result<Vec<u8>>;

    /// Attempts to read a chunk of bytes, borrowing from the reader.
    ///
    /// If the reader supports borrowing data from it, returns [`Some`] with the
    /// result of the operation. If the reader does not support it, returns
    /// [`None`] without advancing.
    ///
    /// If [`None`] was returned, calling another reader method with the same
    /// `len` must have the same result as if this method was never called.
    fn try_read_bytes_borrow(&mut self, len: usize) -> Option<Result<&'de [u8]>> {
        _ = len;
        None
    }
}

// this implementation is required so the reader can be reborrowed
impl<'de, R: Read<'de>> Read<'de> for &mut R {
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        (**self).read_bytes()
    }

    fn read_byte_view<F, T>(&mut self, len: usize, access: F) -> Result<T>
    where
        F: FnOnce(&[u8]) -> Result<T>,
    {
        (**self).read_byte_view(len, access)
    }

    fn read_byte_vec(&mut self, len: usize) -> Result<Vec<u8>> {
        (**self).read_byte_vec(len)
    }

    fn try_read_bytes_borrow(&mut self, len: usize) -> Option<Result<&'de [u8]>> {
        (**self).try_read_bytes_borrow(len)
    }
}

/// Wraps a slice so it can be used as a [`Read`].
///
/// You cannot directly construct this type. Instead use
/// [`Deserializer::from_slice`](super::Deserializer::from_slice).
#[derive(Debug)]
pub struct SliceRead<'de> {
    pub(super) slice: &'de [u8],
}

impl<'de> SliceRead<'de> {
    pub(crate) fn new(slice: &'de [u8]) -> Self {
        Self { slice }
    }

    #[inline]
    fn read_bytes_borrow(&mut self, len: usize) -> Result<&'de [u8]> {
        let (out, rem) = self.slice.split_at_checked(len).ok_or_else(eof)?;
        self.slice = rem;
        Ok(out)
    }
}

impl<'de> Read<'de> for SliceRead<'de> {
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        let (out, rem) = self.slice.split_first_chunk::<N>().ok_or_else(eof)?;
        self.slice = rem;
        Ok(*out)
    }

    fn read_byte_view<F, T>(&mut self, len: usize, access: F) -> Result<T>
    where
        F: FnOnce(&[u8]) -> Result<T>,
    {
        self.read_bytes_borrow(len).and_then(access)
    }

    fn read_byte_vec(&mut self, len: usize) -> Result<Vec<u8>> {
        self.read_bytes_borrow(len).map(<[u8]>::to_vec)
    }

    fn try_read_bytes_borrow(&mut self, len: usize) -> Option<Result<&'de [u8]>> {
        Some(self.read_bytes_borrow(len))
    }
}

impl io::Read for SliceRead<'_> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.slice.read(buf)
    }
}

/// Wraps a [`io::Read`] implementation so it can be used as a [`Read`].
///
/// You cannot directly construct this type. Instead use
/// [`Deserializer::from_reader`](super::Deserializer::from_reader).
#[derive(Debug)]
pub struct IoRead<R> {
    pub(super) inner: R,
}

impl<R> IoRead<R> {
    pub(crate) fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R: io::Read> Read<'_> for IoRead<R> {
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N]> {
        let mut buf = [0u8; N];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_byte_view<F, T>(&mut self, len: usize, access: F) -> Result<T>
    where
        F: FnOnce(&[u8]) -> Result<T>,
    {
        const STACK: usize = 0x1000;

        if len <= STACK {
            let mut buf = [0u8; STACK];
            let buf = &mut buf[..len];
            self.inner.read_exact(buf)?;
            access(buf)
        } else {
            // allocate if more than 4KiB is requested. we don't want to blow up the stack
            // in case the data is wrong. this should also be the only code path that
            // allocates unless the serializer asks for an allocation.
            let vec = self.read_byte_vec(len)?;
            access(&vec)
        }
    }

    #[inline(never)]
    fn read_byte_vec(&mut self, len: usize) -> Result<Vec<u8>> {
        use std::io::Read as _;

        // don't allocate too much or incorrect data could lead to a DoS
        let capacity = len.min(0x1000);
        let mut buf = Vec::with_capacity(capacity);
        let limit = u64::try_from(len).map_err(|_| eof())?;
        self.inner.by_ref().take(limit).read_to_end(&mut buf)?;

        if buf.len() == len {
            Ok(buf)
        } else {
            Err(eof())
        }
    }
}

impl<R: io::Read> io::Read for IoRead<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf)
    }
}
