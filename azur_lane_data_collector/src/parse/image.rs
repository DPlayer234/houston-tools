use std::io::Cursor;

use image::{imageops, ImageOutputFormat};
use unity_read::classes::{ClassID, Texture2D};
use unity_read::unity_fs::{UnityFsData, UnityFsFile};

// shipmodels: chibi sprites, 1:1
// paintingface: alternative faces, 0/1:1
// painting:
// - tex: full sprite, background 1:1
// - n_tex: full sprite, no background 0/1:1

pub fn load_chibi_image(dir: &str, name: &str) -> anyhow::Result<Option<Vec<u8>>> {
    let name = name.to_ascii_lowercase();
    let Ok(mut file) = std::fs::File::open(utils::join_path!(dir, "shipmodels", &name)) else {
        println!("Skin shipmodels file {name} not found.");
        return Ok(None)
    };

    let unity_fs = UnityFsFile::open(&mut file)?;
    for entry in unity_fs.entries() {
        if let UnityFsData::SerializedFile(ser_file) = entry.read()? {
            let texture = ser_file.objects()
                .filter(|o| o.class_id() == ClassID::Texture2D)
                .filter_map(|o| o.try_into_class::<Texture2D>().ok())
                .find(|t| t.name.to_ascii_lowercase() == name);

            if let Some(texture) = texture {
                let mut image = texture.read_data(&unity_fs)?.decode()?;
                imageops::flip_vertical_in_place(&mut image);

                let mut writer = Cursor::new(Vec::new());
                image.write_to(&mut writer, ImageOutputFormat::WebP)?;
                return Ok(Some(writer.into_inner()))
            }
        }
    }

    println!("Skin shipmodels image {name} not present.");
    return Ok(None)
}
