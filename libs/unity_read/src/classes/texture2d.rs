use image::RgbaImage;
use num_enum::FromPrimitive;

use super::StreamingInfo;
use crate::error::Error;
use crate::unity_fs::UnityFsFile;
use crate::{define_unity_class, FromInt};

define_unity_class! {
    /// Data for Unity's Texture2D class.
    pub class Texture2D = "Texture2D" {
        pub name: String = "m_Name",
        pub width: i32 = "m_Width",
        pub height: i32 = "m_Height",
        pub format: i32 = "m_TextureFormat",
        pub image_data: Vec<u8> = "image data",
        pub stream_data: StreamingInfo = "m_StreamData",
    }
}

/// Loaded data for a [`Texture2D`].
#[derive(Debug, Clone)]
pub struct Texture2DData<'t> {
    texture: &'t Texture2D,
    data: &'t [u8],
}

impl Texture2D {
    /// Gets the texture format.
    pub fn format(&self) -> TextureFormat {
        TextureFormat::from_primitive(self.format)
    }

    /// Reads the texture data.
    pub fn read_data<'t, 'fs: 't>(
        &'t self,
        fs: &'fs UnityFsFile<'fs>,
    ) -> crate::Result<Texture2DData<'t>> {
        Ok(Texture2DData {
            texture: self,
            data: self
                .stream_data
                .load_data_or_else(fs, || &self.image_data)?,
        })
    }

    /// Gets the data for this texture if it's not specified through stream
    /// data. In general, you should use [`Self::read_data`] instead.
    ///
    /// If `stream_data` is set, this function returns an [`Err`].
    #[doc(hidden)]
    pub fn as_data(&self) -> crate::Result<Texture2DData<'_>> {
        if self.stream_data.is_empty() {
            Ok(Texture2DData {
                texture: self,
                data: &self.image_data,
            })
        } else {
            Err(Error::InvalidData(
                "cannot use `to_data` when `stream_data` is set. use `read_data` instead.",
            ))
        }
    }
}

impl Texture2DData<'_> {
    /// Gets the block of data.
    #[must_use]
    pub fn data(&self) -> &[u8] {
        self.data
    }

    /// Decodes the image data.
    pub fn decode(&self) -> crate::Result<RgbaImage> {
        let width = u32::from_int(self.texture.width)?;
        let height = u32::from_int(self.texture.height)?;

        let args = Args::new(width, height)?;
        match self.texture.format() {
            TextureFormat::RGBA32 => {
                // this matches the Rgba<u8> layout
                let image = RgbaImage::from_raw(width, height, self.data.to_vec())
                    .ok_or(Error::InvalidData("image data size incorrect"))?;

                Ok(image)
            },
            TextureFormat::ETC2_RGBA8 => args.decode_with(|args, buf| {
                texture2ddecoder::decode_etc2_rgba8(self.data, args.width, args.height, buf)
                    .map_err(Error::InvalidData)
            }),
            TextureFormat::ASTC_RGB_6x6 => args.decode_with(|args, buf| {
                texture2ddecoder::decode_astc_6_6(self.data, args.width, args.height, buf)
                    .map_err(Error::InvalidData)
            }),
            _ => Err(Error::Unsupported(format!(
                "texture format not implemented: {:?}",
                self.texture.format()
            )))?,
        }
    }
}

/// Stores validated image arguments.
struct Args {
    width: usize,
    height: usize,
    size: usize,
}

impl Args {
    /// Creates a new [`Args`], validating the width, height, and total size.
    fn new(width: u32, height: u32) -> crate::Result<Self> {
        let width = usize::from_int(width)?;
        let height = usize::from_int(height)?;
        let size = width
            .checked_mul(height)
            .and_then(|s| s.checked_mul(size_of::<u32>()))
            .filter(|s| isize::try_from(*s).is_ok())
            .ok_or(Error::InvalidData("image size overflows address space"))?;

        Ok(Self {
            width,
            height,
            size,
        })
    }

    /// Decodes the image with a given decoder function.
    fn decode_with<F>(self, decode: F) -> crate::Result<RgbaImage>
    where
        F: FnOnce(&Self, &mut [u32]) -> Result<(), Error>,
    {
        // allocate buffer as Vec<u8> since that's the final data type needed
        // the size has been multiplied by 4 already to match the pixel width
        let mut buffer = vec![0u8; self.size];
        let mut buffer_u32 = None;

        // cast the vec's slice to u32.
        // while this can't fail for the obvious reason (this size of a multiple of 4),
        // it could fail because the allocation isn't sufficiently aligned.
        // no system allocator (at least on expected platforms) actually allocates
        // with an alignment of less than 8, but we may as well handle it.
        // to do that, we allocate a new buffer of u32s and copy it back later.
        let slice_u32 = match bytemuck::try_cast_slice_mut::<u8, u32>(&mut buffer) {
            Ok(b) => b,
            _ => buffer_u32.insert(vec![0u32; buffer.len() / size_of::<u32>()]),
        };

        decode(&self, slice_u32)?;

        // fix the color output to match RGBA32, so `[R, G, B, A]` bytes.
        //
        // `texture2ddecoder` has somewhat weird "endianness" for the output.
        // technically, it claims to output BGRA, however this is actually loaded as a
        // little-endian byte array into a native u32 via `u32::from_le_bytes`. the
        // result is that the _numeric value_ is endian independent and always has the
        // form of `0xAARRGGBB`, but the byte-wise result differs between architectures,
        // and that's what we actually care about.
        for px in slice_u32 {
            if cfg!(target_endian = "little") {
                // for little-endian, the bytes are `[B, G, R, A]`, so, we need to swap the
                // green and red channels.
                *px = (*px & 0xFF00_FF00) | ((*px & 0xFF_0000) >> 16) | ((*px & 0xFF) << 16);
            } else {
                // for big-endian, the bytes are `[A, R, G, B]`. a rotate-left by 1 byte happens
                // to put the bytes into the correct spots.
                *px = px.rotate_left(8);
            }
        }

        if let Some(buffer_u32) = buffer_u32 {
            buffer.copy_from_slice(bytemuck::cast_slice::<u32, u8>(&buffer_u32));
        }

        // truncation is fine -- source values were u32 already
        #[allow(clippy::cast_possible_truncation)]
        let image = RgbaImage::from_raw(self.width as u32, self.height as u32, buffer)
            .expect("buffer should be allocated with the correct size");
        Ok(image)
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
