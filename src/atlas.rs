use std::borrow::Borrow;

use serde::{Deserialize, Serialize};

use crate::{packing::TexturePacker, sources::SourceTexture};

#[derive(Debug, Serialize, Deserialize)]
struct AtlasTexture {
    pub name: String,
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    #[serde(flatten, default, skip_serializing_if = "Option::is_none")]
    pub extra: Option<AtlasTextureExtra>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AtlasTextureExtra {
    pub original_width: u32,
    pub original_height: u32,
    pub rotated: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct AtlasPage {
    pub texture: String,
    pub width: u32,
    pub height: u32,
    pub regions: Vec<AtlasTexture>,
}

pub fn generate_description(packer: &TexturePacker) -> Option<String> {
    let mut r: Vec<AtlasPage> = Vec::new();
    for (idx, page) in packer.pages.iter().enumerate() {
        let texture = match packer.pages.len() {
            1 => format!("{}.png", packer.label),
            _ => format!("{}-{}.png", packer.label, idx),
        };
        let (width, height) = match page.size {
            Some((w, h)) => (w, h),
            None => page.packed_bounds(),
        };
        let regions: Vec<AtlasTexture> = page.textures.iter().map(AtlasTexture::from).collect();
        r.push(AtlasPage {
            texture,
            width,
            height,
            regions,
        });
    }
    //if rotation is disabled in the settings,
    //remove the extra fields from the textures
    if !packer.settings.rotation {
        r.iter_mut().for_each(|x| remove_extra_fields(&mut x.regions))
    }

    match r.len() {
        1 => serde_json::to_string_pretty(&r[0]),
        _ => serde_json::to_string_pretty(&r),
    }
    .ok()
}

pub fn read_from_description(source: &str) -> Option<Vec<(String, Vec<SourceTexture>)>> {
    let mut v = Vec::new();
    //try to parse the string as a single texture page
    if let Ok(page) = serde_json::from_str::<AtlasPage>(source) {
        //if it works, add it to the vector
        let t = page.regions.into_iter().map(SourceTexture::from).collect();
        v.push((page.texture, t));
    }
    //if the first attempt didn't work, try parsing it as a vector of pages
    else if let Ok(pages) = serde_json::from_str::<Vec<AtlasPage>>(source) {
        //if it works, add all the pages to the vector
        for page in pages.into_iter() {
            let t = page.regions.into_iter().map(SourceTexture::from).collect();
            v.push((page.texture, t));
        }
    } else {
        //nothing worked, give up
        return None;
    }
    Some(v)
}

impl<T: Borrow<AtlasTexture>> From<T> for SourceTexture {
    fn from(src: T) -> Self {
        use crate::rectangle::Rect;
        use crate::sources::PackingData;
        let src = src.borrow();
        let name_without_slashes = src.name.to_owned().replace('/', "-");
        let pd = PackingData {
            position: Rect {
                x: src.x,
                y: src.y,
                width: src.width,
                height: src.height,
            },
            rotated: match &src.extra {
                Some(extra) => extra.original_width != src.width || extra.original_height != src.height,
                None => false,
            },
        };
        SourceTexture {
            name: name_without_slashes.clone(),
            path: std::path::PathBuf::from(&name_without_slashes),
            dimensions: if pd.rotated {
                Rect { x: 0, y: 0, width: src.height, height: src.width }
            } else {
                Rect { x: 0, y: 0, width: src.width, height: src.height }
            },
            replica_of: None,
            packing: Some(pd)
        }
    }
}

impl<T: Borrow<SourceTexture>> From<T> for AtlasTexture {
    fn from(src: T) -> Self {
        let src = src.borrow();
        let packing = src.packing.as_ref().unwrap();
        AtlasTexture {
            name: src.name.to_owned(),
            x: packing.position.x,
            y: packing.position.y,
            width: packing.position.width,
            height: packing.position.height,
            extra: Some(AtlasTextureExtra {
                original_width: src.dimensions.width,
                original_height: src.dimensions.height,
                rotated: packing.rotated,
            }),
        }
    }
}

fn remove_extra_fields(entries: &mut [AtlasTexture]) { entries.iter_mut().for_each(|x| x.extra = None); }
