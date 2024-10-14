//! Provides access to UnityFS object information.

use num_enum::FromPrimitive;

use crate::FromInt;
use crate::classes::{ClassID, UnityClass};
use crate::error::Error;
use crate::serialized_file::{SerializedFile, SerializedType};

/// Internal struct with object data.
#[derive(Debug, Clone)]
pub(crate) struct ObjectInfo {
    pub path_id: i64,
    pub start: u64,
    pub size: u32,
    pub type_id: u32,
    pub class_id: Option<i16>,
}

/// A reference to a Unity object.
#[derive(Debug, Clone)]
pub struct ObjectRef<'a> {
    pub(crate) file: &'a SerializedFile<'a>,
    pub(crate) ser_type: &'a SerializedType,
    pub(crate) object: ObjectInfo
}

impl ObjectRef<'_> {
    /// Gets the object's path ID.
    pub fn path_id(&self) -> i64 {
        self.object.path_id
    }

    /// Gets the class ID for this object's type.
    pub fn class_id(&self) -> ClassID {
        ClassID::from_primitive(self.ser_type.class_id)
    }

    /// Whether the data should be read as big endian.
    pub fn is_big_endian(&self) -> bool {
        self.file.is_big_endian
    }

    /// Gets the block of memory with the object data.
    pub fn data(&self) -> crate::Result<&[u8]> {
        let offset = usize::from_int(self.object.start + self.file.data_offset)?;
        let size = usize::from_int(self.object.size)?;

        let data = self.file.buf
            .get(offset..).ok_or(Error::InvalidData("object start out of file range"))?
            .get(..size).ok_or(Error::InvalidData("object size out of file range"))?;

        Ok(data)
    }

    /// Tries to read the object into the specified type.
    pub fn try_into_class<T: UnityClass>(&self) -> crate::Result<T> {
        T::try_from_obj(self)
    }
}
