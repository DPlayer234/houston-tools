//! Exposes a deserializer and deserialization helper methods.

use std::io;

use serde::de;

use crate::leb128;

mod error;
mod read;

pub use error::Error;
pub use read::{IoRead, Read};

/// Deserializes a value from a byte slice.
///
/// In addition to other deserialization errors, this returns
/// [`Error::SliceExcessData`] if the slice isn't fully consumed. If you want to
/// use the rest of the slice instead, refer to [`Deserializer::from_slice`].
pub fn from_slice<'de, T>(buf: &'de [u8]) -> Result<T, Error>
where
    T: de::Deserialize<'de>,
{
    let mut de = Deserializer::from_slice(buf);
    let value = T::deserialize(&mut de)?;

    if !de.remainder().is_empty() {
        return Err(Error::SliceExcessData(de.remainder().len()));
    }

    Ok(value)
}

/// Deserializes a value from a [`io::Read`].
///
/// The reader may still have bytes available when this function returns
/// successfully.
pub fn from_reader<T, R>(reader: R) -> Result<T, Error>
where
    T: de::DeserializeOwned,
    R: io::Read,
{
    T::deserialize(&mut Deserializer::from_reader(reader))
}

/// A [`Deserializer`] for this crate's binary format. The trait is only
/// implemented by `&mut`.
///
/// [`Deserializer`]: serde::de::Deserializer
#[derive(Debug)]
pub struct Deserializer<R> {
    reader: R,
}

impl<'de, R: Read<'de>> Deserializer<R> {
    /// Creates a new deserializer that reads a value from a [`Read`].
    ///
    /// When reading from a slice, using [`Self::from_slice`] may be clearer.
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    fn read_leb128<T: leb128::Leb128>(&mut self) -> Result<T, Error> {
        leb128::read(&mut self.reader)
    }
}

impl<'de> Deserializer<&'de [u8]> {
    /// Creates a new deserializer that reads a value from a slice.
    ///
    /// This is useful over [`from_slice`] when you want the remainder of the
    /// slice instead of an error or want to deserialize a sequence of elements
    /// manually.
    ///
    /// # Examples
    ///
    /// Manually deserialize an unprefixed variable-length sequence:
    ///
    /// ```
    /// # use serde_steph::de::{Deserializer, Error};
    /// # use serde::de::Deserialize;
    /// # fn example() -> Result<Vec<u32>, Error> {
    /// # let buf = [1u8, 2, 3, 4, 5];
    /// // buf is some input slice
    /// // out will be used to collect the data
    /// let mut out = Vec::new();
    /// let mut de = Deserializer::from_slice(&buf);
    /// while !de.remainder().is_empty() {
    ///     out.push(u32::deserialize(&mut de)?);
    /// }
    /// # Ok(out)
    /// # }
    /// # assert_eq!(example().expect("must succeed"), vec![1u32, 2, 3, 4, 5]);
    /// ```
    pub fn from_slice(buf: &'de [u8]) -> Self {
        Self::new(buf)
    }

    /// Gets the remaining unread part of the slice.
    pub fn remainder(&self) -> &'de [u8] {
        self.reader
    }
}

impl<R: io::Read> Deserializer<IoRead<R>> {
    /// Creates a new deserializer that reads a value from a [`io::Read`].
    ///
    /// If you're working with a byte slice, it is more efficient to use
    /// [`from_slice`].
    pub fn from_reader(reader: R) -> Self {
        Self::new(IoRead::new(reader))
    }

    /// Unwraps the deserializer into its inner reader.
    pub fn into_reader(self) -> R {
        self.reader.inner
    }
}

impl<R: io::Read> Deserializer<IoRead<R>> {
    /// Gets a reference to the inner reader.
    pub fn as_reader(&mut self) -> &mut R {
        &mut self.reader.inner
    }
}

// implemented by mut because this avoids adding another layer of indirection
// for every nested Deserialize call. most uses will stilly likely end up having
// 2 layers of indirection here (&mut Deserializer<&mut Write>) but that's
// basically the minimum we end up with for the by-value case.
impl<'de, R: Read<'de>> de::Deserializer<'de> for &mut Deserializer<R> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AnyUnsupported)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let [b] = self.reader.read_bytes()?;
        let v = match b {
            0 => false,
            1 => true,
            _ => return Err(Error::InvalidBool),
        };

        visitor.visit_bool(v)
    }

    #[allow(clippy::cast_possible_wrap)]
    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let [b] = self.reader.read_bytes()?;
        visitor.visit_i8(b as i8)
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i16(self.read_leb128()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i32(self.read_leb128()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i64(self.read_leb128()?)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i128(self.read_leb128()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let [b] = self.reader.read_bytes()?;
        visitor.visit_u8(b)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u16(self.read_leb128()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u32(self.read_leb128()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u64(self.read_leb128()?)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u128(self.read_leb128()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let bytes = self.reader.read_bytes()?;
        visitor.visit_f32(f32::from_le_bytes(bytes))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let bytes = self.reader.read_bytes()?;
        visitor.visit_f64(f64::from_le_bytes(bytes))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let code: u32 = self.read_leb128()?;
        let v = char::try_from(code).map_err(|_| Error::InvalidChar)?;
        visitor.visit_char(v)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = self.read_leb128()?;
        match self.reader.try_read_bytes_borrow(len) {
            Some(v) => {
                let v = std::str::from_utf8(v?).map_err(|_| Error::InvalidUtf8)?;
                visitor.visit_borrowed_str(v)
            },
            None => self.reader.read_byte_view(len, |v| {
                let v = std::str::from_utf8(v).map_err(|_| Error::InvalidUtf8)?;
                visitor.visit_str(v)
            }),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = self.read_leb128()?;
        let v = self.reader.read_byte_vec(len)?;
        let v = String::from_utf8(v).map_err(|_| Error::InvalidUtf8)?;
        visitor.visit_string(v)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = self.read_leb128()?;
        match self.reader.try_read_bytes_borrow(len) {
            Some(v) => visitor.visit_borrowed_bytes(v?),
            None => self.reader.read_byte_view(len, |v| visitor.visit_bytes(v)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = self.read_leb128()?;
        let v = self.reader.read_byte_vec(len)?;
        visitor.visit_byte_buf(v)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let [b] = self.reader.read_bytes()?;
        match b {
            0 => visitor.visit_none(),
            1 => visitor.visit_some(self),
            _ => Err(Error::InvalidOption),
        }
    }

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(
        self,
        _name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = self.read_leb128()?;
        visitor.visit_seq(ListAccess {
            deserializer: self,
            len,
        })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(ListAccess {
            deserializer: self,
            len,
        })
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(ListAccess {
            deserializer: self,
            len,
        })
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = self.read_leb128()?;
        visitor.visit_map(ListAccess {
            deserializer: self,
            len,
        })
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(TupleAccess { deserializer: self })
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(TupleAccess { deserializer: self })
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_u32(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Provides access to a sequence with length prefix.
struct ListAccess<'a, R> {
    deserializer: &'a mut Deserializer<R>,
    len: usize,
}

/// Provides access to a sequence with well-known length.
struct TupleAccess<'a, R> {
    deserializer: &'a mut Deserializer<R>,
}

impl<'de, R: Read<'de>> de::SeqAccess<'de> for ListAccess<'_, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if self.len == 0 {
            Ok(None)
        } else {
            self.len -= 1;
            Ok(Some(seed.deserialize(&mut *self.deserializer)?))
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de, R: Read<'de>> de::MapAccess<'de> for ListAccess<'_, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if self.len == 0 {
            Ok(None)
        } else {
            self.len -= 1;
            Ok(Some(seed.deserialize(&mut *self.deserializer)?))
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserializer)
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.len)
    }
}

impl<'de, R: Read<'de>> de::SeqAccess<'de> for TupleAccess<'_, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        Ok(Some(seed.deserialize(&mut *self.deserializer)?))
    }
}

impl<'de, R: Read<'de>> de::EnumAccess<'de> for TupleAccess<'_, R> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let v = seed.deserialize(&mut *self.deserializer)?;
        Ok((v, self))
    }
}

impl<'de, R: Read<'de>> de::VariantAccess<'de> for TupleAccess<'_, R> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserializer)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(ListAccess {
            deserializer: self.deserializer,
            len,
        })
    }

    fn struct_variant<V>(
        self,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(self)
    }
}
