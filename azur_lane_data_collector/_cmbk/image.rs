// Failed attempt at stitching full sprites together.
// The following things should be noted should I come back to this:
//
// - There is a chance the mesh is in a different archive.
// - There is a chance parts of a sprite need to be resized to fit the indicated area.
// - Foreground sprites do not always contain the ship.
// - Faces may be missing.
//
// For this reason, and because it's beyond the scope of what I need, I will not bother for now.

use std::fs;
use std::io::Cursor;
use std::path::Path;

use image::{imageops, GenericImage, ImageOutputFormat, RgbaImage, SubImage};
use unity_read::classes::{ClassID, Mesh, ResolvedMesh, Texture2D};
use unity_read::unity_fs::{UnityFsData, UnityFsFile};
use unity_read::UnityError;

// shipmodels: chibi sprites, 1:1
// paintingface: alternative faces, 0/1:1
// painting:
// - tex: full sprite, background 1:1
// - n_tex: full sprite, no background 0/1:1

#[must_use]
pub fn extract_stitched_image(path: &Path) -> anyhow::Result<Option<Vec<u8>>> {
    let Ok(file) = fs::read(path) else {
        return Ok(None)
    };

    let unity_fs = UnityFsFile::read(file)?;

    let mut texture: Option<Texture2D> = None;
    let mut mesh: Option<Mesh> = None;

    for node in unity_fs.entries() {
        if let UnityFsData::SerializedFile(ser) = node.read()? {
            for obj in ser.objects() {
                match obj.class_id() {
                    ClassID::Texture2D => texture = Some(obj.try_into_class()?),
                    ClassID::Mesh => mesh = Some(obj.try_into_class()?),
                    _ => ()
                }
            }
        }
    }

    if let Some(texture) = texture {
        let mut image = texture.read_data(&unity_fs)?.decode()?;

        if let Some(mesh) = mesh {
            let mesh = mesh.resolve_meshes(&unity_fs)?.into_iter().next().ok_or(UnityError::UnexpectedEof)?;
            image = resolve_with_mesh(mesh, image)?;
        }

        imageops::flip_vertical_in_place(&mut image);

        let mut writer = Cursor::new(Vec::new());
        image.write_to(&mut writer, ImageOutputFormat::WebP)?;
        return Ok(Some(writer.into_inner()))
    }

    println!("File {path:?} contained no images.");
    Ok(None)
}

#[must_use]
fn resolve_with_mesh(mesh: ResolvedMesh, image: RgbaImage) -> anyhow::Result<RgbaImage> {
    struct Item<'a> {
        top_left: (u32, u32),
        bottom_right: (u32, u32),
        image: SubImage<&'a RgbaImage>
    }

    let parts = mesh.triangles().map(|t| {
        let top_left = (
            t.0.pos.x.min(t.1.pos.x).min(t.2.pos.x).max(0f32) as u32,
            t.0.pos.y.min(t.1.pos.y).min(t.2.pos.y).max(0f32) as u32
        );

        let bottom_right = (
            t.0.pos.x.max(t.1.pos.x).max(t.2.pos.x).max(0f32) as u32,
            t.0.pos.y.max(t.1.pos.y).max(t.2.pos.y).max(0f32) as u32
        );

        let crop_x = (t.0.uv.x.min(t.1.uv.x).min(t.2.uv.x).clamp(0f32, 1f32) * image.width() as f32) as u32;
        let crop_y = (t.0.uv.y.min(t.1.uv.y).min(t.2.uv.y).clamp(0f32, 1f32) * image.height() as f32) as u32;
        let crop_width = (t.0.uv.x.max(t.1.uv.x).max(t.2.uv.x).clamp(0f32, 1f32) * image.width() as f32) as u32 - crop_x;
        let crop_height = (t.0.uv.y.max(t.1.uv.y).max(t.2.uv.y).clamp(0f32, 1f32) * image.height() as f32) as u32 - crop_y;

        let image = imageops::crop_imm(&image, crop_x, crop_y, crop_width, crop_height);

        Item {
            top_left,
            bottom_right,
            image
        }
    }).collect::<Vec<_>>();

    let width = parts.iter().map(|p| p.bottom_right.0).max().unwrap() + 1;
    let height = parts.iter().map(|p| p.bottom_right.1).max().unwrap() + 1;

    let mut new_image = RgbaImage::new(width, height);
    for part in parts {
        new_image.copy_from(&*part.image, part.top_left.0, part.top_left.1)?;
    }

    return Ok(new_image);
}
