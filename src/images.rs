use std::{fs, io::BufWriter, path::Path, sync::mpsc::Sender};

use image::{GenericImage, GenericImageView, ImageEncoder, Rgba};

use crate::{packing::TexturePage, sources::SourceTexture, utils};

fn save_image_to_disk<P: AsRef<Path>>(
    image: &image::RgbaImage, path: P,
) -> utils::GeneralResult<()> {
    use image::codecs::png;

    let file = fs::File::create(path)?;
    let writer = BufWriter::new(file);
    let encoder = png::PngEncoder::new_with_quality(
        writer,
        png::CompressionType::Best,
        png::FilterType::Adaptive,
    );
    encoder.write_image(
        image.as_raw(),
        image.width(),
        image.height(),
        image::ExtendedColorType::Rgba8,
    )?;
    Ok(())
}

pub fn image_to_pixel_buffer<P: AsRef<Path>>(path: P) -> utils::GeneralResult<Vec<u32>> {
    let image = {
        let i = image::open(&path)?;
        i.into_rgba8()
    };
    Ok(image
        .pixels()
        .map(|p| {
            let [r, g, b, a] = p.0;
            ((r as u32) << 24) | ((g as u32) << 16) | ((b as u32) << 8) | (a as u32)
        })
        .collect())
}

pub fn generate_lut<P: AsRef<Path>>(
    destination: P, pixels: &[u32], size: usize, rows: usize,
) -> utils::GeneralResult<()> {
    let columns = (size as f64 / rows as f64).ceil() as usize;
    let to_canvas_coords = |x, y, z| {
        let page_coords = ((z % columns) * size, (z / columns) * size);
        ((page_coords.0 + x) as u32, (page_coords.1 + y) as u32)
    };
    let mut canvas = image::RgbaImage::new((columns * size) as u32, (rows * size) as u32);
    for z in 0..size {
        for y in 0..size {
            for x in 0..size {
                let color = pixels[z * size * size + y * size + x];
                let coords = to_canvas_coords(x, y, z);
                canvas.put_pixel(coords.0, coords.1, Rgba::from(color.to_be_bytes()));
            }
        }
    }
    save_image_to_disk(&canvas, &destination)
}

pub fn generate_image<P: AsRef<Path>>(
    page: TexturePage, destination: P,
) -> utils::GeneralResult<()> {
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
    save_image_to_disk(&canvas, &destination)
}

pub fn unpack_page<P: AsRef<Path>>(
    (src, dst): (P, P), entries: &[SourceTexture], progress: Option<&Sender<u64>>,
) -> utils::GeneralResult<()> {
    let source_image = image::open(&src)?;
    for e in entries {
        let p = e.packing.clone().unwrap(); //safe call to unwrap
        let view = source_image
            .view(
                p.position.x,
                p.position.y,
                p.position.width,
                p.position.height,
            )
            .to_image();
        let mut canvas = image::RgbaImage::new(p.position.width, p.position.height);
        canvas.copy_from(&view, 0, 0)?;
        if p.rotated {
            canvas = image::imageops::rotate270(&canvas);
        }
        save_image_to_disk(&canvas, dst.as_ref().join(&e.path))?;
        if let Some(progress) = progress {
            progress.send(1)?;
        }
    }
    Ok(())
}
