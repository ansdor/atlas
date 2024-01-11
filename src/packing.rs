use std::{cmp, fmt, mem, sync::mpsc};

use super::rectangle::Rect;
use crate::{
    interface,
    sources::{PackingData, SourceTexture},
    utils::{self, GeneralResult},
};

const MAX_DIMENSIONS: u32 = 65535;
const MAX_SPACING: u32 = 1024;

#[derive(Debug, Clone)]
pub enum PackingMethod {
    Distance,
    Area,
}

impl fmt::Display for PackingMethod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{:?}", self) }
}

#[derive(Debug, Clone)]
pub struct PackingSettings {
    pub method: PackingMethod,
    pub spacing: u32,
    pub rotation: bool,
    pub page_size: Option<(u32, u32)>,
}

pub struct TexturePage {
    pub name: String,
    pub textures: Vec<SourceTexture>,
    pub size: Option<(u32, u32)>,
    free_slots: Vec<Rect>,
}

pub struct TexturePacker {
    pub label: String,
    pub pages: Vec<TexturePage>,
    pub settings: PackingSettings,
    sources: Vec<SourceTexture>,
}

impl TexturePacker {
    pub fn new<T>(label: &str, sources: T, settings: PackingSettings) -> Self
    where
        T: IntoIterator<Item = SourceTexture>, {
        TexturePacker {
            label: String::from(label),
            sources: sources.into_iter().collect(),
            pages: vec![TexturePage::new(label, settings.page_size)],
            settings,
        }
    }

    pub fn count(&self) -> usize {
        self.sources
            .len()
            .saturating_add(self.pages.iter().map(|x| x.textures.len()).sum())
    }

    pub fn duplicates(&self) -> usize {
        self.pages
            .iter()
            .map(|x| x.textures.iter().filter(|x| x.replica_of.is_some()).count())
            .sum::<usize>()
            .saturating_add(self.sources.iter().filter(|x| x.replica_of.is_some()).count())
    }

    pub fn page_size(&self) -> (u32, u32) {
        match self.settings.page_size {
            Some((w, h)) => (w, h),
            None if !self.pages.is_empty() => self.pages[0].packed_bounds(),
            None => (0, 0),
        }
    }

    pub fn total_source_area(&self) -> u64 {
        self.pages.iter().map(|x| {
            x.textures.iter().filter_map(|x| match x.replica_of {
                Some(_) => None,
                None => Some((x.dimensions.width * x.dimensions.height) as u64)
            }).sum::<u64>()
        }).sum()
    }

    pub fn total_packed_area(&self) -> u64 {
        match self.settings.page_size {
            Some((w, h)) => ((self.pages.len() as u32) * w * h) as u64,
            None => {
                let bounds = self.pages[0].packed_bounds();
                (bounds.0 * bounds.1) as u64
            }
        }
    }

    pub fn efficiency(&self) -> f64 {
        (self.total_source_area() as f64) / (self.total_packed_area() as f64) * 100.0
    }

    fn add_page(&mut self) {
        self.pages
            .push(TexturePage::new(&self.label, self.settings.page_size));
    }

    fn adjust_page_names(&mut self) {
        match self.pages.len() {
            1 => self.pages[0].name = String::from(&self.label),
            _ => {
                self.pages.iter_mut().enumerate().for_each(|(i, x)| {
                    x.name = format!("{}-{}", &self.label, i);
                })
            }
        }
    }

    pub fn pack_everything(&mut self, progress: Option<mpsc::Sender<u64>>) -> utils::GeneralResult<()> {
        //unbox the sources container
        let sources = mem::take(&mut self.sources);
        let mut replicas = Vec::new();
        //iterate over the source textures
        for mut texture in sources.into_iter() {
            match texture.replica_of {
                //if this texture is not a duplicate
                None => {
                    //retrieve its dimensions
                    let dimensions = (texture.dimensions.width, texture.dimensions.height);
                    //find the first page where it can be packed
                    let mut packing = self
                        .pages
                        .iter_mut()
                        .enumerate()
                        .find_map(|(i, x)| {
                            x.pack_rectangle(dimensions, &self.settings).map(|p| (i, p))
                        });
                    //if the texture couldn't be packed in any page
                    if packing.is_none() {
                        //create a new page
                        self.add_page();
                        let last_page = self.pages.len() - 1;
                        //and pack the texture in it
                        if let Some(p) = self.pages[last_page].pack_rectangle(dimensions, &self.settings) {
                            packing = Some((last_page, p));
                        }
                    }
                    //report progress
                    if let Some(progress) = progress.as_ref() {
                        let _ = progress.send(1);
                    }
                    //at this point it's impossible for
                    //the texture not to be packed
                    if let Some((page_index, packing)) = packing {
                        //add the packing data to the texture struct
                        texture.packing = Some(packing);
                        //and move the texture to the page
                        self.pages[page_index].textures.push(texture);
                    } else {
                        //this should never happen, but just in case...
                        return Err(format!("failed to pack texture '{}'.", texture.name).into());
                    }
                }
                //if this texture is a duplicate of another
                Some(_) => {
                    //move it to the list of replicas
                    replicas.push(texture);
                }
            }
        }

        //iterate over all the duplicate textures
        for mut texture in replicas.drain(..) {
            //find the name of the original texture
            let original = texture.replica_of.clone().unwrap();
            //iterate over all the pages
            for page in self.pages.iter_mut() {
                //if the original is in this page
                if let Some(matrix) = page.textures.iter().find(|p| p.name == original) {
                    //copy the packing data from the original
                    texture.packing = matrix.packing.clone();
                    //add this texture to the same page
                    page.textures.push(texture);
                    //and break the loop
                    break;
                }
            }
        }
        //fix the page names
        self.adjust_page_names();
        Ok(())
    }
}

impl TexturePage {
    fn new(name: &str, size: Option<(u32, u32)>) -> Self {
        TexturePage {
            name: String::from(name),
            textures: Vec::new(),
            size,
            free_slots: match size {
                Some((w, h)) => vec![Rect::new(0, 0, w, h)],
                None => vec![],
            },
        }
    }

    fn pack_rectangle(&mut self, dimensions: (u32, u32), settings: &PackingSettings) -> Option<PackingData> {
        //create a copy of the rectangle to
        //be packed, and apply spacing to it
        let mut r = Rect::new(0, 0, dimensions.0 + settings.spacing, dimensions.1 + settings.spacing);
        //get the bounds of the set of packed rectangles
        let bounds = self.packed_bounds();
        //collect the indices of the free slots that can contain R
        let mut candidates: Vec<usize> = (0..self.free_slots.len())
            .filter(|&x| self.free_slots[x].can_contain(&r))
            .collect();
        //if rotation is allowed by the settings
        if settings.rotation {
            //collect the indices of free slots that
            //can contain a rotated version of R
            let r = Rect::new(0, 0, dimensions.1 + settings.spacing, dimensions.0 + settings.spacing);
            let extra: Vec<usize> = (0..self.free_slots.len())
                .filter(|&x| self.free_slots[x].can_contain(&r) && !candidates.contains(&x))
                .collect();
            //and add them to the list of candidates
            candidates.extend(extra);
        }
        //if there are no viable candidates
        if candidates.is_empty() {
            //see what the settings say about page size
            match settings.page_size {
                //if the page size is fixed
                Some(_) => {
                    //there is no way to pack R in this page
                    return None;
                }
                //if the page size is dynamic
                None => {
                    //create a new slot for R
                    self.free_slots.push(if bounds.0 + r.width >= bounds.1 + r.height {
                        Rect::new(0, bounds.1, cmp::max(bounds.0, r.width), r.height)
                    } else {
                        Rect::new(bounds.0, 0, r.width, cmp::max(bounds.1, r.height))
                    });
                    //and add it to the list of candidates
                    candidates.push(self.free_slots.len() - 1);
                }
            }
        }
        //at this point it's guaranteed that there is
        //at least one viable slot for R in the set.
        let dist_then_area = |a: &usize, b: &usize| {
            let (a, b) = (&self.free_slots[*a], &self.free_slots[*b]);
            Rect::cmp_by_distance(a, b).then(Rect::cmp_by_area(a, b))
        };
        let area_then_dist = |a: &usize, b: &usize| {
            let (a, b) = (&self.free_slots[*a], &self.free_slots[*b]);
            Rect::cmp_by_area(a, b).then(Rect::cmp_by_distance(a, b))
        };
        //pick the best candidate from the list
        //using the method defined in the settings
        let pick = self.free_slots.remove(match settings.method {
            PackingMethod::Distance => candidates.into_iter().min_by(dist_then_area).unwrap(),
            PackingMethod::Area => candidates.into_iter().min_by(area_then_dist).unwrap(),
        });
        //if the picked rectangle can't contain R,
        //it means R must be rotated to fit.
        let rotated = if !pick.can_contain(&r) {
            r.rotate();
            true
        } else {
            false
        };
        //set R's position to the picked slot
        r.place_at(pick.x, pick.y);
        //destroy the picked rectangle, and add
        //its remains to the free slots list
        self.free_slots.append(&mut pick.slice_out(&r));
        //iterate over the remaining indices, back to front
        for idx in (0..self.free_slots.len()).rev() {
            //find the intersection between this rect and R
            let overlap = self.free_slots[idx].intersection(&r);
            //if there is an intersection
            if overlap.area() > 0 {
                //remove this rectangle from the free slots
                let e = self.free_slots.remove(idx);
                //and create new rectangles from the free space
                self.free_slots.append(&mut e.slice_out(&r));
            }
        }
        //iterate one more time over the indices
        for a in (0..self.free_slots.len()).rev() {
            //and for each of the others
            for b in (0..(a.saturating_sub(1))).rev() {
                //if A is entirely contained within B
                if self.free_slots[b].contains(&self.free_slots[a]) {
                    //remove A and break the inner loop
                    self.free_slots.remove(a);
                    break;
                }
            }
        }
        //return the packing data for R
        Some(PackingData {
            position: r,
            rotated,
        })
    }

    pub fn packed_bounds(&self) -> (u32, u32) {
        let (mut w, mut h) = (0, 0);
        for r in self.packed_rects().iter() {
            w = cmp::max(r.x.saturating_add(r.width), w);
            h = cmp::max(r.y.saturating_add(r.height), h);
        }
        (w, h)
    }

    fn packed_rects(&self) -> Vec<&Rect> {
        self.textures
            .iter()
            .filter_map(|x| x.packing.as_ref())
            .map(|x| &x.position)
            .collect()
    }
}

pub fn generate_settings(args: &interface::PackArguments) -> GeneralResult<PackingSettings> {
    match read_page_size(&args.page_size) {
        Ok(page_size) => Ok(PackingSettings {
            method: match args.pack_by_area {
                true => PackingMethod::Area,
                false => PackingMethod::Distance,
            },
            spacing: cmp::min(args.spacing.unwrap_or(0), MAX_SPACING),
            rotation: args.rotate,
            page_size,
        }),
        Err(msg) => Err(msg),
    }
}

fn read_page_size(arg: &Option<String>) -> GeneralResult<Option<(u32, u32)>> {
    if let Some(s) = arg {
        let p: Vec<&str> = s.split('x').collect();
        if let (Ok(w), Ok(h)) = (p[0].parse::<u32>(), p[1].parse::<u32>()) {
            if w < MAX_DIMENSIONS && h < MAX_DIMENSIONS {
                Ok(Some((w, h)))
            } else {
                Err(format!("largest supported page size is {}x{}", MAX_DIMENSIONS, MAX_DIMENSIONS).into())
            }
        } else {
            Err(format!("failed to read dimensions from '{}'.", s).into())
        }
    } else {
        Ok(None)
    }
}
