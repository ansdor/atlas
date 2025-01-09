use std::{
    cmp,
    collections::HashSet,
    io::Write,
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

use crate::{images, interface, outputs, utils};

const ROW_CONST_RATIO: usize = 16;
const DEFAULT_LUT_SIZE: usize = 32;
const LUT_SIZE_RANGE: RangeInclusive<usize> = 1..=256;

struct LutSettings {
    dimensions: usize,
    rows: usize,
}

impl LutSettings {
    fn size(&self) -> (usize, usize) { (cmp::min(self.dimensions, ROW_CONST_RATIO), self.rows) }
}

pub fn lut(
    args: &interface::LutArguments, log: &mut Option<impl Write>,
) -> utils::GeneralResult<()> {
    let current_dir = std::env::current_dir()?;
    let source_image = args.image.as_ref().map(|x| {
        let s = PathBuf::from(x);
        if s.is_relative() {
            current_dir.join(s)
        } else {
            s
        }
    });
    let settings = generate_settings(args)?;
    let palette = match source_image {
        Some(i) => Some(palette_from_image(i)?),
        None => None,
    };
    let (color_count, pixels) = generate_lut_pixels(settings.dimensions, &palette);
    let output_path = {
        let label = match PathBuf::from(&args.output).file_stem() {
            Some(stem) => stem.to_string_lossy().to_string(),
            None => return Err(format!("unable to extract filename from '{}'", args.output).into()),
        };
        let dir = outputs::prepare_output_directory(&args.output, outputs::PathType::Files, log)?;
        dir.join(format!("{}.{}", label, "png"))
    };
    if let Some(msg) = outputs::notify_overwrite(&output_path, args.overwrite)? {
        utils::info_message(log, msg);
    }
    utils::info_message(log, format!("LUT color count: {}", color_count));
    images::generate_lut(&output_path, &pixels, settings.dimensions, settings.size())?;
    Ok(())
}

fn generate_settings(args: &interface::LutArguments) -> utils::GeneralResult<LutSettings> {
    let dimensions = match args.dimensions {
        None => Some(DEFAULT_LUT_SIZE),
        Some(n) => {
            if LUT_SIZE_RANGE.contains(&n) {
                Some(n)
            } else {
                None
            }
        }
    };
    let rows = match (dimensions, args.rows) {
        (None, _) => None,
        (Some(d), None) => Some(d.div_ceil(ROW_CONST_RATIO)),
        (Some(d), Some(r)) => {
            if (1..=d).contains(&r) {
                Some(r)
            } else {
                None
            }
        }
    };
    match (dimensions, rows) {
        (None, _) => Err(format!(
            "LUT dimensions must be in the {}-{} range.",
            LUT_SIZE_RANGE.start(),
            LUT_SIZE_RANGE.end()
        )
        .into()),
        (_, None) => Err("Invalid number of rows".into()),
        (Some(dimensions), Some(rows)) => Ok(LutSettings { dimensions, rows }),
    }
}

fn generate_lut_pixels(size: usize, palette: &Option<Vec<u32>>) -> (usize, Vec<u32>) {
    let get_index = |x, y, z| -> usize { z * (size * size) + y * (size) + x };
    let get_color = |x, y, z| -> u32 {
        let limit = size - 1;
        let r = (((x as f64) / (limit as f64)) * 255.0) as u32;
        let g = (((y as f64) / (limit as f64)) * 255.0) as u32;
        let b = (((z as f64) / (limit as f64)) * 255.0) as u32;
        (r << 24) | (g << 16) | (b << 8) | 0xff
    };
    let mut color_set = HashSet::new();
    let mut pixels = vec![0; size * size * size];
    for z in 0..size {
        for y in 0..size {
            for x in 0..size {
                let hex_color = get_color(x, y, z);
                let rgb_color = rgb_from_hex(hex_color);
                let index = get_index(x, y, z);
                let color = match palette {
                    Some(p) => match best_color_match(rgb_color, p) {
                        Some(color) => color,
                        None => hex_color,
                    },
                    None => hex_color,
                };
                color_set.insert(color);
                pixels[index] = color;
            }
        }
    }
    (color_set.len(), pixels)
}

fn palette_from_image<P: AsRef<Path>>(path: P) -> utils::GeneralResult<Vec<u32>> {
    let pixels = images::image_to_pixel_buffer(path)?;
    Ok(palette_from_pixel_buffer(&pixels))
}

fn palette_from_pixel_buffer(pixels: &[u32]) -> Vec<u32> {
    let mut colors = HashSet::new();
    pixels.iter().for_each(|p| {
        colors.insert(*p);
    });
    colors.into_iter().collect()
}

fn best_color_match(color: (f64, f64, f64), palette: &[u32]) -> Option<u32> {
    palette.iter().copied().min_by(|x, y| {
        let dx = color_distance(color, rgb_from_hex(*x));
        let dy = color_distance(color, rgb_from_hex(*y));
        dx.total_cmp(&dy)
    })
}

fn rgb_from_hex(color: u32) -> (f64, f64, f64) {
    let r = (color >> 24) & 0xff;
    let g = (color >> 16) & 0xff;
    let b = (color >> 8) & 0xff;
    ((r as f64) / 255.0, (g as f64) / 255.0, (b as f64) / 255.0)
}

fn color_distance(x: (f64, f64, f64), y: (f64, f64, f64)) -> f64 {
    ((y.0 - x.0).powi(2) + (y.1 - x.1).powi(2) + (y.2 - x.2).powi(2)).sqrt()
}
