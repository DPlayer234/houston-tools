//! Exposes a deserializer and deserialization helper methods.

use std::io;

use serde_core::de::{self, Deserializer as _};

use crate::error::{Error, Result};
use crate::leb128;
pub use crate::read::{IoRead, Read, SliceRead};

/// Deserializes a value from a byte slice.
///
/// This assumes that `buf` contains just one object. If there are trailing
/// bytes, this function returns [`Error::TrailingBytes`]. If you want to handle
/// trailing bytes yourself, use [`Deserializer::from_slice`] and
/// [`Deserializer::remainder`].
///
/// # Errors
///
/// This function may fail and return [`Err`] when the bytes are invalid for the
/// STEPH format or trailing bytes are found after the value. It may also fail
/// due to the [`Deserialize`] implementation for `T` attempting unsupported
/// operations or the data being invalid for `T`.
///
/// [`Deserialize`]: serde_core::Deserialize
pub fn from_slice<'de, T>(buf: &'de [u8]) -> Result<T>
where
    T: de::Deserialize<'de>,
{
    Deserializer::from_slice(buf).read_to_end()
}

/// Deserializes a value from a [`io::Read`].
///
/// This assumes that `reader` yields only one object. If it provides trailing
/// bytes, this function returns [`Error::TrailingBytes`]. If you want to handle
/// trailing bytes yourself, use [`Deserializer::from_reader`].
///
/// If you use multiple reader types, consider passing them as [`&mut dyn
/// io::Read`](io::Read) to reduce the generated code size.
///
/// # Errors
///
/// This function may fail and return [`Err`] when an I/O error occurs, the read
/// bytes are invalid for the STEPH format or trailing bytes are found after the
/// value. It may also fail due to the [`Deserialize`] implementation for `T`
/// attempting unsupported operations or the data being invalid for `T`.
///
/// [`Deserialize`]: serde_core::Deserialize
pub fn from_reader<T, R>(reader: R) -> Result<T>
where
    T: de::DeserializeOwned,
    R: io::Read,
{
    Deserializer::from_reader(reader).read_to_end()
}

/// A [`Deserializer`] for this crate's binary format. The trait is only
/// implemented by `&mut`.
///
/// After deserializing an object, the underlying reader will be left just after
/// the end of the data. The same deserializer may then be reused to read the
/// next object, discarded to use the reader otherwise, or you may call
/// [`Deserializer::end`] to explicitly expect the end of the data.
///
/// [`Deserializer`]: serde_core::de::Deserializer
#[derive(Debug)]
pub struct Deserializer<R> {
    reader: R,
}

impl<'de, R: Read<'de>> Deserializer<R> {
    /// Creates a new deserializer that reads a value from a [`Read<'de>`].
    ///
    /// You are most likely looking for [`Self::from_slice`] or
    /// [`Self::from_reader`] instead, or perhaps one of the standalone
    /// functions in this module are sufficient.
    pub fn new(reader: R) -> Self {
        Self { reader }
    }

    /// Should be called to indicate that the object has been fully
    /// deserialized. Returns an error if there are more bytes left.
    ///
    /// This will try to read from the underlying reader. If it succeeds to get
    /// more bytes, returns [`Error::TrailingBytes`]. It also propagates I/O
    /// errors that happen during this attempt.
    ///
    /// This behavior also means that the corresponding reader should be
    /// considered exhausted after this point. If you intend to use the reader
    /// after this, you probably shouldn't call this.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if underlying reader wasn't exhausted or an I/O occured.
    pub fn end(&mut self) -> Result<()> {
        if self.reader.read(&mut [0u8])? == 0 {
            Ok(())
        } else {
            Err(Error::TrailingBytes)
        }
    }

    /// Deserializes a single object, followed by a call to [`Self::end`].
    ///
    /// This is a shared function used to implement [`from_slice`] and
    /// [`from_reader`].
    pub(crate) fn read_to_end<T: de::Deserialize<'de>>(&mut self) -> Result<T> {
        let value = T::deserialize(&mut *self)?;
        self.end()?;
        Ok(value)
    }

    /// Reads an LEB128 encoded integer.
    fn read_leb128<T: leb128::Leb128>(&mut self) -> Result<T> {
        leb128::read(&mut self.reader)
    }
}

impl<'de> Deserializer<SliceRead<'de>> {
    /// Creates a new deserializer that reads a value from a slice.
    ///
    /// This is gives more control than [`from_slice`], insofar that it allows
    /// manual handling of the remainder or continuing deserialization after the
    /// last object.
    ///
    /// # Examples
    ///
    /// Manually deserialize an unprefixed variable-length sequence:
    ///
    /// ```
    /// # use serde_steph::{Deserializer, Error};
    /// # use serde_core::de::Deserialize;
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
        Self::new(SliceRead::new(buf))
    }

    /// Gets the remaining unread part of the slice.
    pub fn remainder(&self) -> &'de [u8] {
        self.reader.slice
    }
}

impl<R: io::Read> Deserializer<IoRead<R>> {
    /// Creates a new deserializer that reads a value from a [`io::Read`].
    ///
    /// If you're working with a byte slice, it is more efficient to use
    /// [`Self::from_slice`].
    ///
    /// This is gives more control than [`from_reader`], insofar that it allows
    /// manual handling of the remainder or continuing deserialization after the
    /// last object.
    ///
    /// If you use multiple reader types, consider passing them as [`&mut dyn
    /// io::Read`](io::Read) to reduce the generated code size.
    ///
    /// # Examples
    ///
    /// Manually deserialize an unprefixed variable-length sequence:
    ///
    /// ```
    /// # use serde_steph::{Deserializer, Error};
    /// # use serde_core::de::Deserialize;
    /// # fn example() -> Vec<u32> {
    /// # let buf = [1u8, 2, 3, 4, 5];
    /// # let mut reader = buf.as_slice();
    /// // out will be used to collect the data
    /// // this terminates on any error for simplicity
    /// let mut out = Vec::new();
    /// let mut de = Deserializer::from_reader(&mut reader);
    /// while let Ok(v) = u32::deserialize(&mut de) {
    ///     out.push(v);
    /// }
    /// # out
    /// # }
    /// # assert_eq!(example(), vec![1u32, 2, 3, 4, 5]);
    /// ```
    pub fn from_reader(reader: R) -> Self {
        Self::new(IoRead::new(reader))
    }

    /// Unwraps the deserializer into its inner reader.
    pub fn into_reader(self) -> R {
        self.reader.inner
    }

    /// Gets a reference to the inner reader.
    pub fn as_reader(&mut self) -> &mut R {
        &mut self.reader.inner
    }
}

// implemented by mut because this avoids adding another layer of indirection
// for every nested Deserialize call. most uses will stilly likely end up having
// 2 layers of indirection here (&mut Deserializer<&mut Read>) but that's
// basically the minimum we end up with for the by-value case.
//
// note for the implementations for structured data:
// - `deserialize_seq` is used for length-prefixed data (that is `list` and
//   `struct` types) and is implemented in terms of `deserialize_tuple`
// - `deserialize_tuple` is used for data without one, that is... `tuple` types
// - `deserialize_map` is similar to `deserialize_seq`, just with `visit_map`
impl<'de, R: Read<'de>> de::Deserializer<'de> for &mut Deserializer<R> {
    type Error = Error;

    fn deserialize_any<V>(self, _visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        Err(Error::AnyUnsupported)
    }

    fn deserialize_bool<V>(self, visitor: V) -> Result<V::Value>
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

    fn deserialize_i8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let [b] = self.reader.read_bytes()?;
        visitor.visit_i8(b.cast_signed())
    }

    fn deserialize_i16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i16(self.read_leb128()?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i32(self.read_leb128()?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i64(self.read_leb128()?)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i128(self.read_leb128()?)
    }

    fn deserialize_u8<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let [b] = self.reader.read_bytes()?;
        visitor.visit_u8(b)
    }

    fn deserialize_u16<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u16(self.read_leb128()?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u32(self.read_leb128()?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u64(self.read_leb128()?)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u128(self.read_leb128()?)
    }

    fn deserialize_f32<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let bytes = self.reader.read_bytes()?;
        visitor.visit_f32(f32::from_le_bytes(bytes))
    }

    fn deserialize_f64<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let bytes = self.reader.read_bytes()?;
        visitor.visit_f64(f64::from_le_bytes(bytes))
    }

    fn deserialize_char<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let code: u32 = self.read_leb128()?;
        let v = char::try_from(code).map_err(|_| Error::InvalidChar)?;
        visitor.visit_char(v)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = self.read_leb128()?;
        match self.reader.try_read_bytes_borrow(len) {
            Some(v) => {
                let v = str::from_utf8(v?).map_err(|_| Error::InvalidUtf8)?;
                visitor.visit_borrowed_str(v)
            },
            None => self.reader.read_byte_view(len, |v| {
                let v = str::from_utf8(v).map_err(|_| Error::InvalidUtf8)?;
                visitor.visit_str(v)
            }),
        }
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = self.read_leb128()?;
        let v = self.reader.read_byte_vec(len)?;
        let v = String::from_utf8(v).map_err(|_| Error::InvalidUtf8)?;
        visitor.visit_string(v)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = self.read_leb128()?;
        match self.reader.try_read_bytes_borrow(len) {
            Some(v) => visitor.visit_borrowed_bytes(v?),
            None => self.reader.read_byte_view(len, |v| visitor.visit_bytes(v)),
        }
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = self.read_leb128()?;
        let v = self.reader.read_byte_vec(len)?;
        visitor.visit_byte_buf(v)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value>
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

    fn deserialize_unit<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_unit_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_unit()
    }

    fn deserialize_newtype_struct<V>(self, _name: &'static str, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_newtype_struct(self)
    }

    fn deserialize_seq<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = self.read_leb128()?;
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_tuple<V>(self, mut len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let value = visitor.visit_seq(self.list_access(&mut len))?;
        ensure_remainder_zero(len)?;
        Ok(value)
    }

    fn deserialize_tuple_struct<V>(
        self,
        _name: &'static str,
        len: usize,
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_tuple(len, visitor)
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        let mut len: usize = self.read_leb128()?;
        let value = visitor.visit_map(self.list_access(&mut len))?;
        ensure_remainder_zero(len)?;
        Ok(value)
    }

    fn deserialize_struct<V>(
        self,
        _name: &'static str,
        _fields: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_seq(visitor)
    }

    fn deserialize_enum<V>(
        self,
        _name: &'static str,
        _variants: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_enum(self.enum_access())
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_u32(visitor)
    }

    fn deserialize_ignored_any<V>(self, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserialize_any(visitor)
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

impl<'de, R: Read<'de>> Deserializer<R> {
    /// Gets a matching [`ListAccess`].
    fn list_access<'a>(&'a mut self, len: &'a mut usize) -> ListAccess<'a, R> {
        ListAccess {
            deserializer: self,
            len,
        }
    }

    /// Gets a matching [`EnumAccess`].
    fn enum_access(&mut self) -> EnumAccess<'_, R> {
        EnumAccess { deserializer: self }
    }
}

fn ensure_remainder_zero(len: usize) -> Result<()> {
    if len == 0 {
        Ok(())
    } else {
        Err(Error::ShortSeqRead)
    }
}

/// Provides access to a sequence with length prefix.
struct ListAccess<'a, R> {
    deserializer: &'a mut Deserializer<R>,
    // don't blindly use this for size hints. this may come from the deserialized data,
    // and if used as-is as a size hint, could trigger a massive allocation.
    // that doesn't even need "malicious" input, it could just be wrong.
    len: &'a mut usize,
}

/// Provides access to an enum.
struct EnumAccess<'a, R> {
    deserializer: &'a mut Deserializer<R>,
}

impl<R> ListAccess<'_, R> {
    fn cautious_size_hint(&self) -> Option<usize> {
        // we probably won't encounter absurdly large types, so this is defensive
        // enough... probably. hopefully.
        Some((*self.len).min(1024))
    }
}

impl<'de, R: Read<'de>> de::SeqAccess<'de> for ListAccess<'_, R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>>
    where
        T: de::DeserializeSeed<'de>,
    {
        if *self.len == 0 {
            Ok(None)
        } else {
            *self.len -= 1;
            Ok(Some(seed.deserialize(&mut *self.deserializer)?))
        }
    }

    fn size_hint(&self) -> Option<usize> {
        self.cautious_size_hint()
    }
}

impl<'de, R: Read<'de>> de::MapAccess<'de> for ListAccess<'_, R> {
    type Error = Error;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>>
    where
        K: de::DeserializeSeed<'de>,
    {
        if *self.len == 0 {
            Ok(None)
        } else {
            *self.len -= 1;
            Ok(Some(seed.deserialize(&mut *self.deserializer)?))
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value>
    where
        V: de::DeserializeSeed<'de>,
    {
        seed.deserialize(&mut *self.deserializer)
    }

    fn size_hint(&self) -> Option<usize> {
        self.cautious_size_hint()
    }
}

impl<'de, R: Read<'de>> de::EnumAccess<'de> for EnumAccess<'_, R> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant)>
    where
        V: de::DeserializeSeed<'de>,
    {
        let v = seed.deserialize(&mut *self.deserializer)?;
        Ok((v, self))
    }
}

impl<'de, R: Read<'de>> de::VariantAccess<'de> for EnumAccess<'_, R> {
    type Error = Error;

    fn unit_variant(self) -> Result<()> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self.deserializer)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer.deserialize_tuple(len, visitor)
    }

    fn struct_variant<V>(self, _fields: &'static [&'static str], visitor: V) -> Result<V::Value>
    where
        V: de::Visitor<'de>,
    {
        self.deserializer.deserialize_seq(visitor)
    }
}
