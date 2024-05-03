use std::io::Cursor;
use std::path::Path;

use image::ImageOutputFormat;
use unity_rs::{ClassID, Env};
use unity_rs::classes::Texture2D;

// shipmodels: chibi sprites, 1:1
// paintingface: alternative faces, 0/1:1
// painting:
// - tex: full sprite, background 1:1
// - n_tex: full sprite, no background 0/1:1

#[must_use]
pub fn load_chibi_image(dir: &str, name: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let name = name.to_ascii_lowercase();
    let Ok(file) = std::fs::read(Path::new(dir).join("shipmodels").join(&name)) else {
        println!("Skin shipmodels file {name} not found.");
        return Ok(None)
    };

    let mut env = Env::new();
    env.load_from_slice(&file)?;

    let texture = env.objects()
        .filter(|o| o.class() == ClassID::Texture2D)
        .filter_map(|o| o.read::<Texture2D>().ok())
        .find(|t| t.name.to_ascii_lowercase() == name);

    if let Some(texture) = texture {
        let image = texture.decode_image_without_cache()?;

        let mut writer = Cursor::new(Vec::new());
        image.write_to(&mut writer, ImageOutputFormat::WebP)?;
        Ok(Some(writer.into_inner()))
    } else {
        println!("Skin shipmodels image {name} not present.");
        Ok(None)
    }
}
