//! Provides access to Unity class/object data.

use std::io::{Cursor, Seek as _, SeekFrom};

use crate::error::Error;
use crate::object::ObjectRef;
use crate::serialized_file::TypeTreeNode;
use crate::{BinReadEndian as _, SeekRead as _};

mod asset_bundle;
mod class_id;
mod mesh;
mod streaming_info;
mod text_asset;
mod texture2d;

pub use asset_bundle::*;
pub use class_id::*;
pub use mesh::*;
pub use streaming_info::*;
pub use text_asset::*;
pub use texture2d::*;

/// Trait that allows reading Unity object data in a structured form.
pub trait UnityClass: Sized {
    /// Parses a tree into a structure.
    ///
    /// `tree` holds the necessary part of the tree to parse children.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if `Self` cannot be parsed from the given tree or the
    /// tree is invalid.
    fn parse_tree(
        r: &mut Cursor<&[u8]>,
        is_big_endian: bool,
        root: &TypeTreeNode,
        tree: &[TypeTreeNode],
    ) -> crate::Result<Self>;

    /// Tries to load a structure from an object reference.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if `Self` cannot be parsed from the tree type specified
    /// by the object or its tree is invalid.
    fn try_from_obj(obj: &ObjectRef<'_>) -> crate::Result<Self> {
        let cursor = &mut Cursor::new(obj.data()?);
        if let Some((root, tree)) = obj.ser_type.type_tree.split_first() {
            Self::parse_tree(cursor, obj.is_big_endian(), root, tree)
        } else {
            Err(Error::InvalidData("type tree is unexpectedly empty"))?
        }
    }

    /// Aligns the reader to the next 4-byte boundary.
    #[doc(hidden)]
    fn align_reader(r: &mut Cursor<&[u8]>) -> crate::Result<()> {
        Ok(r.align_to(4)?)
    }

    /// Skips the object the current reader is at.
    #[doc(hidden)]
    fn skip(
        r: &mut Cursor<&[u8]>,
        is_big_endian: bool,
        root: &TypeTreeNode,
        tree: &[TypeTreeNode],
    ) -> crate::Result<()> {
        if root.size >= 0 {
            r.seek(SeekFrom::Current(i64::from(root.size)))?;
        } else {
            match root.type_name.as_str() {
                "Array" | "TypelessData" => {
                    // The first element is the size, and the second is the child data.
                    // We assume that there cannot be siblings after that.
                    let size = u32::read_endian(r, is_big_endian)?;
                    let (next, children) = tree.get(1usize..).and_then(|o| o.split_first()).ok_or(
                        Error::InvalidData("skipped array type data does not contain data element"),
                    )?;

                    for _ in 0..size {
                        Self::skip(r, is_big_endian, next, children)?;
                    }
                },
                _ => {
                    let mut rest = tree;
                    while let Some((next, children, siblings)) = split_tree(rest) {
                        Self::skip(r, is_big_endian, next, children)?;
                        rest = siblings;
                    }
                },
            }
        }

        if root.needs_align_after() {
            Self::align_reader(r)?;
        }

        Ok(())
    }
}

/// Support trait to reduce code needed to implement [`UnityClass`].
///
/// If this trait is implemented, a blanket implementation will cover the actual
/// [`UnityClass`] implementation.
#[doc(hidden)]
pub trait AutoUnityClass: UnityClass + Default {
    /// The type name for this Unity class.
    const TYPE_NAME: &'static str;

    /// Like [`UnityClass::parse_tree`], but fills an existing object.
    fn parse_tree_into(
        &mut self,
        r: &mut Cursor<&[u8]>,
        is_big_endian: bool,
        tree: &[TypeTreeNode],
    ) -> crate::Result<()>;
}

impl<T: AutoUnityClass> UnityClass for T {
    // blanket implementation for `define_unity_class` generated types.
    fn parse_tree(
        r: &mut Cursor<&[u8]>,
        is_big_endian: bool,
        root: &TypeTreeNode,
        tree: &[TypeTreeNode],
    ) -> crate::Result<Self> {
        if root.type_name.as_str() != Self::TYPE_NAME {
            Err(Error::Mismatch {
                expected: Self::TYPE_NAME.to_owned(),
                received: root.type_name.clone(),
            })?
        }

        let mut result = Self::default();
        result.parse_tree_into(r, is_big_endian, tree)?;

        if root.needs_align_after() {
            Self::align_reader(r)?;
        }

        Ok(result)
    }
}

/// Splits the tree into:
///
/// - The next root node
/// - its children
/// - its siblings
///
/// If empty, returns [`None`].
#[must_use]
pub fn split_tree(
    tree: &[TypeTreeNode],
) -> Option<(&TypeTreeNode, &[TypeTreeNode], &[TypeTreeNode])> {
    let (next, other) = tree.split_first()?;

    let mut last_index = 0usize;
    for tree in other {
        if tree.level <= next.level {
            break;
        }

        last_index += 1;
    }

    let (children, siblings) = other.split_at(last_index);
    Some((next, children, siblings))
}

/// Defines a new structure that represents a Unity class.
///
/// The [`UnityClass`] implementation will skip unknown fields and leave ones
/// not found as default. If this needs to be known, wrap fields in an
/// [`Option`].
///
/// The resulting struct will additionally implement [`Default`], [`Clone`], and
/// [`Debug`](std::fmt::Debug). Subsequently, all fields must also implement
/// those traits.
///
/// Additional attributes added to the class or fields will also be added to the
/// resulting struct.
///
/// # Example
///
/// ```no_run
/// unity_read::define_unity_class! {
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
                $(#[$field_attr:meta])*
                $field_vis:vis $field_name:ident : $FieldType:ty = $key:literal
            ),* $(,)?
        }
    ) => {
        $(#[$attr])*
        #[derive(::std::fmt::Debug, ::std::clone::Clone, ::std::default::Default)]
        $v struct $Type {
            $(
                $(#[$field_attr])*
                $field_vis $field_name : $FieldType
            ),*
        }

        impl $crate::classes::AutoUnityClass for $Type {
            const TYPE_NAME: &'static str = $type_key;

            fn parse_tree_into(
                &mut self,
                r: &mut ::std::io::Cursor<&[u8]>,
                is_big_endian: bool,
                tree: &[$crate::serialized_file::TypeTreeNode],
            ) -> $crate::Result<()> {
                let mut rest = tree;
                while let Some((next, children, siblings)) = $crate::classes::split_tree(rest) {
                    match ::std::string::String::as_str(&next.name) {
                        $(
                            $key => self.$field_name = <$FieldType as $crate::classes::UnityClass>::parse_tree(
                                r,
                                is_big_endian,
                                next,
                                children,
                            )?,
                        )*
                        _ => <Self as $crate::classes::UnityClass>::skip(r, is_big_endian, next, children)?,
                    }

                    rest = siblings;
                }

                ::std::result::Result::Ok(())
            }
        }
    };
}

macro_rules! check_mismatch {
    ($root:expr, $expected:literal $(| $extra:literal)*) => {{
        match $root.type_name.as_str() {
            $expected $(| $extra)* => (),
            _ => Err(Error::Mismatch {
                expected: $expected.to_owned(),
                received: $root.type_name.clone(),
            })?
        }
    }};
}

impl UnityClass for String {
    fn parse_tree(
        r: &mut Cursor<&[u8]>,
        is_big_endian: bool,
        root: &TypeTreeNode,
        tree: &[TypeTreeNode],
    ) -> crate::Result<Self> {
        check_mismatch!(root, "string");

        // string should always have an Array of char nested
        let (next, children) = tree.split_first().ok_or(Error::InvalidData(
            "string type data does not contain children",
        ))?;

        let data = <Vec<u8>>::parse_tree(r, is_big_endian, next, children)?;

        Ok(Self::from_utf8(data)?)
    }
}

impl<T: UnityClass> UnityClass for Option<T> {
    fn parse_tree(
        r: &mut Cursor<&[u8]>,
        is_big_endian: bool,
        root: &TypeTreeNode,
        tree: &[TypeTreeNode],
    ) -> crate::Result<Self> {
        // Just delegates to the inner type and wraps it in Some
        // This is mostly intended for cases when the field might be missing in the data
        T::parse_tree(r, is_big_endian, root, tree).map(Some)
    }
}

impl<T: UnityClass> UnityClass for Vec<T> {
    fn parse_tree(
        r: &mut Cursor<&[u8]>,
        is_big_endian: bool,
        root: &TypeTreeNode,
        tree: &[TypeTreeNode],
    ) -> crate::Result<Self> {
        if matches!(root.type_name.as_str(), "vector" | "string") {
            let (next, children) = tree.split_first().ok_or(Error::InvalidData(
                "vector type data does not contain children",
            ))?;

            let result = Self::parse_tree(r, is_big_endian, next, children)?;

            if root.needs_align_after() {
                Self::align_reader(r)?;
            }

            return Ok(result);
        }

        check_mismatch!(root, "Array" | "TypelessData");

        // The first element is the size, and the second is the child data.
        // We assume that there cannot be siblings after that.
        let len = u32::read_endian(r, is_big_endian)?;
        let (next, children) =
            tree.get(1usize..)
                .and_then(|o| o.split_first())
                .ok_or(Error::InvalidData(
                    "array type data does not contain data element",
                ))?;

        let mut result = Self::new();
        for _ in 0..len {
            result.push(T::parse_tree(r, is_big_endian, next, children)?);
        }

        if root.needs_align_after() {
            Self::align_reader(r)?;
        }

        Ok(result)
    }
}

macro_rules! impl_unity_class_primitive {
    ($Type:ty, $expected:literal $(| $extra:literal)*) => {
        impl UnityClass for $Type {
            fn parse_tree(r: &mut Cursor<&[u8]>, is_big_endian: bool, root: &TypeTreeNode, _tree: &[TypeTreeNode]) -> crate::Result<Self> {
                check_mismatch!(root, $expected $(| $extra)*);

                let value = <$Type>::read_endian(r, is_big_endian)?;
                if (root.meta_flags & 0x4000) != 0 {
                    Self::align_reader(r)?;
                }

                Ok(value)
            }
        }
    };
}

impl_unity_class_primitive!(i8, "SInt8");
impl_unity_class_primitive!(u8, "UInt8" | "char");
impl_unity_class_primitive!(i16, "SInt16" | "short");
impl_unity_class_primitive!(u16, "UInt16" | "unsigned short");
impl_unity_class_primitive!(i32, "SInt32" | "int");
impl_unity_class_primitive!(u32, "UInt32" | "unsigned int" | "Type*");
impl_unity_class_primitive!(i64, "SInt64" | "long long");
impl_unity_class_primitive!(u64, "UInt64" | "unsigned long long" | "FileSize");
impl_unity_class_primitive!(f32, "float");
impl_unity_class_primitive!(f64, "double");
