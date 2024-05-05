use crate::{define_unity_class, UnityError};
use crate::unity_fs::UnityFsFile;

define_unity_class! {
    /// Streaming information for resources.
    pub class StreamingInfo = "StreamingInfo" {
        pub offset: u32 = "offset",
        pub size: u32 = "size",
        pub path: String = "path",
    }
}

impl StreamingInfo {
    pub fn is_empty(&self) -> bool {
        self.path.is_empty()
    }

    /// Loads the streaming data.
    pub fn load_data(&self, fs: &UnityFsFile) -> anyhow::Result<Vec<u8>> {
        let path = self.path.split('/').last().ok_or(UnityError::InvalidData("image streaming data path incorrect"))?;
        let node = fs.entries().find(|e| e.path().as_str() == path).ok_or(UnityError::InvalidData("stream data file not found"))?;

        let mut full = node.read_raw()?;
        if self.offset == 0 {
            full.truncate(self.size as usize);
            Ok(full)
        } else {
            Ok(full[self.offset as usize ..][.. self.size as usize].to_vec())
        }
    }
}