//! Structs for using serialized files within UnityFS.

use std::io::{Cursor, Read, Seek, SeekFrom};

use binrw::{binread, BinRead, NullString};

use crate::object::{ObjectInfo, ObjectRef};
use crate::read_endian;

/// Information about the serialized files.
#[derive(Debug, Clone, Default)]
pub struct SerializedFile<'a> {
    pub(crate) buf: &'a [u8],
    metadata_size: u32,
    file_size: u64,
    pub version: u32,
    pub data_offset: u64,
    pub unity_version: Option<NullString>,
    target_platform: Option<u32>,
    pub is_big_endian: bool,
    enable_type_tree: bool,
    big_id_enabled: bool,
    types: Vec<SerializedType>,
    objects: Vec<ObjectInfo>
}

/// Information about a serialized type.
#[derive(Debug, Clone, Default)]
pub struct SerializedType {
    pub class_id: u32,
    is_stripped_type: bool,
    script_type_index: Option<u16>,
    script_id: Option<[u8; 16]>,
    pub type_tree: Vec<TypeTreeNode>,
}

/// A node within the type tree. Which is a list. That still represents a tree.
#[derive(Debug, Clone, Default)]
pub struct TypeTreeNode {
    pub type_name: String,
    pub name: String,
    pub size: i32,
    pub index: u32,
    pub type_flags: u32,
    pub version: u32,
    pub meta_flags: u32,
    pub level: u8,
}

impl<'a> SerializedFile<'a> {
    /// Enumerates the objects listed within this file.
    pub fn objects(&'a self) -> impl Iterator<Item = ObjectRef<'a>> {
        self.objects.iter().map(|obj| ObjectRef {
            file: self,
            // the below will panic if the class_id and type_id don't map to anything
            ser_type: &obj.class_id
                .and_then(|c| self.types.iter().find(|t| t.class_id == u32::from(c)))
                .unwrap_or_else(|| &self.types[obj.type_id as usize]),
            object: obj.clone()
        })
    }

    /// Gets the serialized types.
    pub fn types(&self) -> &[SerializedType] {
        &self.types
    }

    /// Determines whether a buffer represents a serialized file.
    pub(crate) fn is_serialized_file(buf: &[u8]) -> bool {
        let cursor = &mut Cursor::new(buf);
        let Ok(main) = HeaderMain::read(cursor) else { return false };
        if main.file_size < main.metadata_size { return false }

        if main.version >= 9 {
            if matches!(HeaderV9Ext::read(cursor), Err(_)) { return false }
        } else {
            cursor.set_position(u64::from(main.file_size - main.metadata_size));
            if matches!(u8::read(cursor), Err(_)) { return false }
        }

        if main.version >= 22 {
            let Ok(v22ext) = HeaderV22Ext::read(cursor) else { return false };
            buf.len() == v22ext.file_size as usize && v22ext.data_offset <= v22ext.file_size
        } else {
            buf.len() == main.file_size as usize && main.data_offset <= main.file_size
        }
    }

    /// Reads a buffer into a [`SerializedFile`] struct.
    pub fn read(buf: &'a [u8]) -> anyhow::Result<Self> {
        let cursor = &mut Cursor::new(buf);

        let mut result = SerializedFile::default();

        let main = HeaderMain::read(cursor)?;
        result.metadata_size = main.metadata_size;
        result.file_size = u64::from(main.file_size);
        result.version = main.version;
        result.data_offset = u64::from(main.data_offset);

        result.is_big_endian = if main.version >= 9 {
            let v9ext = HeaderV9Ext::read(cursor)?;
            v9ext.endian
        } else {
            cursor.set_position(u64::from(main.file_size - main.metadata_size));
            u8::read(cursor)?
        } != 0;

        if main.version >= 22 {
            let v22ext = HeaderV22Ext::read(cursor)?;
            result.metadata_size = v22ext.metadata_size;
            result.file_size = v22ext.file_size;
            result.data_offset = v22ext.data_offset;
        }

        if main.version >= 7 {
            result.unity_version = Some(NullString::read(cursor)?);
        }

        // Endianness applies from here.
        if main.version >= 8 {
            result.target_platform = Some(read_endian!(u32, result.is_big_endian, cursor)?);
        }

        if main.version >= 13 {
            result.enable_type_tree = u8::read(cursor)? != 0;
        }

        let type_count = read_endian!(u32, result.is_big_endian, cursor)?;
        for _ in 0 .. type_count {
            result.types.push(result.read_serialized_type(cursor, false)?);
        }

        if result.version >= 7 && result.version < 14 {
            result.big_id_enabled = read_endian!(u32, result.is_big_endian, cursor)? != 0;
        }

        let object_count = read_endian!(u32, result.is_big_endian, cursor)?;
        for _ in 0 .. object_count {
            result.objects.push(result.read_object_info(cursor)?);
        }

        // Skipping trying to read script file refs, external file refs, ref types, and user info for now

        // Also move the buffer in.
        result.buf = buf;
        Ok(result)
    }

    fn read_serialized_type(&self, cursor: &mut LocalCursor, is_ref_type: bool) -> anyhow::Result<SerializedType> {
        let mut result = SerializedType {
            class_id: read_endian!(u32, self.is_big_endian, cursor)?,
            .. SerializedType::default()
        };

        if self.version >= 16 {
            result.is_stripped_type = u8::read(cursor)? != 0;
        }

        if self.version >= 17 {
            result.script_type_index = Some(u16::read_be(cursor)?);
        }

        if self.version >= 13 {
            if (is_ref_type && result.script_type_index.is_some())
            || (self.version < 16 && result.class_id >= 0x8000_0000)
            || (self.version >= 16 && result.class_id == 114 /* Script */) {
                result.script_id = Some(BinRead::read(cursor)?);
            }

            // old type hash? Either way, 16 bytes to skip, we don't need this.
            let _ = <[u8; 16]>::read(cursor)?;
        }

        if self.enable_type_tree {
            result.type_tree = if self.version >= 12 || self.version == 10 {
                // Unity, what happened in version 11 and 12???
                self.read_type_tree_blob(cursor)?
            } else {
                self.read_type_tree(cursor, 0)?
            };

            // I don't think I really need this set of data.
            if self.version >= 21 {
                if is_ref_type {
                    let _ = read_endian!(SerializedTypeRefNames, self.is_big_endian, cursor)?;
                } else {
                    let _ = read_endian!(SerializedTypeDeps, self.is_big_endian, cursor)?;
                }
            }
        }

        Ok(result)
    }

    fn read_type_tree_blob(&self, cursor: &mut LocalCursor) -> anyhow::Result<Vec<TypeTreeNode>> {
        let node_count = read_endian!(u32, self.is_big_endian, cursor)?;
        let str_buf_size = read_endian!(u32, self.is_big_endian, cursor)?;

        let mut raw_nodes = Vec::new();

        for _ in 0 .. node_count {
            let raw_node = read_endian!(TypeTreeNodeBlob, self.is_big_endian, cursor)?;

            if self.version >= 19 {
                // ref type hash
                let _ = read_endian!(u64, self.is_big_endian, cursor)?;
            }

            raw_nodes.push(raw_node);
        }

        // what kinda unhinged behavior is putting the length for this earlier
        let mut str_buf = vec![0u8; str_buf_size as usize];
        cursor.read_exact(&mut str_buf)?;

        fn read_str(cursor: &mut LocalCursor, offset: u32) -> anyhow::Result<String> {
            // If the last bit is set, the remainder indicates an index into a table
            // of common known strings rather than actually storing the data.
            Ok(if (offset & 0x8000_0000) == 0 {
                cursor.set_position(u64::from(offset));
                NullString::read(cursor)?.try_into()?
            } else {
                super::unity_fs_common_str::index_to_common_string(offset & 0x7FFF_FFFF)
                    .map(String::from)
                    .unwrap_or_else(|| format!("unknown:{offset}"))
            })
        }

        let str_cursor = &mut Cursor::new(str_buf.as_slice());
        let nodes = raw_nodes.iter()
            .map(|raw_node| Ok(TypeTreeNode {
                type_name: read_str(str_cursor, raw_node.type_str_offset)?,
                name: read_str(str_cursor, raw_node.name_str_offset)?,
                size: raw_node.size,
                index: raw_node.index,
                type_flags: u32::from(raw_node.type_flags),
                version: u32::from(raw_node.version),
                meta_flags: raw_node.meta_flags,
                level: raw_node.level,
            })).collect::<anyhow::Result<Vec<_>>>()?;

        //for node in &nodes {
        //    println!("{: >l$} {} {} w {} @ {}, {}", "", node.type_name, node.name, node.size, (node.meta_flags & 0x4000) != 0, node.index, l = (node.level * 4) as usize);
        //}

        Ok(nodes)
    }

    fn read_type_tree(&self, cursor: &mut LocalCursor, level: u8) -> anyhow::Result<Vec<TypeTreeNode>> {
        // this format is dogshit
        let mut node = TypeTreeNode {
            level,
            type_name: NullString::read(cursor)?.try_into()?,
            name: NullString::read(cursor)?.try_into()?,
            size: read_endian!(i32, self.is_big_endian, cursor)?,
            .. TypeTreeNode::default()
        };

        if self.version > 1 {
            node.index = read_endian!(u32, self.is_big_endian, cursor)?;
        }

        node.type_flags = read_endian!(u32, self.is_big_endian, cursor)?;
        node.version = read_endian!(u32, self.is_big_endian, cursor)?;

        // unity wtf why is this missing in one version
        if self.version != 3 {
            node.meta_flags = read_endian!(u32, self.is_big_endian, cursor)?;
        }

        let mut nodes = Vec::new();

        // we flatten it because other people do that here
        // also because i don't get why this is even a tree
        // no really, why?
        let child_count = read_endian!(u32, self.is_big_endian, cursor)?;
        for _ in 0 .. child_count {
            nodes.extend(self.read_type_tree(cursor, level + 1)?);
        }

        Ok(nodes)
    }

    fn read_object_info(&self, cursor: &mut LocalCursor) -> anyhow::Result<ObjectInfo> {
        let mut object = if self.big_id_enabled {
            // Big ID flag only exists from v7 to v13
            ObjectInfo::from(read_endian!(ObjectBlobBigId, self.is_big_endian, cursor)?)
        } else if self.version < 14 {
            ObjectInfo::from(read_endian!(ObjectBlob, self.is_big_endian, cursor)?)
        } else if self.version < 22 {
            // Starting with v14, big ID is the default, and it is aligned.
            align_cursor(cursor)?;
            ObjectInfo::from(read_endian!(ObjectBlobBigId, self.is_big_endian, cursor)?)
        } else {
            // With v22, the blob start changes to 64-bit
            align_cursor(cursor)?;
            ObjectInfo::from(read_endian!(ObjectBlobV22, self.is_big_endian, cursor)?)
        };

        // Up to v16, class_id maps the the type's type_id.
        // After, the object's type_id is an index into the types list.
        if self.version < 16 {
            object.class_id = Some(read_endian!(u16, self.is_big_endian, cursor)?);
        }

        if self.version < 11 {
            // is destroyed
            let _ = read_endian!(u16, self.is_big_endian, cursor)?;
        }

        if self.version >= 11 && self.version < 17 {
            // object's own script type index
            let _ = read_endian!(u16, self.is_big_endian, cursor)?;
        }

        if self.version >= 15 && self.version < 17 {
            // stripped flag
            let _ = u8::read(cursor)?;
        }

        Ok(object)
    }
}

type LocalCursor<'a> = Cursor<&'a [u8]>;

fn align_cursor(cursor: &mut LocalCursor) -> anyhow::Result<()> {
    let pos = cursor.position();
    let offset = pos % 4u64;
    if offset != 0 {
        cursor.seek(SeekFrom::Current(4i64 - offset as i64))?;
    }

    Ok(())
}

#[binread]
#[br(big)]
#[derive(Debug, Clone)]
struct HeaderMain {
    metadata_size: u32,
    file_size: u32,
    version: u32,
    data_offset: u32
}

#[binread]
#[br(big)]
#[derive(Debug, Clone)]
struct HeaderV9Ext {
    endian: u8,
    #[allow(dead_code)]
    reserved: [u8; 3]
}

#[binread]
#[br(big)]
#[derive(Debug, Clone)]
struct HeaderV22Ext {
    metadata_size: u32,
    file_size: u64,
    data_offset: u64,
    #[allow(dead_code)]
    reserved: u64
}

#[allow(dead_code)]
#[binread]
#[derive(Debug, Clone)]
struct SerializedTypeRefNames {
    class_name: NullString,
    namespace: NullString,
    asm_name: NullString,
}

#[allow(dead_code)]
#[binread]
#[derive(Debug, Clone)]
struct SerializedTypeDeps {
    #[br(temp)]
    count: u32,
    #[br(count = count)]
    vec: Vec<u32>
}

#[binread]
#[derive(Debug, Clone)]
struct TypeTreeNodeBlob {
    version: u16,
    level: u8,
    type_flags: u8,
    type_str_offset: u32,
    name_str_offset: u32,
    size: i32,
    index: u32,
    meta_flags: u32,
}

#[binread]
#[derive(Debug, Clone)]
struct ObjectBlob {
    path_id: u32,
    start: u32,
    size: u32,
    type_id: u32
}

#[binread]
#[derive(Debug, Clone)]
struct ObjectBlobBigId {
    path_id: u64,
    start: u32,
    size: u32,
    type_id: u32
}

#[binread]
#[derive(Debug, Clone)]
struct ObjectBlobV22 {
    path_id: u64,
    start: u64,
    size: u32,
    type_id: u32
}

macro_rules! impl_obj_blob_to_info {
    ($Source:ty) => {
        impl From<$Source> for ObjectInfo {
            fn from(value: $Source) -> Self {
                Self {
                    path_id: u64::from(value.path_id),
                    start: u64::from(value.start),
                    size: value.size,
                    type_id: value.type_id,
                    class_id: None
                }
            }
        }
    };
}

impl_obj_blob_to_info!(ObjectBlob);
impl_obj_blob_to_info!(ObjectBlobBigId);
impl_obj_blob_to_info!(ObjectBlobV22);
