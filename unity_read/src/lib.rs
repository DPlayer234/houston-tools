//! Allows reading in UnityFS archives, enumerating their files, and objects.
//!
//! Note that some functionality is not generally applicable, e.g. image decoding and meshes are only
//! implemented for a small subset of the functionality required to work with Azur Lane's data.
//!
//! Inspired and made by referencing <https://github.com/gameltb/io_unity> and <https://github.com/yuanyan3060/unity-rs> for file formats.
#![allow(clippy::upper_case_acronyms)]

use std::any::type_name;
use std::fmt;
use std::io::{self, Read, Seek};

pub mod classes;
pub mod error;
pub mod object;
pub mod serialized_file;
pub mod unity_fs;
mod unity_fs_common_str;

/// Result type with [`Error`](error::Error) error variant.
pub type Result<T> = std::result::Result<T, error::Error>;

/// Trait combining [`Read`] and [`Seek`] with read-alignment support.
///
/// Blanket-implemented for any type that implements both [`Read`] and [`Seek`].
pub trait SeekRead: Read + Seek {
    #[inline]
    fn align_to(&mut self, align: u16) -> io::Result<()> {
        let pos = self.stream_position()?;
        let offset = pos % u64::from(align);

        if offset != 0 {
            // offset is within (0..=u16::MAX) and thus cannot wrap
            #[allow(clippy::cast_possible_wrap)]
            self.seek(io::SeekFrom::Current(i64::from(align) - offset as i64))?;
        }

        Ok(())
    }
}

impl<T: Read + Seek> SeekRead for T {}

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

/// Internal int-to-int conversion.
trait FromInt<T>: Sized {
    fn from_int(value: T) -> Result<Self>;
}

impl<T, U> FromInt<T> for U
where
    T: Copy + fmt::Display,
    U: TryFrom<T, Error = std::num::TryFromIntError>,
{
    /// Casts from `T` to `U`. This cast is not expected to fail.
    fn from_int(value: T) -> Result<Self> {
        U::try_from(value).map_err(|_| error::Error::Unsupported(format!(
            "cast from value {} of type {} to {} failed",
            value, type_name::<T>(), type_name::<U>(),
        )))
    }
}
