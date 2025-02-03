//! Exposes a serializer and serialization helper methods.

use std::io;

use serde::ser;

use crate::leb128;

mod error;

pub use error::Error;

/// Serializes a value to a [`Vec<u8>`].
///
/// The resulting buffer will have exactly the length required.
pub fn to_vec<T>(value: &T) -> Result<Vec<u8>, Error>
where
    T: ser::Serialize,
{
    let mut buf = Vec::new();
    to_writer(&mut buf, value)?;
    Ok(buf)
}

/// Serializes a value to a [`io::Write`].
pub fn to_writer<T, W>(writer: W, value: &T) -> Result<(), Error>
where
    T: ser::Serialize,
    W: io::Write,
{
    value.serialize(Serializer::from_writer(writer))
}

/// A [`Serializer`] for this crate's binary format.
///
/// [`Serializer`]: serde::ser::Serializer
#[derive(Debug)]
pub struct Serializer<W> {
    writer: W,
}

impl<W> Serializer<W> {
    /// Reborrows the serializer so it can be used for multiple
    /// [`serialize`](ser::Serialize::serialize) calls.
    ///
    /// This could be useful for manually serializing a sequence of elements.
    pub fn reborrow(&mut self) -> Serializer<&mut W> {
        Serializer {
            writer: &mut self.writer,
        }
    }
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

    fn write_byte(mut self, v: u8) -> Result<(), Error> {
        Ok(self.writer.write_all(&[v])?)
    }
}

impl<W: io::Write> ser::Serializer for Serializer<W> {
    type Ok = ();
    type Error = Error;

    // these types correspond to how the value is logically serialized, as noted in
    // the documentation of the crate root
    type SerializeSeq = SerializeList<W>;
    type SerializeTuple = SerializeTuple<W>;
    type SerializeTupleStruct = SerializeTuple<W>;
    type SerializeTupleVariant = SerializeTuple<W>;
    type SerializeMap = SerializeMap<W>;
    type SerializeStruct = SerializeTuple<W>;
    type SerializeStructVariant = SerializeTuple<W>;

    fn serialize_bool(self, v: bool) -> Result<Self::Ok, Self::Error> {
        self.write_byte(v.into())
    }

    #[allow(clippy::cast_sign_loss)]
    fn serialize_i8(self, v: i8) -> Result<Self::Ok, Self::Error> {
        self.write_byte(v as u8)
    }

    fn serialize_i16(self, v: i16) -> Result<Self::Ok, Self::Error> {
        leb128::write(self.writer, v)
    }

    fn serialize_i32(self, v: i32) -> Result<Self::Ok, Self::Error> {
        leb128::write(self.writer, v)
    }

    fn serialize_i64(self, v: i64) -> Result<Self::Ok, Self::Error> {
        leb128::write(self.writer, v)
    }

    fn serialize_i128(self, v: i128) -> Result<Self::Ok, Self::Error> {
        leb128::write(self.writer, v)
    }

    fn serialize_u8(self, v: u8) -> Result<Self::Ok, Self::Error> {
        self.write_byte(v)
    }

    fn serialize_u16(self, v: u16) -> Result<Self::Ok, Self::Error> {
        leb128::write(self.writer, v)
    }

    fn serialize_u32(self, v: u32) -> Result<Self::Ok, Self::Error> {
        leb128::write(self.writer, v)
    }

    fn serialize_u64(self, v: u64) -> Result<Self::Ok, Self::Error> {
        leb128::write(self.writer, v)
    }

    fn serialize_u128(self, v: u128) -> Result<Self::Ok, Self::Error> {
        leb128::write(self.writer, v)
    }

    fn serialize_f32(mut self, v: f32) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer.write_all(&v.to_le_bytes())?)
    }

    fn serialize_f64(mut self, v: f64) -> Result<Self::Ok, Self::Error> {
        Ok(self.writer.write_all(&v.to_le_bytes())?)
    }

    fn serialize_char(self, v: char) -> Result<Self::Ok, Self::Error> {
        self.serialize_u32(v.into())
    }

    fn serialize_str(self, v: &str) -> Result<Self::Ok, Self::Error> {
        self.serialize_bytes(v.as_bytes())
    }

    fn serialize_bytes(mut self, v: &[u8]) -> Result<Self::Ok, Self::Error> {
        leb128::write(&mut self.writer, v.len())?;
        Ok(self.writer.write_all(v)?)
    }

    fn serialize_none(self) -> Result<Self::Ok, Self::Error> {
        self.write_byte(0)
    }

    fn serialize_some<T>(mut self, value: &T) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.reborrow().write_byte(1)?;
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_struct(self, _name: &'static str) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }

    fn serialize_unit_variant(
        self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
    ) -> Result<Self::Ok, Self::Error> {
        self.serialize_u32(variant_index)
    }

    fn serialize_newtype_struct<T>(
        self,
        _name: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self)
    }

    fn serialize_newtype_variant<T>(
        mut self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        value: &T,
    ) -> Result<Self::Ok, Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        self.reborrow().serialize_u32(variant_index)?;
        value.serialize(self)
    }

    fn serialize_seq(mut self, len: Option<usize>) -> Result<Self::SerializeSeq, Self::Error> {
        let len = len.ok_or(Error::LengthRequired)?;
        leb128::write(&mut self.writer, len)?;
        Ok(SerializeList(self))
    }

    fn serialize_tuple(self, _len: usize) -> Result<Self::SerializeTuple, Self::Error> {
        Ok(SerializeTuple(self))
    }

    fn serialize_tuple_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleStruct, Self::Error> {
        Ok(SerializeTuple(self))
    }

    fn serialize_tuple_variant(
        mut self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeTupleVariant, Self::Error> {
        self.reborrow().serialize_u32(variant_index)?;
        Ok(SerializeTuple(self))
    }

    fn serialize_map(mut self, len: Option<usize>) -> Result<Self::SerializeMap, Self::Error> {
        let len = len.ok_or(Error::LengthRequired)?;
        leb128::write(&mut self.writer, len)?;
        Ok(SerializeMap(self))
    }

    fn serialize_struct(
        self,
        _name: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStruct, Self::Error> {
        Ok(SerializeTuple(self))
    }

    fn serialize_struct_variant(
        mut self,
        _name: &'static str,
        variant_index: u32,
        _variant: &'static str,
        _len: usize,
    ) -> Result<Self::SerializeStructVariant, Self::Error> {
        self.reborrow().serialize_u32(variant_index)?;
        Ok(SerializeTuple(self))
    }

    fn is_human_readable(&self) -> bool {
        false
    }
}

/// Allows serializing a sequence of elements as a `list`.
///
/// You shouldn't use this type directly. It is returned by [`Serializer`] as
/// needed.
pub struct SerializeList<W>(Serializer<W>);

/// Allows serializing a sequence of elements as a `tuple`.
///
/// You shouldn't use this type directly. It is returned by [`Serializer`] as
/// needed.
pub struct SerializeTuple<W>(Serializer<W>);

/// Allows serializing a sequence of elements as a `map`.
///
/// You shouldn't use this type directly. It is returned by [`Serializer`] as
/// needed.
pub struct SerializeMap<W>(Serializer<W>);

impl<W: io::Write> ser::SerializeSeq for SerializeList<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self.0.reborrow())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeTuple for SerializeTuple<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_element<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self.0.reborrow())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeTupleStruct for SerializeTuple<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self.0.reborrow())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeTupleVariant for SerializeTuple<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self.0.reborrow())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeMap for SerializeMap<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_key<T>(&mut self, key: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        key.serialize(self.0.reborrow())
    }

    fn serialize_value<T>(&mut self, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self.0.reborrow())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeStruct for SerializeTuple<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self.0.reborrow())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}

impl<W: io::Write> ser::SerializeStructVariant for SerializeTuple<W> {
    type Ok = ();
    type Error = Error;

    fn serialize_field<T>(&mut self, _key: &'static str, value: &T) -> Result<(), Self::Error>
    where
        T: ?Sized + ser::Serialize,
    {
        value.serialize(self.0.reborrow())
    }

    fn end(self) -> Result<Self::Ok, Self::Error> {
        Ok(())
    }
}
