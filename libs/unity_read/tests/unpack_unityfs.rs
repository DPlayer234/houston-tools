#![allow(unused_crate_dependencies)]
use std::io::Cursor;

use unity_read::classes::{ClassID, Texture2D, TextureFormat};
use unity_read::unity_fs::{UnityFsData, UnityFsFile};

const WIDTH: i32 = 144;
const HEIGHT: i32 = 152;
const UNITY_FS: &[u8] = include_bytes!("assets/abeikelongbi");
const ETC2_RGBA8: &[u8] = include_bytes!("assets/abeikelongbi.ETC2_RGBA8.bin");
const RGBA32: &[u8] = include_bytes!("assets/abeikelongbi.RGBA32.bin");

#[test]
fn unpack_unityfs() {
    let mut buf = Cursor::new(UNITY_FS);
    let unity_fs = UnityFsFile::open(&mut buf).expect("valid unityfs archive");

    let mut image = None::<Texture2D>;

    // this archive only has one SerializedFile and one Texture2D, so this search
    // should be good enough
    for entry in unity_fs.entries() {
        if let UnityFsData::SerializedFile(ser_file) = entry.read().expect("entry must be readable")
        {
            let texture = ser_file
                .objects()
                .filter_map(Result::ok)
                .filter(|o| o.class_id() == ClassID::Texture2D)
                .filter_map(|o| o.try_into_class::<Texture2D>().ok())
                .find(|t| t.name == "abeikelongbi");

            image = texture;
            break;
        }
    }

    let image = image.expect("must have found texture");
    let data = image
        .read_data(&unity_fs)
        .expect("image data must be valid");

    assert_eq!(data.data(), ETC2_RGBA8, "must be the same image data");
}

#[test]
fn decode_etc2_rgba8() {
    let texture = Texture2D {
        width: WIDTH,
        height: HEIGHT,
        format: TextureFormat::ETC2_RGBA8 as i32,
        image_data: ETC2_RGBA8.to_vec(),
        ..Texture2D::default()
    };

    let texture_data = texture.as_data().expect("we set no stream data");
    let image = texture_data.decode().expect("provided valid texture data");

    assert_eq!(
        image.as_raw(),
        RGBA32,
        "decoded data must match RGBA32 data"
    );
}
