use std::io::Cursor;

use super::UnityClass;
use crate::error::Error;
use crate::serialized_file::TypeTreeNode;
use crate::unity_fs::UnityFsFile;
use crate::{FromInt as _, define_unity_class};

define_unity_class! {
    /// Streaming information for resources.
    pub class StreamingInfo = "StreamingInfo" {
        pub offset: Offset = "offset",
        pub size: u32 = "size",
        pub path: String = "path",
    }
}

#[derive(Debug, Default, Clone, Copy)]
pub struct Offset(pub u64);

impl UnityClass for Offset {
    fn parse_tree(
        r: &mut Cursor<&[u8]>,
        is_big_endian: bool,
        root: &TypeTreeNode,
        tree: &[TypeTreeNode],
    ) -> crate::Result<Self> {
        u32::parse_tree(r, is_big_endian, root, tree)
            .map(u64::from)
            .or_else(|_| u64::parse_tree(r, is_big_endian, root, tree))
            .map(Offset)
    }
}

impl StreamingInfo {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    /// Loads the streaming data.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the streaming info points at an invalid path or
    /// section of data.
    pub fn load_data<'a>(&self, fs: &'a UnityFsFile<'a>) -> crate::Result<&'a [u8]> {
        let path = self
            .path
            .rsplit_once('/')
            .ok_or(Error::InvalidData("streaming data path incorrect"))?
            .1
            .as_bytes();

        let node = fs
            .entries()
            .find(|e| e.path_raw() == path)
            .ok_or(Error::InvalidData("streaming data file not found"))?;

        let offset = usize::from_int(self.offset.0)?;
        let size = usize::from_int(self.size)?;

        let slice = node
            .read_raw()?
            .get(offset..)
            .ok_or(Error::InvalidData("streaming data offset out of bounds"))?
            .get(..size)
            .ok_or(Error::InvalidData("streaming data size out of bounds"))?;

        Ok(slice)
    }

    /// Loads the streaming data, if its path is not empty. If it is empty,
    /// instead returns the `fallback`.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the streaming info is specified and points at an
    /// invalid path or section of data.
    pub fn load_data_or_else<'t, 'fs: 't>(
        &self,
        fs: &'fs UnityFsFile<'fs>,
        fallback: impl FnOnce() -> &'t [u8],
    ) -> crate::Result<&'t [u8]> {
        if self.path.is_empty() {
            Ok(fallback())
        } else {
            self.load_data(fs)
        }
    }
}
