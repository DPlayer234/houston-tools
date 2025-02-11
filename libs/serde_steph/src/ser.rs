//! Exposes a serializer and serialization helper methods.

use std::io;

use serde::ser;

use crate::error::{Error, Result};
use crate::leb128;

/// Serializes a value to a [`Vec<u8>`].
///
/// The resulting buffer will have exactly the length required.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>>
where
    T: ser::Serialize,
{
    let mut ser = Serializer::from_writer(Vec::new());
    value.serialize(&mut ser)?;
    Ok(ser.into_writer())
}

/// Serializes a value to a [`io::Write`].
pub fn to_writer<T, W>(writer: W, value: &T) -> Result<()>
where
    T: ser::Serialize,
    W: io::Write,
{
    value.serialize(&mut Serializer::from_writer(writer))
}

/// A [`Serializer`] for this crate's binary format. The trait is only
/// implemented by `&mut`.
///
/// [`Serializer`]: serde::ser::Serializer
#[derive(Debug)]
pub struct Serializer<W> {
    writer: W,
}

impl<W: io::Write> Serializer<W> {
    /// Creates a new deserializer that reads a value from a [`io::Write`].
    pub fn from_writer(writer: W) -> Self {
        Self { writer }
    }

    /// Unwraps the deserializer into its inner writer.
    pub fn into_writer(self) -> W {
        self.writer
    }

    /// Gets a reference to the inner writer.
    pub fn as_writer(&mut self) -> &mut W {
        &mut self.writer
    }

    fn write_byte(&mut self, v: u8) -> Result<()> {
        Ok(self.writer.write_all(&[v])?)
    }

    fn write_leb128(&mut self, v: impl leb128::Leb128) -> Result<()> {
        leb128::write(&mut self.writer, v)
    }
}

// implemented by mut because this avoids adding another layer of indirection
// for every nested Serialize call. most uses will stilly likely end up having
// 2 layers of indirection here (&mut Serializer<&mut Write>) but that's
// basically the minimum we end up with for the by-value case.
impl<'a, W: io::Write> ser::Serializer for &'a mut Serializer<W> {
    type Ok = ();
    type Error = Error;

    // these types correspond to how the value is logically serialized, as noted in
    // the documentation of the crate root
    type SerializeSeq = SerializeList<'a, W>;
    type SerializeTuple = SerializeTuple<'a, W>;
    type SerializeTupleStruct = SerializeTuple<'a, W>;
    type SerializeTupleVariant = SerializeTuple<'a, W>;
    type SerializeMap = SerializeMap<'a, W>;
    type SerializeStruct = SerializeStruct<'a, W>;
    type SerializeStructVariant = SerializeStruct<'a, W>;

    fn serialize_bool(self, v: bool) -> Result<()> {
        self.write_byte(v.into())
    }

    #[allow(clippy::cast_sign_loss)]
    fn serialize_i8(self, v: i8) -> Result<()> {
        self.write_byte(v as u8)
    }

    fn serialize_i16(self, v: i16) -> Result<()> {
        self.write_leb128(v)
    }

    fn serialize_i32(self, v: i32) -> Result<()> {
        self.write_leb128(v)
    }

    fn serialize_i64(self, v: i64) -> Result<()> {
        self.write_leb128(v)
    }

    fn serialize_i128(self, v: i128) -> Result<()> {
        self.write_leb128(v)
    }

    fn serialize_u8(self, v: u8) -> Result<()> {
        self.write_byte(v)
    }

    fn serialize_u16(self, v: u16) -> Result<()> {
        self.write_leb128(v)
    }

    fn serialize_u32(self, v: u32) -> Result<()> {
        self.write_leb128(v)
    }

    fn serialize_u64(self, v: u64) -> Result<()> {
        self.write_leb128(v)
    }

    fn serialize_u128(self, v: u128) -> Result<()> {
        self.write_leb128(v)
    }

    fn serialize_f32(self, v: f32) -> Result<()> {
        Ok(self.writer.write_all(&v.to_le_bytes())?)
    }

    fn serialize_f64(self, v: f64) -> Result<()> {
        Ok(self.writer.write_all(&v.to_le_bytes())?)
    }

    fn serialize_char(self, v: char) -> Result<()> {
        self.serialize_u32(v.into())
    }

    fn serialize_str(self, v: &str) -> Result<()> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(self, v: &[u8]) -> Result<()> {
        self.write_leb128(v.len())?;
        Ok(self.writer.write_all(v)?)
    }

    fn serialize_none(self) -> Result<()> {
        self.write_byte(0)
    }

    fn serialize_some<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.write_byte(1)?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<()> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<()> {
        self.serialize_u32(variant_index)
    }

    fn serialize_newtype_struct<T>(self, _name: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        self.serialize_u32(variant_index)?;
        value.serialize(self)
    }

    fn serialize_seq(self, len: Option<usize>) -> Result<Self::SerializeSeq> {
        let len = len.ok_or(Error::LengthRequired)?;
        self.write_leb128(len)?;
        Ok(SerializeList(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple> {
        Ok(SerializeTuple(self))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct> {
        Ok(SerializeTuple(self))
    }

    fn serialize_tuple_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant> {
        self.serialize_u32(variant_index)?;
        Ok(SerializeTuple(self))
    }

    fn serialize_map(self, len: Option<usize>) -> Result<Self::SerializeMap> {
        let len = len.ok_or(Error::LengthRequired)?;
        self.write_leb128(len)?;
        Ok(SerializeMap(self))
    }

    fn serialize_struct(self, _name: &'static str, len: usize) -> Result<Self::SerializeStruct> {
        self.write_leb128(len)?;
        Ok(SerializeStruct(self))
    }

    fn serialize_struct_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        len: usize,
    ) -> Result<Self::SerializeStructVariant> {
        self.serialize_u32(variant_index)?;
        self.write_leb128(len)?;
        Ok(SerializeStruct(self))
    }

    fn collect_str<T>(self, value: &T) -> Result<()>
    where
        T: ?Sized + std::fmt::Display,
    {
        use io::Write as _;

        // we can't just write to the buffer directly since we need to know the length,
        // and to know the encoded length of the length, we do need its value.
        // there's also no way to shift the data generically later so whatever.
        // aaaanyways:
        // we try to use a small temporary buffer since large strings are uncommon for
        // this. on failure (which should only ever be that the buffer is too small), we
        // instead fall back on `to_string`.
        let mut buf = [0u8; 256];
        let mut rem = buf.as_mut_slice();

        if write!(rem, "{value}").is_ok() {
            // `rem` should only hold the unwritten tail, so slice `buf` to be the written
            // head and pass that down to the serializer. this could only panic if the
            // `impl Write for &mut [u8]` is faulty, which i trust it's not
            let rem_len = rem.len();
            let len = buf.len() - rem_len;
            let buf = &buf[..len];

            // str is serialized the same as bytes
            self.serialize_bytes(buf)
        } else {
            self.serialize_bytes(value.to_string().as_bytes())
        }
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Allows serializing a sequence of elements as a `list`.
#[doc(hidden)]
pub struct SerializeList<'a, W>(&'a mut Serializer<W>);

/// Allows serializing a sequence of elements as a `struct`.
#[doc(hidden)]
pub struct SerializeStruct<'a, W>(&'a mut Serializer<W>);

/// Allows serializing a sequence of elements as a `tuple`.
#[doc(hidden)]
pub struct SerializeTuple<'a, W>(&'a mut Serializer<W>);

/// Allows serializing a sequence of elements as a `map`.
#[doc(hidden)]
pub struct SerializeMap<'a, W>(&'a mut Serializer<W>);

impl<W: io::Write> ser::SerializeSeq for SerializeList<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeTuple for SerializeTuple<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeTupleStruct for SerializeTuple<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeTupleVariant for SerializeTuple<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeMap for SerializeMap<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        key.serialize(&mut *self.0)
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeStruct for SerializeStruct<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeStructVariant for SerializeStruct<'_, W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<()>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(&mut *self.0)
    }

    fn end(self) -> Result<()> {
        Ok(())
    }
}
