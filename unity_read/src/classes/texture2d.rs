use num_enum::FromPrimitive;
use image::RgbaImage;

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
pub struct Texture2DData<'t> {
    texture: &'t Texture2D,
    data: &'t [u8]
}

impl Texture2D {
    /// Gets the texture format.
    pub fn format(&self) -> TextureFormat {
        TextureFormat::from_primitive(self.format)
    }

    /// Reads the texture data.
    pub fn read_data<'t, 'fs: 't>(&'t self, fs: &'fs UnityFsFile<'fs>) -> anyhow::Result<Texture2DData<'t>> {
        Ok(Texture2DData {
            texture: self,
            data: self.stream_data.load_data_or_else(fs, || &self.image_data)?
        })
    }
}

impl Texture2DData<'_> {
    /// Gets the block of data.
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Decodes the image data.
    pub fn decode(&self) -> anyhow::Result<RgbaImage> {
        let width = u32::try_from(self.texture.width)?;
        let height = u32::try_from(self.texture.height)?;

        match self.texture.format() {
            TextureFormat::RGBA32 => {
                // this matches the Rgba<u8> layout
                let image = RgbaImage::from_raw(width, height, self.data.to_vec())
                    .ok_or(UnityError::InvalidData("image data size incorrect"))?;

                Ok(image)
            },
            TextureFormat::ETC2_RGBA8 => {
                let width_s = usize::try_from(width)?;
                let height_s = usize::try_from(height)?;
                let size = width_s.checked_mul(height_s)
                    .and_then(|s| s.checked_mul(size_of::<u32>()))
                    .ok_or(UnityError::InvalidData("image size overflows address space"))?;

                // allocate buffer as Vec<u8> since that's the final data type needed
                // the size has been multiplied by 4 already to match the pixel width
                let mut buffer = vec![0u8; size];

                // cast the vec's slice to u32. this can't fail since the buffer's size is a multiple of 4.
                // following that, try to decode the image data
                let buffer_u32 = bytemuck::cast_slice_mut::<u8, u32>(&mut buffer);
                texture2ddecoder::decode_etc2_rgba8(self.data, width_s, height_s, buffer_u32)
                    .map_err(UnityError::InvalidData)?;

                // Swap red and green channels
                for px in buffer_u32 {
                    if cfg!(target_endian = "little") {
                        *px = (*px & 0xFF00_FF00) | ((*px & 0xFF_0000) >> 16) | ((*px & 0xFF) << 16);
                    } else {
                        *px = (*px & 0x00_FF00FF) | ((*px & 0xFF00_0000) >> 16) | ((*px & 0xFF00) << 16);
                    }
                }

                let image = RgbaImage::from_raw(width, height, buffer)
                    .expect("buffer allocated with the correct size");
                Ok(image)
            },
            _ => Err(UnityError::Unsupported(
                format!("texture format not implemented: {:?}", self.texture.format())
            ))?,
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
