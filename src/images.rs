use std::{path::Path, sync::mpsc::Sender};

use image::{GenericImage, GenericImageView};

use crate::{packing::TexturePage, sources::SourceTexture, utils};

pub fn generate_image<P: AsRef<Path>>(page: TexturePage, destination: P) -> utils::GeneralResult<()> {
    let (w, h) = match page.size {
        Some((w, h)) => (w, h),
        None => page.packed_bounds(),
    };
    let mut canvas = image::RgbaImage::new(w, h);
    for e in page.textures.into_iter().filter(|x| x.replica_of.is_none()) {
        let packing = e.packing.unwrap();
        let mut source = image::open(&e.path)?;
        if packing.rotated {
            source = image::DynamicImage::from(image::imageops::rotate90(&source));
        }
        canvas.copy_from(&source, packing.position.x, packing.position.y)?;
    }
    canvas.save(&destination)?;
    Ok(())
}

pub fn unpack_page<P: AsRef<Path>>(
    (src, dst): (P, P), entries: &[SourceTexture], progress: Option<&Sender<u64>>,
) -> utils::GeneralResult<()> {
    let source_image = image::open(&src)?;
    for e in entries {
        let p = e.packing.clone().unwrap(); //safe call to unwrap
        let view = source_image
            .view(p.position.x, p.position.y, p.position.width, p.position.height)
            .to_image();
        let mut canvas = image::RgbaImage::new(p.position.width, p.position.height);
        canvas.copy_from(&view, 0, 0)?;
        if p.rotated {
            canvas = image::imageops::rotate270(&canvas);
        }
        canvas.save(&dst.as_ref().join(&e.path))?;
        if let Some(progress) = progress {
            progress.send(1)?;
        }
    }
    Ok(())
}
