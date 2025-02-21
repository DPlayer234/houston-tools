use std::io::{Cursor, Read, Seek};
use std::mem::swap;

use binrw::{BinRead, BinResult, Endian};
use half::f16;

use super::StreamingInfo;
use crate::error::Error;
use crate::unity_fs::UnityFsFile;
use crate::{FromInt, define_unity_class};

define_unity_class! {
    pub class Mesh = "Mesh" {
        pub name: String = "m_Name",
        pub sub_meshes: Vec<SubMesh> = "m_SubMeshes",
        pub index_format: i32 = "m_IndexFormat",
        pub index_buffer: Vec<u8> = "m_IndexBuffer",
        pub vertex_data: VertexData = "m_VertexData",
        pub local_aabb: AABB = "m_LocalAABB",
        pub stream_data: StreamingInfo = "m_StreamData",
    }
}

define_unity_class! {
    pub class SubMesh = "SubMesh" {
        pub first_byte: u32 = "firstByte",
        pub index_count: u32 = "indexCount",
        pub topology: i32 = "topology",
        pub base_vertex: u32 = "baseVertex",
        pub first_vertex: u32 = "firstVertex",
        pub vertex_count: u32 = "vertexCount",
        pub local_aabb: AABB = "localAABB",
    }
}

define_unity_class! {
    pub class VertexData = "VertexData" {
        pub vertex_count: u32 = "m_VertexCount",
        pub channels: Vec<ChannelInfo> = "m_Channels",
        pub data_size: Vec<u8> = "m_DataSize",
        pub streams: Option<Vec<StreamInfo>> = "m_Streams",
    }
}

define_unity_class! {
    pub class StreamInfo = "StreamInfo" {
        pub channel_mask: u32 = "channelMask",
        pub offset: u32 = "offset",
        pub stride: u32 = "stride",
        pub divider_op: u32 = "dividerOp",
        pub frequency: u32 = "frequency",
    }
}

define_unity_class! {
    pub class ChannelInfo = "ChannelInfo" {
        pub stream: u8 = "stream",
        pub offset: u8 = "offset",
        pub format: u8 = "format",
        pub dimension: u8 = "dimension",
    }
}

define_unity_class! {
    pub class Vector3f = "Vector3f" {
        pub x: f32 = "x",
        pub y: f32 = "y",
        pub z: f32 = "z",
    }
}

define_unity_class! {
    pub class AABB = "AABB" {
        pub center: Vector3f = "m_Center",
        pub extent: Vector3f = "m_Extent",
    }
}

#[derive(Debug, Clone, Default)]
pub struct Vertex {
    pub pos: Vector3f,
    pub uv: Vector3f,
}

#[derive(Debug, Clone, Default)]
pub struct ResolvedMesh {
    vertices: Vec<Vertex>,
    triangle_data: Vec<(usize, usize, usize)>,
}

/// Loaded vertex data for a [`Mesh`].
#[derive(Debug, Clone)]
pub struct MeshVertexData<'t> {
    mesh: &'t Mesh,
    data: &'t [u8],
}

impl Mesh {
    /// Reads the mesh's vertex data.
    pub fn read_vertex_data<'t, 'fs: 't>(
        &'t self,
        fs: &'fs UnityFsFile<'fs>,
    ) -> crate::Result<MeshVertexData<'t>> {
        Ok(MeshVertexData {
            mesh: self,
            data: self
                .stream_data
                .load_data_or_else(fs, || &self.vertex_data.data_size)?,
        })
    }
}

impl MeshVertexData<'_> {
    // Only assuming Unity 2018 and newer.
    pub fn resolve_meshes(&self) -> crate::Result<Vec<ResolvedMesh>> {
        // Would you believe me if this handles barely anything a mesh can store?
        let (index_size, index_buffer) = self.load_index_buffer()?;
        let streams = self.load_streams()?;

        let mut result_meshes = Vec::new();

        for sub_mesh in &self.mesh.sub_meshes {
            let mut result = ResolvedMesh {
                vertices: vec![Vertex::default(); usize::from_int(sub_mesh.vertex_count)?],
                ..Default::default()
            };

            for (index, channel) in self.mesh.vertex_data.channels.iter().enumerate() {
                #[allow(clippy::manual_range_patterns)]
                if !matches!(channel.dimension, 1 | 2 | 3) {
                    continue;
                }

                // CMBK: currently only supporting some channels
                if !matches!(index, 0 | 3 | 4) {
                    continue;
                }

                let Some(stream) = streams.get(usize::from(channel.stream)) else {
                    continue;
                };

                let channel_size = channel.stride();
                let stream_size = u64::from(stream.stride) * u64::from(sub_mesh.vertex_count)
                    + u64::from(stream.offset);

                if channel_size > stream.stride || stream_size > u64::from_int(self.data.len())? {
                    continue;
                }

                // assert that the loops below can always cast `i as usize`
                _ = usize::from_int(sub_mesh.vertex_count)?;

                match index {
                    0 => {
                        // pos
                        if channel.dimension != 3 {
                            continue;
                        }
                        for i in 0..sub_mesh.vertex_count {
                            let cursor = &mut make_cursor(self.data, i, sub_mesh, stream, channel);
                            result.vertices[i as usize].pos =
                                read_f32_vector::<3>(cursor, channel.format)?.into();
                        }
                    },
                    3 | 4 => {
                        // uv1/2
                        for i in 0..sub_mesh.vertex_count {
                            let cursor = &mut make_cursor(self.data, i, sub_mesh, stream, channel);
                            let uv = &mut result.vertices[i as usize].uv;
                            match channel.dimension {
                                1 => *uv = read_f32_vector::<1>(cursor, channel.format)?.into(),
                                2 => *uv = read_f32_vector::<2>(cursor, channel.format)?.into(),
                                3 => *uv = read_f32_vector::<3>(cursor, channel.format)?.into(),
                                _ => unreachable!(),
                            }
                        }
                    },
                    _ => unreachable!(),
                }
            }

            // Revisit if x of vertices/normals needs to be inverted

            let index_offset = sub_mesh.first_byte / index_size;
            let mut index_iter = index_buffer
                .iter()
                .skip(usize::from_int(index_offset)?)
                .take(usize::from_int(sub_mesh.index_count)?);

            // This is only used to switch triangle winding
            let mut topology_offset = index_offset % 2u32;

            while let Some(&vertex_index_0) = index_iter.next() {
                let Some(&vertex_index_1) = index_iter.next() else {
                    break;
                };

                let Some(&vertex_index_2) = index_iter.next() else {
                    break;
                };

                let mut triangle = (
                    usize::from_int(vertex_index_0 + sub_mesh.base_vertex - sub_mesh.first_vertex)?,
                    usize::from_int(vertex_index_1 + sub_mesh.base_vertex - sub_mesh.first_vertex)?,
                    usize::from_int(vertex_index_2 + sub_mesh.base_vertex - sub_mesh.first_vertex)?,
                );

                if sub_mesh.topology != 0 && (topology_offset & 1) != 0 {
                    swap(&mut triangle.0, &mut triangle.2);
                }

                topology_offset += 1;
                result.triangle_data.push(triangle);
            }

            result_meshes.push(result);
        }

        return Ok(result_meshes);

        fn make_cursor<'a>(
            data: &'a [u8],
            k: u32,
            sub_mesh: &SubMesh,
            stream: &StreamInfo,
            channel: &ChannelInfo,
        ) -> Cursor<&'a [u8]> {
            let data_offset = u64::from(stream.offset)
                + u64::from(sub_mesh.first_vertex + k) * u64::from(stream.stride)
                + u64::from(channel.offset);

            let mut cursor = Cursor::new(data);
            cursor.set_position(data_offset);
            cursor
        }

        fn read_f32_vector<const N: usize>(
            cursor: &mut Cursor<&[u8]>,
            t: u8,
        ) -> crate::Result<[f32; N]> {
            match t {
                0 => read_vector_of::<f32, N>(cursor),
                1 => read_vector_of::<ReadF16, N>(cursor).map(NormFloat::to_f32_array),
                2 | 3 => read_vector_of::<Norm<u8>, N>(cursor).map(NormFloat::to_f32_array),
                4 => read_vector_of::<Norm<i8>, N>(cursor).map(NormFloat::to_f32_array),
                5 => read_vector_of::<Norm<u16>, N>(cursor).map(NormFloat::to_f32_array),
                6 => read_vector_of::<Norm<i16>, N>(cursor).map(NormFloat::to_f32_array),
                _ => Err(Error::Unsupported(format!(
                    "unsupported mesh data type: {t}"
                )))?,
            }
        }

        fn read_vector_of<T, const N: usize>(cursor: &mut Cursor<&[u8]>) -> crate::Result<[T; N]>
        where
            T: Copy + Default + BinRead,
            for<'a> T::Args<'a>: Default,
        {
            let mut res = [T::default(); N];
            for f in &mut res {
                *f = T::read_le(cursor)?;
            }

            Ok(res)
        }
    }

    fn load_index_buffer(&self) -> crate::Result<(u32, Vec<u32>)> {
        macro_rules! map_buffer {
            ($Ty:ty) => {{
                const N: usize = size_of::<$Ty>();
                let vec = self
                    .mesh
                    .index_buffer
                    .chunks_exact(N)
                    .map(|chunk| {
                        // cannot fail since chunk size == N
                        let chunk = <[u8; N]>::try_from(chunk).unwrap();
                        <$Ty>::from_le_bytes(chunk).into()
                    })
                    .collect();

                #[allow(clippy::cast_possible_truncation)]
                (const { N as u32 }, vec)
            }};
        }

        match self.mesh.index_format {
            0 => Ok(map_buffer!(u16)), // UInt16
            1 => Ok(map_buffer!(u32)), // UInt32
            _ => Err(Error::InvalidData("unexpected mesh index format")),
        }
    }

    fn load_streams(&self) -> crate::Result<Vec<StreamInfo>> {
        let data_size = u32::from_int(self.data.len())?;
        let vertex_data = &self.mesh.vertex_data;

        let mut streams = vertex_data.streams.clone().unwrap_or_default();

        let max_stream = vertex_data
            .channels
            .iter()
            .map(|c| c.stream)
            .max()
            .unwrap_or_default();
        let max_stream = usize::from(max_stream);

        while streams.len() <= max_stream {
            streams.push(StreamInfo::default());
        }

        if vertex_data.streams.is_none() {
            for (index, channel) in vertex_data.channels.iter().enumerate() {
                let stream = &mut streams[usize::from(channel.stream)];

                stream.channel_mask |= 1u32 << index;

                let cur_size = channel.stride();
                if cur_size > stream.stride {
                    stream.stride = cur_size;
                }
            }

            let mut cur_offset = 0u32;
            for stream in &mut streams {
                stream.offset = cur_offset;
                cur_offset += stream.stride * vertex_data.vertex_count;
            }

            if cur_offset > data_size {
                Err(Error::InvalidData(
                    "mesh channel info specified too much stream data",
                ))?;
            }

            if streams.len() == 2 {
                streams[1].offset = data_size - streams[1].stride * vertex_data.vertex_count;
            }
        }

        Ok(streams)
    }
}

impl ChannelInfo {
    fn stride(&self) -> u32 {
        u32::from(self.offset) + u32::from(self.dimension) * u32::from(self.element_size())
    }

    fn element_size(&self) -> u8 {
        /* Copied from UABE:
        0  : Float32; 1  : Float16; 2  : UNorm8; 3  : UNorm8; 4  : SNorm8; 5  : UNorm16; 6  : SNorm16;
        7  : UInt8;   8  : SInt8;   9  : UInt16; 10 : SInt16; 11 : UInt32; 12 : SInt32;
        */
        const FORMATS: [u8; 13] = [4, 2, 1, 1, 1, 2, 2, 1, 1, 2, 2, 4, 4];

        FORMATS
            .get(usize::from(self.format))
            .copied()
            .unwrap_or_default()
    }
}

impl ResolvedMesh {
    pub fn vertices(&self) -> &[Vertex] {
        &self.vertices
    }

    pub fn triangles(&self) -> impl ExactSizeIterator<Item = (&Vertex, &Vertex, &Vertex)> {
        self.triangle_data.iter().map(|t| {
            (
                &self.vertices[t.0],
                &self.vertices[t.1],
                &self.vertices[t.2],
            )
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Norm<T>(T);

trait NormFloat: Sized {
    fn to_f32(self) -> f32;

    fn to_f32_array<const N: usize>(arr: [Self; N]) -> [f32; N] {
        arr.map(Self::to_f32)
    }
}

impl NormFloat for Norm<u8> {
    fn to_f32(self) -> f32 {
        f32::from(self.0) / f32::from(u8::MAX)
    }
}

impl NormFloat for Norm<u16> {
    fn to_f32(self) -> f32 {
        f32::from(self.0) / f32::from(u16::MAX)
    }
}

impl NormFloat for Norm<i8> {
    fn to_f32(self) -> f32 {
        if self.0 == i8::MIN {
            -1.0f32
        } else {
            f32::from(self.0) / f32::from(i8::MAX)
        }
    }
}

impl NormFloat for Norm<i16> {
    fn to_f32(self) -> f32 {
        if self.0 == i16::MIN {
            -1.0f32
        } else {
            f32::from(self.0) / f32::from(i16::MAX)
        }
    }
}

impl<T: BinRead> BinRead for Norm<T> {
    type Args<'a> = T::Args<'a>;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        Ok(Self(T::read_options(reader, endian, args)?))
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ReadF16(f16);

impl NormFloat for ReadF16 {
    fn to_f32(self) -> f32 {
        f16::to_f32(self.0)
    }
}

impl BinRead for ReadF16 {
    type Args<'a> = <u16 as BinRead>::Args<'a>;

    fn read_options<R: Read + Seek>(
        reader: &mut R,
        endian: Endian,
        args: Self::Args<'_>,
    ) -> BinResult<Self> {
        u16::read_options(reader, endian, args).map(|b| Self(f16::from_bits(b)))
    }
}

impl From<[f32; 1]> for Vector3f {
    fn from(value: [f32; 1]) -> Self {
        Self {
            x: value[0],
            y: 0f32,
            z: 0f32,
        }
    }
}

impl From<[f32; 2]> for Vector3f {
    fn from(value: [f32; 2]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: 0f32,
        }
    }
}

impl From<[f32; 3]> for Vector3f {
    fn from(value: [f32; 3]) -> Self {
        Self {
            x: value[0],
            y: value[1],
            z: value[2],
        }
    }
}
