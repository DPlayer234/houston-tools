use crate::serialized_file::TypeTreeNode;
use crate::{define_unity_class, UnityError};
use crate::unity_fs::UnityFsFile;
use super::UnityClass;

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
    fn parse_tree(r: &mut std::io::Cursor<&[u8]>, is_big_endian: bool, root: &TypeTreeNode, tree: &[TypeTreeNode]) -> anyhow::Result<Self> {
        u32::parse_tree(r, is_big_endian, root, tree).map(u64::from)
            .or_else(|_| u64::parse_tree(r, is_big_endian, root, tree))
            .map(Offset)
    }
}

impl StreamingInfo {
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    /// Loads the streaming data.
    pub fn load_data<'a>(&self, fs: &'a UnityFsFile<'a>) -> anyhow::Result<&'a [u8]> {
        let path = self.path.split('/').last().ok_or(UnityError::InvalidData("streaming data path incorrect"))?.as_bytes();
        let node = fs.entries().find(|e| e.path_raw() == path).ok_or(UnityError::InvalidData("streaming data file not found"))?;

        let offset = usize::try_from(self.offset.0)?;
        let size = usize::try_from(self.size)?;

        let slice = node.read_raw()?
            .get(offset..).ok_or(UnityError::InvalidData("streaming data offset out of bounds"))?
            .get(..size).ok_or(UnityError::InvalidData("streaming data size out of bounds"))?;

        Ok(slice)
    }

    pub fn load_data_or_else<'t, 'fs: 't>(&self, fs: &'fs UnityFsFile<'fs>, fallback: impl FnOnce() -> &'t [u8]) -> anyhow::Result<&'t [u8]> {
        if self.path.is_empty() {
            Ok(fallback())
        } else {
            self.load_data(fs)
        }
    }
}