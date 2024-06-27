// binrw emits code that doesn't get used and we hit this. ugh.
#![allow(dead_code)]

use std::borrow::Cow;
use std::cell::Cell;
use std::io::{Cursor, Read, Seek, SeekFrom};
use std::ops::Deref;

use binrw::{binread, BinRead, NullString};
use num_enum::TryFromPrimitive;
use modular_bitfield::{bitfield, BitfieldSpecifier};
use modular_bitfield::specifiers::*;

use crate::serialized_file::SerializedFile;
use crate::UnityError;

// Since UnityFsFile stores a `dyn SeekRead`, it cannot be !Send and !Sync.
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
#[derive(Debug, Clone)]
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
#[derive(Clone, Copy, Debug, PartialEq)]
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
#[derive(Clone, Debug, PartialEq)]
struct Node {
    offset: u64,
    size: u64,
    flags: u32,
    path: NullString,

    /// Stores the uncompressed bytes for this node.
    /// This is initialized lazily when [`UnityFsNode::read_raw`] is called.
    #[br(ignore)]
    uncompressed_cache: std::rc::Rc<once_cell::unsync::OnceCell<Vec<u8>>>
}

#[repr(u32)]
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, TryFromPrimitive, BitfieldSpecifier)]
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
    pub fn open(mut buf: &'a mut dyn SeekRead) -> anyhow::Result<Self> {
        let header = UnityFsHeader::read(&mut buf)?;

        fn seek_to_16_byte_boundary(buf: &mut dyn SeekRead) -> anyhow::Result<()> {
            let pos = buf.stream_position()?;
            let offset = pos % 16;
            if offset != 0 {
                buf.seek(SeekFrom::Current(16i64 - offset as i64))?;
            }

            Ok(())
        }

        // Load blocks info
        let blocks_info = {
            if header.version >= 7 {
                // Starting with version 7, the blocks info is aligned to the next 16-byte boundary.
                seek_to_16_byte_boundary(buf)?;
            }

            let mut compressed_data = vec![0u8; header.compressed_blocks_info_size.try_into()?];

            if header.flags.blocks_info_at_end() {
                let pos = buf.stream_position()?;
                buf.seek(SeekFrom::End(-i64::from(header.compressed_blocks_info_size)))?;
                buf.read_exact(&mut compressed_data)?;
                buf.seek(SeekFrom::Start(pos))?;
            } else {
                buf.read_exact(&mut compressed_data)?;
            }

            if header.flags.blocks_info_need_start_pad() {
                seek_to_16_byte_boundary(buf)?;
            }

            let decompressed_data = decompress_data(
                &compressed_data,
                header.flags.compression(),
                header.uncompressed_blocks_info_size
            )?;

            let mut reader = Cursor::new(decompressed_data.deref());
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
    fn decompress(&self) -> anyhow::Result<Vec<u8>> {
        let uncompressed_start = self.node.offset;
        let BlockOffset {
            index,
            mut compressed_offset,
            mut uncompressed_offset
        } = self.file.get_block_index_by_offset(uncompressed_start).ok_or(UnityError::InvalidData("compressed data position out of bounds"))?;

        let mut result = Vec::new();

        // in any reasonable scenario, this expect should be impossible to hit.
        // however, it's not impossible to construct a scenario where `UnityFsFile -> reader -> UnityFsFile` holds true,
        // in which case this would trigger. if that wasn't possible, an `UnsafeCell` might be appropriate.
        //
        // `buf` is returned after the loop.
        let buf = self.file.buf.take()
            .expect("reader passed to UnityFsFile should not access the same UnityFsFile instance");

        for block in &self.file.blocks_info.blocks[index ..] {
            // Read and decompress the entire block
            let start = compressed_offset + self.file.data_offset;
            let mut compressed_data = vec![0u8; block.compressed_size.try_into()?];

            buf.seek(SeekFrom::Start(start))?;
            buf.read_exact(&mut compressed_data)?;

            let uncompressed_data = decompress_data(
                &compressed_data,
                block.flags.compression(),
                block.uncompressed_size
            )?;

            // Determine the relative offsets for this file into this block
            let sub_start = uncompressed_start.saturating_sub(uncompressed_offset) as usize;
            let missing_size = (self.node.size - result.len() as u64) as usize;
            let sub_end = sub_start + missing_size;

            if sub_end <= uncompressed_data.len() {
                result.extend(&uncompressed_data[sub_start .. sub_end]);
                break
            }

            result.extend(&uncompressed_data[sub_start ..]);

            compressed_offset += u64::from(block.compressed_size);
            uncompressed_offset += u64::from(block.uncompressed_size);
        }

        // return the buffer so future have access to it again
        self.file.buf.set(Some(buf));

        debug_assert!(result.len() as u64 == self.node.size);
        Ok(result)
    }

    /// Reads the raw binary data for this node.
    pub fn read_raw(&self) -> anyhow::Result<&'a [u8]> {
        Ok(&self.node.uncompressed_cache.get_or_try_init(|| self.decompress())?)
    }

    /// Reads the data for this node.
    pub fn read(&self) -> anyhow::Result<UnityFsData<'a>>{
        let buf = self.read_raw()?;
        if SerializedFile::is_serialized_file(&buf) {
            Ok(UnityFsData::SerializedFile(SerializedFile::read(buf)?))
        } else {
            Ok(UnityFsData::RawData(buf))
        }
    }
}

impl UnityFsNode<'_> {
    /// Gets the path name for this node.
    pub fn path(&self) -> String {
        String::from_utf8_lossy(&self.node.path.0).into_owned()
    }
}

fn decompress_data(compressed_data: &[u8], compression: Compression, size: u32) -> anyhow::Result<Cow<[u8]>> {
    match compression {
        Compression::None => Ok(Cow::Borrowed(compressed_data)),
        Compression::Lz4 | Compression::Lz4Hc => Ok(Cow::Owned(lz4::block::decompress(compressed_data, Some(size.try_into()?))?)),
        Compression::Lzma => {
            use lzma_rs::decompress::*;

            let mut output = Cursor::new(Vec::with_capacity(size.try_into()?));
            let mut reader = Cursor::new(compressed_data);
            lzma_rs::lzma_decompress_with_options(&mut reader, &mut output, &Options {
                unpacked_size: UnpackedSize::UseProvided(Some(u64::from(size))),
                ..Default::default()
            })?;
            Ok(Cow::Owned(output.into_inner()))
        }
        _ => Err(UnityError::Unsupported("unsupported compression method"))?
    }
}

pub trait SeekRead: Read + Seek {}
impl<T: Read + Seek> SeekRead for T {}

#[derive(Clone)]
struct DebugIgnore<T>(pub T);

impl<T> Deref for DebugIgnore<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::fmt::Debug for DebugIgnore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("<hidden>")
    }
}

impl<T: std::fmt::Display> std::fmt::Display for DebugIgnore<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Display::fmt(&self.0, f)
    }
}
