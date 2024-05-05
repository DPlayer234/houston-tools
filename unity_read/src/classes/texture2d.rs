use std::borrow::Cow;

use num_enum::FromPrimitive;
use image::*;

use crate::{define_unity_class, UnityError};
use crate::unity_fs::UnityFsFile;
use super::StreamingInfo;

define_unity_class! {
    /// Data for Unity's Texture2D class.
    pub class Texture2D = "Texture2D" {
        pub name: String = "m_Name",
        pub width: i32 = "m_Width",
        pub height: i32 = "m_Height",
        format: i32 = "m_TextureFormat",
        image_data: Vec<u8> = "image data",
        stream_data: StreamingInfo = "m_StreamData",
    }
}

/// Loaded data for a [`Texture2D`].
#[derive(Debug, Clone)]
pub struct Texture2DData<'a> {
    texture: &'a Texture2D,
    data: Cow<'a, [u8]>
}

impl Texture2D {
    /// Gets the texture format.
    pub fn format(&self) -> TextureFormat {
        TextureFormat::from_primitive(self.format)
    }

    /// Reads the texture data.
    pub fn read_data<'a>(&'a self, fs: &UnityFsFile) -> anyhow::Result<Texture2DData<'a>> {
        let data = if self.stream_data.is_empty() {
            Cow::Borrowed(self.image_data.as_slice())
        } else {
            Cow::Owned(self.stream_data.load_data(fs)?)
        };

        Ok(Texture2DData {
            texture: self,
            data
        })
    }
}

impl Texture2DData<'_> {
    /// Gets the block of data.
    pub fn data(&self) -> &[u8] {
        &self.data
    }

    /// Decodes the image data.
    pub fn decode(&self) -> anyhow::Result<RgbaImage> {
        let width: u32 = self.texture.width.try_into()?;
        let height: u32 = self.texture.height.try_into()?;

        fn as_bytes<T>(v: &[T]) -> Vec<u8> {
            // NOTE: You cannot construct Vecs from the raw data of another.
            // That is because the allocator allocates blocks using a certain SIZE AND LAYOUT.

            let ptr = v.as_ptr().cast::<u8>();
            let byte_len = v.len() * std::mem::size_of::<T>() / std::mem::size_of::<u8>();
            unsafe { std::slice::from_raw_parts(ptr, byte_len) }.to_vec()
        }

        match self.texture.format() {
            TextureFormat::RGBA32 => {
                // I think this matches the Rgba<u8> layout?
                let image = RgbaImage::from_raw(width.try_into()?, height.try_into()?, self.data.to_vec())
                    .ok_or(UnityError::InvalidData("image data size incorrect"))?;

                Ok(image)
            },
            TextureFormat::ETC2_RGBA8 => {
                let mut buffer = vec![0u32; (width * height) as usize];
                texture2ddecoder::decode_etc2_rgba8(&self.data, width as usize, height as usize, buffer.as_mut_slice()).map_err(UnityError::InvalidData)?;

                // Swap red and green channels
                #[cfg(target_endian = "little")]
                for px in buffer.iter_mut() {
                    *px = (*px & 0xFF00_FF00) | ((*px & 0xFF_0000) >> 16) | ((*px & 0xFF) << 16);
                }

                #[cfg(target_endian = "big")]
                for px in buffer.iter_mut() {
                    *px = (*px & 0x00_FF00FF) | ((*px & 0xFF00_0000) >> 16) | ((*px & 0xFF00) << 16);
                }

                let image = RgbaImage::from_raw(width, height, as_bytes(&buffer)).unwrap();
                Ok(image)
            },
            _ => todo!("texture format {:?} not implemented", self.texture.format())
        }
    }
}

/// Well-known texture 2D formats.
#[allow(non_camel_case_types, non_upper_case_globals)]
#[derive(Debug, Eq, PartialEq, FromPrimitive, Clone, Copy, Default, Hash)]
#[repr(i32)]
#[non_exhaustive]
pub enum TextureFormat {
    #[default]
    UnknownType = -1,
    Alpha8 = 1,
    ARGB4444,
    RGB24,
    RGBA32,
    ARGB32,
    RGB565 = 7,
    R16 = 9,
    DXT1,
    DXT5 = 12,
    RGBA4444,
    BGRA32,
    RHalf,
    RGHalf,
    RGBAHalf,
    RFloat,
    RGFloat,
    RGBAFloat,
    YUY2,
    RGB9e5Float,
    BC4 = 26,
    BC5,
    BC6H = 24,
    BC7,
    DXT1Crunched = 28,
    DXT5Crunched,
    PVRTC_RGB2,
    PVRTC_RGBA2,
    PVRTC_RGB4,
    PVRTC_RGBA4,
    ETC_RGB4,
    ATC_RGB4,
    ATC_RGBA8,
    EAC_R = 41,
    EAC_R_SIGNED,
    EAC_RG,
    EAC_RG_SIGNED,
    ETC2_RGB,
    ETC2_RGBA1,
    ETC2_RGBA8,
    ASTC_RGB_4x4,
    ASTC_RGB_5x5,
    ASTC_RGB_6x6,
    ASTC_RGB_8x8,
    ASTC_RGB_10x10,
    ASTC_RGB_12x12,
    ASTC_RGBA_4x4,
    ASTC_RGBA_5x5,
    ASTC_RGBA_6x6,
    ASTC_RGBA_8x8,
    ASTC_RGBA_10x10,
    ASTC_RGBA_12x12,
    ETC_RGB4_3DS,
    ETC_RGBA8_3DS,
    RG16,
    R8,
    ETC_RGB4Crunched,
    ETC2_RGBA8Crunched,
    ASTC_HDR_4x4,
    ASTC_HDR_5x5,
    ASTC_HDR_6x6,
    ASTC_HDR_8x8,
    ASTC_HDR_10x10,
    ASTC_HDR_12x12,
}
