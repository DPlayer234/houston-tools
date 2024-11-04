//! Main access into Unity FS archives.

// binrw emits code that doesn't get used and we hit this. ugh.
#![allow(dead_code)]

use std::borrow::Cow;
use std::cell::Cell;
use std::fmt;
use std::io::{Cursor, SeekFrom};
use std::ops::Deref;

use binrw::{binread, BinRead, NullString};
use modular_bitfield::specifiers::*;
use modular_bitfield::{bitfield, BitfieldSpecifier};
use num_enum::TryFromPrimitive;

use crate::error::Error;
use crate::serialized_file::SerializedFile;
use crate::{FromInt, SeekRead};

// Since UnityFsFile stores a `dyn SeekRead`, it cannot be `Send` and `Sync`.
// While that would be nice, short of requiring it for *every* reader there is no nice way around it.
// Subsequently, none of the code here bothers to support synchronization.

/// A UnityFS file.
#[derive(Debug)]
pub struct UnityFsFile<'a> {
    buf: DebugIgnore<Cell<Option<&'a mut dyn SeekRead>>>,
    blocks_info: BlocksInfo,
    data_offset: u64
}

/// A node within a UnityFS file.
/// Broadly represents a block of binary data.
#[derive(Debug, Clone)]
pub struct UnityFsNode<'a> {
    file: &'a UnityFsFile<'a>,
    node: &'a Node
}

/// Data for UnityFS node.
#[derive(Debug, Clone)]
pub enum UnityFsData<'a> {
    SerializedFile(SerializedFile<'a>),
    RawData(&'a [u8])
}

#[binread]
#[br(big, magic = b"UnityFS\0")] // Only going to support UnityFS and no other formats
#[derive(Clone, Debug)]
struct UnityFsHeader {
    version: u32,
    unity_version: NullString,
    unity_revision: NullString,
    size: i64,
    compressed_blocks_info_size: u32,
    uncompressed_blocks_info_size: u32,
    flags: ArchiveFlags,
}

#[bitfield]
#[binread]
#[derive(Debug, Clone)]
#[br(map = |x: u32| Self::from_bytes(x.to_le_bytes()))]
struct ArchiveFlags {
    #[bits = 6]
    compression: Compression,
    #[allow(dead_code)]
    block_directory_merged: bool,
    blocks_info_at_end: bool,
    #[allow(dead_code)]
    old_web_plugin_compatible: bool,
    blocks_info_need_start_pad: bool,
    #[allow(dead_code)]
    #[doc(hidden)]
    pad: B22
}

#[binread]
#[br(big)]
#[derive(Debug)]
struct BlocksInfo {
    data_hash: [u8; 16],
    #[br(temp)]
    blocks_count: u32,
    #[br(count = blocks_count)]
    blocks: Vec<Block>,
    #[br(temp)]
    nodes_count: u32,
    #[br(count = nodes_count)]
    nodes: Vec<Node>
}

#[binread]
#[br(big)]
#[derive(Clone, Debug)]
struct Block {
    uncompressed_size: u32,
    compressed_size: u32,
    flags: BlockFlags,
}

#[bitfield]
#[binread]
#[derive(Clone, Copy, Debug)]
#[br(map = |x: u16| Self::from_bytes(x.to_le_bytes()))]
struct BlockFlags {
    #[bits = 6]
    compression: Compression,
    #[allow(dead_code)]
    streamed: bool,
    #[skip]
    #[allow(dead_code)]
    #[doc(hidden)]
    pad: B9,
}

#[binread]
#[br(big)]
#[derive(Debug)]
struct Node {
    offset: u64,
    size: u64,
    flags: u32,
    path: NullString,

    /// Stores the uncompressed bytes for this node.
    /// This is initialized lazily when [`UnityFsNode::read_raw`] is called.
    // CMBK: replace with `std::cell::OnceCell` when its `get_or_try_init`
    // is stabilized which is probably in approximately Never™️.
    #[br(ignore)]
    uncompressed_cache: once_cell::unsync::OnceCell<Vec<u8>>,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, TryFromPrimitive, BitfieldSpecifier)]
#[bits = 6]
enum Compression {
    None = 0,
    Lzma,
    Lz4,
    Lz4Hc,
    _Lzham
}

#[derive(Debug, Clone)]
struct BlockOffset {
    index: usize,
    compressed_offset: u64,
    uncompressed_offset: u64
}

impl<'a> UnityFsFile<'a> {
    /// Reads a UnityFS from a reader.
    pub fn open(mut buf: &'a mut dyn SeekRead) -> crate::Result<Self> {
        let header = UnityFsHeader::read(&mut buf)?;

        // Load blocks info
        let blocks_info = {
            if header.version >= 7 {
                // Starting with version 7, the blocks info is aligned to the next 16-byte boundary.
                buf.align_to(16)?;
            }

            let mut compressed_data = vec![0u8; usize::from_int(header.compressed_blocks_info_size)?];

            if header.flags.blocks_info_at_end() {
                let pos = buf.stream_position()?;
                buf.seek(SeekFrom::End(-i64::from(header.compressed_blocks_info_size)))?;
                buf.read_exact(&mut compressed_data)?;
                buf.seek(SeekFrom::Start(pos))?;
            } else {
                buf.read_exact(&mut compressed_data)?;
            }

            if header.flags.blocks_info_need_start_pad() {
                buf.align_to(16)?;
            }

            let decompressed_data = decompress_data(
                &compressed_data,
                header.flags.compression(),
                header.uncompressed_blocks_info_size
            )?;

            let mut reader = Cursor::new(&*decompressed_data);
            BlocksInfo::read(&mut reader)?
        };

        let data_offset = buf.stream_position()?;

        Ok(UnityFsFile {
            buf: DebugIgnore(Cell::new(Some(buf))),
            blocks_info,
            data_offset
        })
    }

    /// Enumerates all node entries within the file.
    pub fn entries(&'a self) -> impl Iterator<Item = UnityFsNode<'a>> {
        self.blocks_info.nodes.iter().map(|n| UnityFsNode {
            file: self,
            node: n
        })
    }

    fn get_block_index_by_offset(&self, offset: u64) -> Option<BlockOffset> {
        let mut compressed_offset = 0u64;
        let mut uncompressed_offset = 0u64;
        for (index, block) in self.blocks_info.blocks.iter().enumerate() {
            let next_compressed_offset = compressed_offset + u64::from(block.compressed_size);
            let next_uncompressed_offset = uncompressed_offset + u64::from(block.uncompressed_size);

            if offset >= uncompressed_offset && offset < next_uncompressed_offset {
                return Some(BlockOffset { index, compressed_offset, uncompressed_offset });
            }

            compressed_offset = next_compressed_offset;
            uncompressed_offset = next_uncompressed_offset;
        }

        None
    }
}

impl<'a> UnityFsNode<'a> {
    fn decompress(&self) -> crate::Result<Vec<u8>> {
        let uncompressed_start = self.node.offset;
        let BlockOffset {
            index,
            mut compressed_offset,
            mut uncompressed_offset
        } = self.file.get_block_index_by_offset(uncompressed_start).ok_or(Error::InvalidData("compressed data position out of bounds"))?;

        let mut result = Vec::new();

        // in any reasonable scenario, this expect should be impossible to hit.
        // however, it's not impossible to construct a scenario where `UnityFsFile -> reader -> UnityFsFile` holds true,
        // in which case this would trigger. if that wasn't possible, an `UnsafeCell` might be appropriate.
        //
        // `buf` is returned after the loop or when this function returns early.
        let mut guard = BufGuard::take(self.file);
        let buf = guard.buf();

        for block in &self.file.blocks_info.blocks[index ..] {
            // Read and decompress the entire block
            let start = compressed_offset + self.file.data_offset;
            let mut compressed_data = vec![0u8; usize::from_int(block.compressed_size)?];

            buf.seek(SeekFrom::Start(start))?;
            buf.read_exact(&mut compressed_data)?;

            let uncompressed_data = decompress_data(
                &compressed_data,
                block.flags.compression(),
                block.uncompressed_size
            )?;

            // Determine the relative offsets for this file into this block
            let sub_start = usize::from_int(uncompressed_start.saturating_sub(uncompressed_offset))?;
            let missing_size = usize::from_int(self.node.size - u64::from_int(result.len())?)?;
            let sub_end = sub_start + missing_size;

            if sub_end <= uncompressed_data.len() {
                result.extend(&uncompressed_data[sub_start .. sub_end]);
                break
            }

            result.extend(&uncompressed_data[sub_start ..]);

            compressed_offset += u64::from(block.compressed_size);
            uncompressed_offset += u64::from(block.uncompressed_size);
        }

        // sanity check to ensure the guard is still valid here and can be dropped
        drop(guard);

        debug_assert!(
            u64::from_int(result.len())? == self.node.size,
            "sanity: result len is {}, but the node specified {}", result.len(), self.node.size
        );
        Ok(result)
    }

    /// Reads the raw binary data for this node.
    pub fn read_raw(&self) -> crate::Result<&'a [u8]> {
        Ok(self.node.uncompressed_cache.get_or_try_init(|| self.decompress())?)
    }

    /// Reads the data for this node.
    pub fn read(&self) -> crate::Result<UnityFsData<'a>>{
        let buf = self.read_raw()?;
        if SerializedFile::is_serialized_file(buf) {
            Ok(UnityFsData::SerializedFile(SerializedFile::read(buf)?))
        } else {
            Ok(UnityFsData::RawData(buf))
        }
    }

    /// Gets the path name for this node.
    ///
    /// This will allocate a UTF-8 string with escape sequences for invalid characters in the underlying data.
    /// If you don't care about that or want to avoid the allocation, use [`Self::path_raw`] instead.
    #[must_use]
    pub fn path(&self) -> String {
        String::from_utf8_lossy(&self.node.path.0).into_owned()
    }

    /// Gets the path name for this node as raw bytes.
    ///
    /// This is the raw data as it appears in the file and doesn't necessarily represent valid UTF-8.
    #[must_use]
    pub fn path_raw(&self) -> &[u8] {
        &self.node.path.0
    }
}

fn decompress_data(compressed_data: &[u8], compression: Compression, size: u32) -> crate::Result<Cow<'_, [u8]>> {
    match compression {
        Compression::None => Ok(Cow::Borrowed(compressed_data)),
        Compression::Lz4 | Compression::Lz4Hc => Ok(Cow::Owned(lz4::block::decompress(compressed_data, Some(i32::from_int(size)?))?)),
        Compression::Lzma => {
            use lzma_rs::decompress::*;

            let mut output = Cursor::new(Vec::with_capacity(usize::from_int(size)?));
            let mut reader = Cursor::new(compressed_data);
            lzma_rs::lzma_decompress_with_options(&mut reader, &mut output, &Options {
                unpacked_size: UnpackedSize::UseProvided(Some(u64::from(size))),
                ..Default::default()
            })?;
            Ok(Cow::Owned(output.into_inner()))
        }
        _ => Err(Error::Unsupported(
            format!("unsupported compression method: {compression:?}")
        ))
    }
}

#[derive(Clone)]
struct DebugIgnore<T>(pub T);

impl<T> Deref for DebugIgnore<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> fmt::Debug for DebugIgnore<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("<hidden>")
    }
}

impl<T: fmt::Display> fmt::Display for DebugIgnore<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

/// Allows easy exclusive access to the buffer that's backing a [`UnityFsFile`].
struct BufGuard<'a> {
    file: &'a UnityFsFile<'a>,
    buf: Option<&'a mut dyn SeekRead>,
}

impl<'a> BufGuard<'a> {
    /// Takes the buffer. Panics if it isn't available.
    fn take(file: &'a UnityFsFile<'a>) -> Self {
        let buf = file.buf.take()
            .expect("reader passed to UnityFsFile should not access the same UnityFsFile instance");

        Self { file, buf: Some(buf) }
    }

    /// Gets the buffer.
    fn buf(&mut self) -> &mut dyn SeekRead {
        // `as_mut` is required to get the reborrow to work correctly
        self.buf.as_mut()
            .expect("buf cannot be accessed after drop")
    }
}

impl Drop for BufGuard<'_> {
    fn drop(&mut self) {
        // return the buf to the file
        self.file.buf.set(self.buf.take());
    }
}
