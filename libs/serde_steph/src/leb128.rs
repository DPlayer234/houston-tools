//! Generic LEB128 variable-length encoding/decoding.
//!
//! This is used as the serialized format for integers, excluding [`u8`] and
//! [`i8`] which are encoded as just single bytes.
//!
//! See also: <https://en.wikipedia.org/wiki/LEB128>

use std::io;
use std::ops::{BitOr, BitOrAssign, Shl, Shr, ShrAssign};

use crate::de::Read;
use crate::error::Error;

/// Supports the en-/decoding functions.
///
/// Implemented for unsigned integers.
trait Uleb128Encode:
    Sized
    + Default
    + Copy
    + PartialOrd
    + Shr<usize, Output = Self>
    + Shl<usize, Output = Self>
    + ShrAssign<usize>
    + BitOr<Output = Self>
    + BitOrAssign
    + From<u8>
{
    type Buf: AsMut<[u8]> + Default;

    fn trunc_u8(self) -> u8;
}

/// Declares a type as supporting being encoded in LEB128.
///
/// This type essentially specifies a conversion to/from an unsigned type that
/// implements [`Uleb128Encode`]. For unsigned integers, that is a no-op.
pub trait Leb128: Sized + Copy {
    #[allow(private_bounds)]
    type Unsigned: Uleb128Encode;

    fn into_unsigned(self) -> Self::Unsigned;
    fn from_unsigned(value: Self::Unsigned) -> Self;
}

impl<T: Uleb128Encode> Leb128 for T {
    type Unsigned = Self;

    fn into_unsigned(self) -> Self::Unsigned {
        self
    }

    fn from_unsigned(value: Self::Unsigned) -> Self {
        value
    }
}

const fn buf_size<T>() -> usize {
    (bitness::<T>() + 7) / 7
}

const fn bitness<T>() -> usize {
    size_of::<T>() * 8
}

pub fn write<T, W>(writer: W, x: T) -> Result<(), Error>
where
    T: Leb128,
    W: io::Write,
{
    write_inner(writer, x.into_unsigned())
}

fn write_inner<T, W>(mut writer: W, mut x: T) -> Result<(), Error>
where
    T: Uleb128Encode,
    W: io::Write,
{
    let mut buf = T::Buf::default();
    let buf = buf.as_mut();
    let mut i = 0usize;
    while x >= T::from(0x80) {
        buf[i] = x.trunc_u8() | 0x80;
        x >>= 7;
        i += 1;
    }

    buf[i] = x.trunc_u8();
    i += 1;

    Ok(writer.write_all(&buf[..i])?)
}

pub fn read<T, R>(reader: R) -> Result<T, Error>
where
    T: Leb128,
    R: Read,
{
    read_inner(reader).map(T::from_unsigned)
}

fn read_inner<T, R>(mut reader: R) -> Result<T, Error>
where
    T: Uleb128Encode,
    R: Read,
{
    let mut x = T::default();
    let mut s = 0usize;
    loop {
        let [b] = reader.read_bytes()?;
        // ensure the shift isn't greater than the bit-count of `T`
        if s >= bitness::<T>() {
            return Err(Error::IntegerOverflow);
        }

        // convert to shifted `T`
        // ensure that all bits fit into `T`
        let tb = T::from(b & 0x7F);
        let ts = tb << s;
        if ts >> s != tb {
            return Err(Error::IntegerOverflow);
        }

        x |= ts;
        s += 7;

        if b < 0x80 {
            // No continuation bit is set
            return Ok(x);
        }
    }
}

macro_rules! impl_uleb {
    ($($Ty:ty)*) => { $(
        impl Uleb128Encode for $Ty {
            type Buf = [u8; buf_size::<Self>()];

            #[allow(clippy::cast_possible_truncation)]
            fn trunc_u8(self) -> u8 {
                self as u8
            }
        }
    )* };
}

macro_rules! impl_uleb_signed {
    ($($Ty:ty as $Unsigned:ty),* $(,)?) => { $(
        impl Leb128 for $Ty {
            type Unsigned = $Unsigned;

            #[allow(clippy::cast_sign_loss)]
            fn into_unsigned(self) -> Self::Unsigned {
                let mut x = (self as $Unsigned) << 1;
                if self < 0 {
                    x = !x;
                }
                x
            }

            #[allow(clippy::cast_possible_wrap)]
            fn from_unsigned(value: Self::Unsigned) -> Self {
                let mut x = value >> 1;
                if value & 1 != 0 {
                    x = !x;
                }
                x as $Ty
            }
        }
    )* };
}

impl_uleb!(u16 u32 u64 u128 usize);
impl_uleb_signed!(
    i16 as u16,
    i32 as u32,
    i64 as u64,
    i128 as u128,
    isize as usize,
);

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! round_trip {
        ($fn_name:ident, $Ty:ty, $values:expr) => {
            #[test]
            fn $fn_name() {
                const VALUES: &[$Ty] = &$values;
                let mut buf = Vec::new();
                for &v in VALUES {
                    buf.clear();
                    write(&mut buf, v).expect("encoding worked");

                    let r: $Ty = read(buf.as_slice()).expect("decoding worked");
                    assert_eq!(v, r, "must be equal");
                }
            }
        };
    }

    round_trip!(round_trip_usize, usize, [500, 5000, 0, usize::MAX]);
    round_trip!(round_trip_u16, u16, [500, 5000, 0, u16::MAX]);
    round_trip!(
        round_trip_u32,
        u32,
        [500, 5000, 500_000, 500_000_000, 0, u32::MAX]
    );
    round_trip!(
        round_trip_u64,
        u64,
        [500, 500_000_000, 5_000_000_000_000_000_000, 0, u64::MAX]
    );
    round_trip!(
        round_trip_u128,
        u128,
        [
            500,
            5_000_000_000_000_000_000,
            50_000_000_000_000_000_000_000_000_000_000_000_000,
            0,
            u128::MAX
        ]
    );

    round_trip!(
        round_trip_isize,
        isize,
        [500, 5000, -500, -5000, isize::MIN, isize::MAX]
    );
    round_trip!(
        round_trip_i16,
        i16,
        [500, 5000, -500, -5000, i16::MIN, i16::MAX]
    );
    round_trip!(
        round_trip_i32,
        i32,
        [
            500,
            5000,
            500_000,
            500_000_000,
            -500,
            -5000,
            -500_000,
            -500_000_000,
            i32::MIN,
            i32::MAX
        ]
    );
    round_trip!(
        round_trip_i64,
        i64,
        [
            500,
            500_000_000,
            5_000_000_000_000_000_000,
            -500,
            -500_000_000,
            -5_000_000_000_000_000_000,
            i64::MIN,
            i64::MAX
        ]
    );
    round_trip!(
        round_trip_i128,
        i128,
        [
            500,
            5_000_000_000_000_000_000,
            50_000_000_000_000_000_000_000_000_000_000_000_000,
            -500,
            -5_000_000_000_000_000_000,
            -50_000_000_000_000_000_000_000_000_000_000_000_000,
            i128::MIN,
            i128::MAX
        ]
    );
}
