//! Exposes a deserializer and deserialization helper methods.

use std::io;

use serde::de;

use crate::error::Error;
use crate::leb128;

/// Deserializes a value from a byte slice.
///
/// Excess bytes in the slice will be ignored. If you need to handle the
/// remaining bytes, use [`Deserializer`]'s `from_slice` and `remainder`.
pub fn from_slice<T>(buf: &[u8]) -> Result<T, Error>
where
    T: de::DeserializeOwned,
{
    T::deserialize(&mut Deserializer::from_slice(buf))
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

/// A deserializer for this crate's binary format. The [`Deserializer`] trait is
/// only implemented by mutable reference.
///
/// [`Deserializer`]: serde::de::Deserializer
#[derive(Debug)]
pub struct Deserializer<R> {
    reader: R,
}

impl<R: Read> Deserializer<R> {
    /// Creates a new deserializer that reads a value from a [`Read`].
    ///
    /// When reading from a slice, using [`Self::from_slice`] may be clearer.
    pub fn new(reader: R) -> Self {
        Self { reader }
    }
}

impl<'a> Deserializer<&'a [u8]> {
    /// Creates a new deserializer that reads a value from a slice.
    pub fn from_slice(buf: &'a [u8]) -> Self {
        Self::new(buf)
    }

    /// Gets the remaining unread part of the slice.
    pub fn remainder(&self) -> &'a [u8] {
        self.reader
    }
}

impl<R: io::Read> Deserializer<IoRead<R>> {
    /// Creates a new deserializer that reads a value from a [`io::Read`].
    ///
    /// If you're working with a byte slice, it is more efficient to use
    /// [`from_slice`].
    pub fn from_reader(reader: R) -> Self {
        Self {
            reader: IoRead::new(reader),
        }
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

/// Specialized reader trait for use with [`Deserializer`].
///
/// By default, this is implemented for `&[u8]` (byte slices), [`IoRead`] and
/// mutable references to [`Read`] implementations.
pub trait Read {
    /// Reads a constant size chunk of bytes.
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Error>;

    /// Reads a chunk of bytes, possibly borrowed from the reader for the
    /// duration of the call.
    fn read_byte_view<F, T>(&mut self, len: usize, access: F) -> Result<T, Error>
    where
        F: FnOnce(&[u8]) -> Result<T, Error>;

    /// Reads a chunk of bytes, returning it as a newly allocated [`Vec`].
    fn read_byte_vec(&mut self, len: usize) -> Result<Vec<u8>, Error>;
}

impl<R: Read> Read for &mut R {
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        (**self).read_bytes()
    }

    fn read_byte_view<F, T>(&mut self, len: usize, access: F) -> Result<T, Error>
    where
        F: FnOnce(&[u8]) -> Result<T, Error>,
    {
        (**self).read_byte_view(len, access)
    }

    fn read_byte_vec(&mut self, len: usize) -> Result<Vec<u8>, Error> {
        (**self).read_byte_vec(len)
    }
}

impl Read for &[u8] {
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let (out, rem) = self.split_first_chunk::<N>().ok_or(Error::UnexpectedEof)?;
        *self = rem;
        Ok(*out)
    }

    fn read_byte_view<F, T>(&mut self, len: usize, access: F) -> Result<T, Error>
    where
        F: FnOnce(&[u8]) -> Result<T, Error>,
    {
        let (out, rem) = self.split_at_checked(len).ok_or(Error::UnexpectedEof)?;
        *self = rem;
        access(out)
    }

    fn read_byte_vec(&mut self, len: usize) -> Result<Vec<u8>, Error> {
        let (out, rem) = self.split_at_checked(len).ok_or(Error::UnexpectedEof)?;
        *self = rem;
        Ok(out.to_vec())
    }
}

/// Wraps a [`io::Read`] implementation so it can be used as a [`Read`].
///
/// You cannot directly construct this type, instead use
/// [`Deserializer::from_reader`].
#[derive(Debug)]
pub struct IoRead<R> {
    inner: R,
}

impl<R> IoRead<R> {
    pub(crate) fn new(inner: R) -> Self {
        Self { inner }
    }
}

impl<R: io::Read> Read for IoRead<R> {
    fn read_bytes<const N: usize>(&mut self) -> Result<[u8; N], Error> {
        let mut buf = [0u8; N];
        self.inner.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_byte_view<F, T>(&mut self, len: usize, access: F) -> Result<T, Error>
    where
        F: FnOnce(&[u8]) -> Result<T, Error>,
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
    fn read_byte_vec(&mut self, len: usize) -> Result<Vec<u8>, Error> {
        use std::io::Read;

        // don't allocate too much or incorrect data could lead to a DoS
        let capacity = len.min(0x1000);
        let mut buf = Vec::with_capacity(capacity);
        let limit = u64::try_from(len).map_err(|_| Error::UnexpectedEof)?;
        (&mut self.inner).take(limit).read_to_end(&mut buf)?;

        if buf.len() >= len {
            Ok(buf)
        } else {
            Err(Error::UnexpectedEof)
        }
    }
}

impl<'de, R: Read> de::Deserializer<'de> for &mut Deserializer<R> {
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
        visitor.visit_i16(leb128::read(&mut self.reader)?)
    }

    fn deserialize_i32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i32(leb128::read(&mut self.reader)?)
    }

    fn deserialize_i64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i64(leb128::read(&mut self.reader)?)
    }

    fn deserialize_i128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_i128(leb128::read(&mut self.reader)?)
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
        visitor.visit_u16(leb128::read(&mut self.reader)?)
    }

    fn deserialize_u32<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u32(leb128::read(&mut self.reader)?)
    }

    fn deserialize_u64<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u64(leb128::read(&mut self.reader)?)
    }

    fn deserialize_u128<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_u128(leb128::read(&mut self.reader)?)
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
        let code: u32 = leb128::read(&mut self.reader)?;
        let v = char::try_from(code).map_err(|_| Error::InvalidChar)?;
        visitor.visit_char(v)
    }

    fn deserialize_str<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = leb128::read(&mut self.reader)?;
        self.reader.read_byte_view(len, |v| {
            let v = std::str::from_utf8(v).map_err(|_| Error::InvalidUtf8)?;
            visitor.visit_str(v)
        })
    }

    fn deserialize_string<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = leb128::read(&mut self.reader)?;
        let v = self.reader.read_byte_vec(len)?;
        let v = String::from_utf8(v).map_err(|_| Error::InvalidUtf8)?;
        visitor.visit_string(v)
    }

    fn deserialize_bytes<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = leb128::read(&mut self.reader)?;
        self.reader.read_byte_view(len, |v| visitor.visit_bytes(v))
    }

    fn deserialize_byte_buf<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = leb128::read(&mut self.reader)?;
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
            _ => Err(Error::InvalidBool),
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
        let len: usize = leb128::read(&mut self.reader)?;
        visitor.visit_seq(SeqAccess {
            deserializer: self,
            len,
        })
    }

    fn deserialize_tuple<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(SeqAccess {
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
        visitor.visit_seq(SeqAccess {
            deserializer: self,
            len,
        })
    }

    fn deserialize_map<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let len: usize = leb128::read(&mut self.reader)?;
        visitor.visit_map(SeqAccess {
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
        visitor.visit_seq(self)
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
        visitor.visit_enum(self)
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

struct SeqAccess<'a, R> {
    deserializer: &'a mut Deserializer<R>,
    len: usize,
}

impl<'de, R: Read> de::SeqAccess<'de> for SeqAccess<'_, R> {
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

impl<'de, R: Read> de::MapAccess<'de> for SeqAccess<'_, R> {
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

impl<'de, R: Read> de::SeqAccess<'de> for &mut Deserializer<R> {
    type Error = Error;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        Ok(Some(seed.deserialize(&mut **self)?))
    }
}

impl<'de, R: Read> de::EnumAccess<'de> for &mut Deserializer<R> {
    type Error = Error;
    type Variant = Self;

    fn variant_seed<V>(self, seed: V) -> Result<(V::Value, Self::Variant), Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let v = seed.deserialize(&mut *self)?;
        Ok((v, self))
    }
}

impl<'de, R: Read> de::VariantAccess<'de> for &mut Deserializer<R> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Self::Error> {
        Ok(())
    }

    fn newtype_variant_seed<T>(self, seed: T) -> Result<T::Value, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        seed.deserialize(self)
    }

    fn tuple_variant<V>(self, len: usize, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(SeqAccess {
            deserializer: self,
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
