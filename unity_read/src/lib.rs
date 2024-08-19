//! Allows reading in UnityFS archives, enumerating their files, and objects.
//!
//! Note that some functionality is not generally applicable, e.g. image decoding and meshes are only
//! implemented for a small subset of the functionality required to work with Azur Lane's data.
//!
//! Inspired and made by referencing https://github.com/gameltb/io_unity and https://github.com/yuanyan3060/unity-rs for file formats.

use std::fmt::{Debug, Display};
use std::error::Error;
use std::io::{Read, Seek};

pub mod classes;
pub mod object;
pub mod serialized_file;
mod unity_fs_common_str;
pub mod unity_fs;

#[derive(Debug, Clone)]
pub enum UnityError {
    UnexpectedEof,
    InvalidData(&'static str),
    Mismatch(UnityMismatch),
    Unsupported(&'static str)
}

#[derive(Debug, Clone)]
pub struct UnityMismatch {
    pub expected: String,
    pub received: String,
}

impl Display for UnityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(self, f)
    }
}

impl Error for UnityError {}

/// Extension type to allow specifying the endianness of the read with a bool.
trait BinReadEndian: Sized {
    /// Reads `Self` from the reader, given whether to read as big-endian.
    fn read_endian<R: Read + Seek>(reader: &mut R, is_big_endian: bool) -> binrw::BinResult<Self>;
}

impl<T: binrw::BinRead> BinReadEndian for T
where
    for<'a> T::Args<'a>: Default,
{
    fn read_endian<R: Read + Seek>(reader: &mut R, is_big_endian: bool) -> binrw::BinResult<Self> {
        let endian = match is_big_endian {
            true => binrw::Endian::Big,
            false => binrw::Endian::Little,
        };

        T::read_options(reader, endian, T::Args::default())
    }
}
