use crate::atlas::{AtlasPage, self};
use crate::sources::SourceTexture;

pub trait AtlasFormatter {
    fn format_atlas(&self, pages: &Vec<AtlasPage>) -> Option<String>;
    fn read_atlas(&self, source: &str) -> Option<Vec<(String, Vec<SourceTexture>)>>;
}

pub struct JsonFormatter;
pub struct TextFormatter;

impl AtlasFormatter for JsonFormatter {
    fn format_atlas(&self, pages: &Vec<AtlasPage>) -> Option<String> {
        match pages.len() {
            1 => serde_json::to_string_pretty(&pages[0]),
            _ => serde_json::to_string_pretty(&pages),
        }
        .ok()
    }

    fn read_atlas(&self, source: &str) -> Option<Vec<(String, Vec<SourceTexture>)>> {
        let mut r = Vec::new();
        if let Ok(page) = serde_json::from_str::<AtlasPage>(source) {
            let t = page.regions.into_iter().map(SourceTexture::from).collect();
            r.push((page.texture, t));
        } else if let Ok(pages) = serde_json::from_str::<Vec<AtlasPage>>(source) {
            for page in pages.into_iter() {
                let t = page.regions.into_iter().map(SourceTexture::from).collect();
                r.push((page.texture, t));
            }
        } else {
            return None;
        }
        Some(r)
    }
}

impl AtlasFormatter for TextFormatter {
    fn format_atlas(&self, pages: &Vec<AtlasPage>) -> Option<String> {
        let mut buffer = String::new();
        buffer += "# page <name> <width> <height>\n";
        buffer += "# region <name> <x> <y> <width> <height> [<rotated> <original_width> <original_height>]\n";
        for page in pages {
            buffer += format!("page \"{}\" {} {}\n", page.texture, page.width, page.height).as_str();
            for region in page.regions.iter() {
                let (name, x, y, w, h) = (&region.name, region.x, region.y, region.width, region.height);
                let extra = match &region.extra {
                    Some(x) => Some((x.rotated, x.original_width, x.original_height)),
                    None => None,
                };
                let mut line = format!("region \"{}\" {} {} {} {}", name, x, y, w, h);
                line += match extra {
                    Some((r, ow, oh)) => format!(" {} {} {}\n", if r { 1 } else { 0 }, ow, oh),
                    None => String::from("\n"),
                }
                .as_str();
                buffer += line.as_str();
            }
        }
        Some(buffer)
    }

    fn read_atlas(&self, source: &str) -> Option<Vec<(String, Vec<SourceTexture>)>> {
        let mut result = vec![];
        for line in source.lines() {
            let elements: Vec<&str> = line.split(' ').collect();
            match elements[0] {
                "#" => continue,
                "page" => {
                    result.push((String::from(elements[1]).replace('"', ""), Vec::new()));
                }
                "region" => {
                    let region_name = String::from(elements[1]).replace('"', "");
                    let region_values = elements
                        .iter()
                        .skip(2) //'region' and name
                        .map(|x| x.parse::<u32>().ok())
                        .collect::<Vec<Option<u32>>>();

                    if !region_values.iter().all(Option::is_some) {
                        return None;
                    }

                    let mut region_values: Vec<u32> = region_values.into_iter().map(Option::unwrap).collect();
                    let region_extras = if region_values.len() == 7 {
                        let original_height = region_values.pop().unwrap();
                        let original_width = region_values.pop().unwrap();
                        let rotated = region_values.pop().unwrap() != 0;
                        Some(atlas::AtlasTextureExtra {
                            original_width,
                            original_height,
                            rotated
                        })
                    } else {
                        None
                    };

                    let region_info = {
                        let height = region_values.pop().unwrap();
                        let width = region_values.pop().unwrap();
                        let y = region_values.pop().unwrap();
                        let x = region_values.pop().unwrap();
                        atlas::AtlasTexture {
                            name: region_name.clone(),
                            x,
                            y,
                            width,
                            height,
                            extra: region_extras
                        }
                    };

                    let current_page = result.last_mut();
                    match current_page {
                        Some(page) => page.1.push(SourceTexture::from(region_info)),
                        None => {
                            //it's an error if there's no current page
                            return None;
                        }
                    }
                }
                _ => {
                    //unrecognized line, error
                    return None;
                }
            }
        }
        Some(result)
    }
}
