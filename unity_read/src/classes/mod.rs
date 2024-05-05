//! Provides access to Unity class/object data.

use std::io::{Cursor, Seek, SeekFrom};

use binrw::BinRead;

use crate::object::ObjectRef;
use crate::{UnityError, UnityMismatch};
use crate::serialized_file::TypeTreeNode;
use crate::read_endian;

mod class_id;
mod mesh;
mod streaming_info;
mod texture2d;

pub use class_id::*;
pub use mesh::*;
pub use streaming_info::*;
pub use texture2d::{Texture2D, Texture2DData};

/// Trait that allows reading Unity object data in a structured form.
pub trait UnityClass: Default {
    /// Parses a tree into a structure.
    ///
    /// `tree` holds the necessary part of the tree to parse children.
    fn parse_tree(r: &mut Cursor<&[u8]>, is_big_endian: bool, root: &TypeTreeNode, tree: &[TypeTreeNode]) -> anyhow::Result<Self>;

    /// Tries to load a structure from an object reference.
    fn try_from_obj(obj: &ObjectRef) -> anyhow::Result<Self> {
        let cursor = &mut Cursor::new(obj.data());
        if let Some((root, tree)) = obj.ser_type.type_tree.split_first() {
            Self::parse_tree(cursor, obj.is_big_endian(), root, tree)
        } else {
            Err(UnityError::InvalidData("type tree is unexpectedly empty"))?
        }
    }

    #[doc(hidden)]
    fn align_reader(r: &mut Cursor<&[u8]>) -> anyhow::Result<()> {
        let pos = r.position();
        let offset = pos % 4u64;
        if offset != 0 {
            r.seek(SeekFrom::Current(4i64 - offset as i64))?;
        }

        Ok(())
    }

    #[doc(hidden)]
    fn skip(r: &mut Cursor<&[u8]>, is_big_endian: bool, root: &TypeTreeNode, tree: &[TypeTreeNode]) -> anyhow::Result<()> {
        if root.size >= 0 {
            r.seek(SeekFrom::Current(i64::from(root.size)))?;
        } else {
            match root.type_name.as_str() {
                "Array" | "TypelessData" => {
                    // The first element is the size, and the second is the child data.
                    // We assume that there cannot be siblings after that.
                    let size = read_endian!(u32, is_big_endian, r)?;
                    let (next, children) = tree.get(1usize ..)
                        .and_then(|o| o.split_first())
                        .ok_or(UnityError::InvalidData("skipped array type data does not contain data element"))?;

                    for _ in 0 .. size {
                        Self::skip(r, is_big_endian, next, children)?;
                    }
                }
                _ => {
                    let mut rest = tree;
                    while let Some((next, children, siblings)) = split_tree(rest) {
                        Self::skip(r, is_big_endian, next, children)?;
                        rest = siblings;
                    }
                }
            }
        }

        if (root.meta_flags & 0x4000) != 0 {
            Self::align_reader(r)?;
        }

        Ok(())
    }
}

/// Splits the tree into:
///
/// - The next root node
/// - its children
/// - its siblings
///
/// If empty, returns [`None`].
pub fn split_tree(tree: &[TypeTreeNode]) -> Option<(&TypeTreeNode, &[TypeTreeNode], &[TypeTreeNode])> {
    let (next, other) = tree.split_first()?;

    let mut last_index = 0usize;
    for tree in other.iter() {
        if tree.level <= next.level {
            break
        }

        last_index += 1;
    }

    let (children, siblings) = other.split_at(last_index);
    Some((next, children, siblings))
}

/// Defines a new structure that represents a Unity class.
///
/// The [`UnityClass`] implementation will skip unknown fields and leave ones not found as default.
/// If this needs to be known, wrap fields in an [`Option`].
///
/// The resulting class will additionally implement [`Default`], [`Clone`], and [`std::fmt::Debug`].
///
/// # Example
///
/// ```
/// use crate::define_unity_class;
///
/// define_unity_class! {
///     /// Data for Unity's Texture2D class.
///     pub class Texture2D = "Texture2D" {
///         pub name: String = "m_Name",
///         pub width: i32 = "m_Width",
///         pub height: i32 = "m_Height",
///         format: i32 = "m_TextureFormat",
///     }
/// }
/// ```
#[macro_export]
macro_rules! define_unity_class {
    (
        $(#[$attr:meta])*
        $v:vis class $Type:ident = $type_key:literal {
            $(
                $field_vis:vis $field_name:ident : $FieldType:ty = $key:literal
            ),* $(,)?
        }
    ) => {
        $(#[$attr])*
        #[derive(Debug, Clone, Default)]
        $v struct $Type {
            $(
                $field_vis $field_name : $FieldType
            ),*
        }

        impl $crate::classes::UnityClass for $Type {
            fn parse_tree(r: &mut std::io::Cursor<&[u8]>, is_big_endian: bool, root: &$crate::serialized_file::TypeTreeNode, tree: &[$crate::serialized_file::TypeTreeNode]) -> anyhow::Result<Self> {
                if root.type_name.as_str() != $type_key {
                    ::core::result::Result::Err($crate::UnityError::Mismatch($crate::UnityMismatch {
                        expected: $type_key.to_string(),
                        received: root.type_name.clone()
                    }))?
                }

                let mut result = <Self as Default>::default();

                let mut rest = tree;
                while let Some((next, children, siblings)) = $crate::classes::split_tree(rest) {
                    match next.name.as_str() {
                        $(
                            $key => { result.$field_name = <$FieldType as $crate::classes::UnityClass>::parse_tree(r, is_big_endian, next, children)?; },
                        )*
                        _ => { Self::skip(r, is_big_endian, next, children)?; }
                    }

                    rest = siblings;
                }

                if (root.meta_flags & 0x4000) != 0 {
                    Self::align_reader(r)?;
                }

                ::core::result::Result::Ok(result)
            }
        }
    };
}

macro_rules! check_mismatch {
    ($root:expr, $expected:literal $(| $extra:literal)*) => {{
        match $root.type_name.as_str() {
            $expected $(| $extra)* => (),
            _ => Err(UnityError::Mismatch(UnityMismatch {
                expected: $expected.into(),
                received: $root.type_name.clone()
            }))?
        }
    }};
}

impl UnityClass for String {
    fn parse_tree(r: &mut Cursor<&[u8]>, is_big_endian: bool, root: &TypeTreeNode, tree: &[TypeTreeNode]) -> anyhow::Result<Self> {
        check_mismatch!(root, "string");

        // string should always have an Array of char nested
        let (next, children) = tree.split_first()
            .ok_or(UnityError::InvalidData("string type data does not contain children"))?;

        let data = <Vec<u8>>::parse_tree(r, is_big_endian, next, children)?;

        Ok(String::from_utf8(data)?)
    }
}

impl<T: UnityClass> UnityClass for Option<T> {
    fn parse_tree(r: &mut Cursor<&[u8]>, is_big_endian: bool, root: &TypeTreeNode, tree: &[TypeTreeNode]) -> anyhow::Result<Self> {
        // Just deletes to the inner type and wraps it in Some
        T::parse_tree(r, is_big_endian, root, tree).map(Some)
    }
}

impl<T: UnityClass> UnityClass for Vec<T> {
    fn parse_tree(r: &mut Cursor<&[u8]>, is_big_endian: bool, root: &TypeTreeNode, tree: &[TypeTreeNode]) -> anyhow::Result<Self> {
        if root.type_name.as_str() == "vector" {
            let (next, children) = tree.split_first()
                .ok_or(UnityError::InvalidData("vector type data does not contain children"))?;

            let result = Self::parse_tree(r, is_big_endian, next, children)?;

            if (root.meta_flags & 0x4000) != 0 {
                Self::align_reader(r)?;
            }

            return Ok(result)
        }

        check_mismatch!(root, "Array" | "TypelessData");

        // The first element is the size, and the second is the child data.
        // We assume that there cannot be siblings after that.
        let len = read_endian!(u32, is_big_endian, r)?;
        let (next, children) = tree.get(1usize ..)
            .and_then(|o| o.split_first())
            .ok_or(UnityError::InvalidData("array type data does not contain data element"))?;

        let mut result = Vec::new();
        for _ in 0 .. len {
            result.push(T::parse_tree(r, is_big_endian, next, children)?);
        }

        if (root.meta_flags & 0x4000) != 0 {
            Self::align_reader(r)?;
        }

        Ok(result)
    }
}

macro_rules! impl_unity_class_simple {
    ($Type:ty, $expected:literal $(| $extra:literal)*) => {
        impl UnityClass for $Type {
            fn parse_tree(r: &mut Cursor<&[u8]>, is_big_endian: bool, root: &TypeTreeNode, _tree: &[TypeTreeNode]) -> anyhow::Result<Self> {
                check_mismatch!(root, $expected $(| $extra)*);

                let value = read_endian!($Type, is_big_endian, r)?;
                if (root.meta_flags & 0x4000) != 0 {
                    Self::align_reader(r)?;
                }

                Ok(value)
            }
        }
    };
}

impl_unity_class_simple!(i8, "SInt8");
impl_unity_class_simple!(u8, "UInt8" | "char");
impl_unity_class_simple!(i16, "SInt16" | "short");
impl_unity_class_simple!(u16, "UInt16" | "unsigned short");
impl_unity_class_simple!(i32, "SInt32" | "int");
impl_unity_class_simple!(u32, "UInt32" | "unsigned int" | "Type*");
impl_unity_class_simple!(i64, "SInt64" | "long long");
impl_unity_class_simple!(u64, "UInt64" | "unsigned long long" | "FileSize");
impl_unity_class_simple!(f32, "float");
impl_unity_class_simple!(f64, "double");
