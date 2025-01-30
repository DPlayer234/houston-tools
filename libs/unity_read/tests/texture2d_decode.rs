#![allow(unused_crate_dependencies)]
use unity_read::classes::{Texture2D, TextureFormat};

const WIDTH: i32 = 144;
const HEIGHT: i32 = 152;
const ETC2_RGBA8: &[u8] = include_bytes!("assets/abeikelongbi.ETC2_RGBA8.bin");
const RGBA32: &[u8] = include_bytes!("assets/abeikelongbi.RGBA32.bin");

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
