use std::{
    cmp,
    collections::HashSet,
    io::Write,
    ops::RangeInclusive,
    path::{Path, PathBuf},
};

use kiddo::{SquaredEuclidean, float::kdtree::KdTree};

use crate::{images, interface, outputs, utils};

const LUT_SIZE_RANGE: RangeInclusive<usize> = 1..=256;
const KDTREE_BUCKET_SIZE: usize = 1024;

struct LutSettings {
    dimensions: usize,
    max_columns: usize,
}

impl LutSettings {
    fn columns(&self) -> usize { cmp::min(self.dimensions, self.max_columns) }

    fn rows(&self) -> usize { self.dimensions.div_ceil(self.columns()) }

    fn size(&self) -> (usize, usize) { (self.columns(), self.rows()) }
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
    let mut palette = match source_image {
        Some(i) => Some(palette_from_image(i)?),
        None => None,
    };
    if args.expand {
        if let Some(p) = palette.take() {
            palette.replace(expand_palette(&p)?);
        }
    }
    let (color_count, pixels) = generate_lut_pixels(settings.dimensions, palette);
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
    utils::info_message(log, format!("LUT color count: {color_count}"));
    images::generate_lut(&output_path, &pixels, settings.dimensions, settings.size())?;
    Ok(())
}

fn generate_settings(args: &interface::LutArguments) -> utils::GeneralResult<LutSettings> {
    const DEFAULT_LUT_SIZE: usize = 32;
    const DEFAULT_MAX_ROWS: usize = 16;
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
    let max_columns = match (dimensions, args.max_columns) {
        (None, _) => None,
        (Some(d), None) => Some(d.div_ceil(DEFAULT_MAX_ROWS)),
        (Some(d), Some(c)) => Some(cmp::min(c, d)),
    };
    match (dimensions, max_columns) {
        (None, _) => Err(format!(
            "LUT dimensions must be in the {}-{} range.",
            LUT_SIZE_RANGE.start(),
            LUT_SIZE_RANGE.end()
        )
        .into()),
        (_, None) => Err("Invalid max number of columns.".into()),
        (Some(dimensions), Some(max_columns)) => Ok(LutSettings {
            dimensions,
            max_columns,
        }),
    }
}

fn generate_lut_pixels(size: usize, palette: Option<Vec<u32>>) -> (usize, Vec<u32>) {
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
    let color_tree = palette.as_ref().map(|p| kdtree_from_palette(p));
    for z in 0..size {
        for y in 0..size {
            for x in 0..size {
                let hex_color = get_color(x, y, z);
                let rgb_color = rgb_from_hex(hex_color);
                let index = get_index(x, y, z);
                let color = match &palette {
                    Some(p) => find_nearest_color(rgb_color, p, color_tree.as_ref().unwrap()),
                    None => hex_color,
                };
                color_set.insert(color);
                pixels[index] = color;
            }
        }
    }
    (color_set.len(), pixels)
}

fn expand_palette(palette: &[u32]) -> utils::GeneralResult<Vec<u32>> {
    const EXPANSION_LIMIT: usize = 512;
    if palette.len() > EXPANSION_LIMIT {
        Err(format!(
            "palette expansion is limited to to {} colors, length {} given",
            EXPANSION_LIMIT,
            palette.len()
        )
        .into())
    } else {
        let mut p = HashSet::<u32>::new();
        palette.iter().for_each(|c0| {
            palette.iter().for_each(|c1| {
                p.insert(blend_colors(*c0, *c1, 0.5));
            })
        });
        Ok(Vec::from_iter(p.drain()))
    }
}

fn palette_from_image<P: AsRef<Path>>(path: P) -> utils::GeneralResult<Vec<u32>> {
    let pixels = images::image_to_pixel_buffer(path)?;
    Ok(palette_from_pixel_buffer(&pixels))
}

fn palette_from_pixel_buffer(pixels: &[u32]) -> Vec<u32> {
    let mut colors = HashSet::new();
    pixels.iter().for_each(|p| {
        colors.insert(*p | 0xff);
    });
    colors.into_iter().collect()
}

fn kdtree_from_palette(palette: &[u32]) -> KdTree<f64, usize, 3, KDTREE_BUCKET_SIZE, u32> {
    KdTree::from_iter(palette.iter().enumerate().map(|(i, c)| {
        let rgb = rgb_from_hex(*c);
        ([rgb.0, rgb.1, rgb.2], i)
    }))
}

fn find_nearest_color(
    color: (f64, f64, f64), palette: &[u32], tree: &KdTree<f64, usize, 3, KDTREE_BUCKET_SIZE, u32>,
) -> u32 {
    let index = tree
        .nearest_one::<SquaredEuclidean>(&[color.0, color.1, color.2])
        .item;
    palette[index]
}

fn blend_colors(c0: u32, c1: u32, blend: f64) -> u32 {
    fn lerp(a: f64, b: f64, t: f64) -> f64 { a + (b - a) * t }
    let h0 = rgb_from_hex(c0);
    let h1 = rgb_from_hex(c1);
    hex_from_rgb((
        lerp(h0.0, h1.0, blend),
        lerp(h0.1, h1.1, blend),
        lerp(h0.2, h1.2, blend),
    ))
}

fn hex_from_rgb(color: (f64, f64, f64)) -> u32 {
    let (cr, cg, cb) = color;
    let r = (cr * 255.0) as u32;
    let g = (cg * 255.0) as u32;
    let b = (cb * 255.0) as u32;
    (r << 24) | (g << 16) | (b << 8) | 0xff
}

fn rgb_from_hex(color: u32) -> (f64, f64, f64) {
    let r = (color >> 24) & 0xff;
    let g = (color >> 16) & 0xff;
    let b = (color >> 8) & 0xff;
    ((r as f64) / 255.0, (g as f64) / 255.0, (b as f64) / 255.0)
}
