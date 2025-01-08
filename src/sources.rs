use std::cmp;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use crate::rectangle::Rect;
use crate::utils;

#[derive(Debug)]
pub struct SourceTexture {
    pub name: String,
    pub path: PathBuf,
    pub dimensions: Rect,
    pub replica_of: Option<String>,
    pub packing: Option<PackingData>,
}

#[derive(Debug, Clone)]
pub struct PackingData {
    pub position: Rect,
    pub rotated: bool,
}

impl SourceTexture {
    /// Adds another part of the texture's path to its name
    pub fn specialize_name(&mut self) {
        let separator = "/";
        let mut buffer = String::new();
        for c in self.path.components().rev() {
            let prefix = String::from(c.as_os_str().to_string_lossy());
            buffer.insert_str(0, prefix.as_str());
            if !self.name.ends_with(buffer.as_str()) {
                self.name = buffer;
                break;
            } else {
                buffer.insert_str(0, separator);
            }
        }
    }
}

fn scan_for_sources<P>(
    node: P, extensions: &[&str], bucket: &mut Vec<PathBuf>,
) -> utils::GeneralResult<()>
where
    P: AsRef<Path>, {
    let node = if node.as_ref().is_absolute() {
        PathBuf::from(node.as_ref())
    } else {
        std::env::current_dir()?.join(node)
    };
    match node {
        x if x.is_dir() => {
            for e in std::fs::read_dir(x)? {
                let path = e?.path();
                scan_for_sources(path, extensions, bucket)?;
            }
        }
        x if x.is_file() => {
            let ext = x.extension().unwrap_or_else(|| OsStr::new(""));
            if extensions.contains(&ext.to_str().unwrap()) {
                bucket.push(x);
            }
        }
        //if a directory entry isn't a file or a folder, just skip it
        _ => {}
    };
    Ok(())
}

pub fn source_list_from_paths<P: AsRef<Path>>(
    sources: &[P], extensions: &[&str]) -> utils::GeneralResult<Vec<SourceTexture>> {
    //if there are no sources, nothing to do
    if sources.is_empty() {
        return Err("No source provided".into());
    }
    let mut paths = Vec::new();
    for src in sources.iter() {
        let src = src.as_ref();
        //if a source doesn't exist, return an error
        if !src.exists() {
            return Err(format!("source '{}' not found.", src.display()).into());
        }
        //recursively scan for textures
        scan_for_sources(src, extensions, &mut paths)?;
    }
    if paths.is_empty() {
        return Err("no textures found.".into());
    }
    //sort and dedup
    paths.sort();
    paths.dedup();
    Ok(paths
        .into_iter()
        .filter_map(|x| read_texture_info(x).ok())
        .collect())
}

fn read_texture_info<P: AsRef<Path>>(source: P) -> utils::GeneralResult<SourceTexture> {
    let source = source.as_ref();
    let (width, height) = image::image_dimensions(source)?;
    Ok(SourceTexture {
        name: String::from(source.file_name().unwrap().to_str().unwrap()),
        path: PathBuf::from(source),
        dimensions: Rect::new(0, 0, width, height),
        replica_of: None,
        packing: None,
    })
}

fn textures_are_duplicates(a: &SourceTexture, b: &SourceTexture) -> utils::GeneralResult<bool> {
    //step 1: dimensions
    if a.dimensions.width != b.dimensions.width || a.dimensions.height != b.dimensions.height {
        return Ok(false);
    }
    //step 2: byte lengths
    let (len_a, len_b) = (
        std::fs::metadata(&a.path)?.len(),
        std::fs::metadata(&b.path)?.len(),
    );
    if len_a != len_b {
        return Ok(false);
    }
    //step 3, byte by byte comparison
    const BUFFER_SIZE: usize = 1024;
    let mut buffers = (vec![0u8; BUFFER_SIZE], vec![0u8; BUFFER_SIZE]);
    let mut handles = (
        BufReader::new(File::open(&a.path)?),
        BufReader::new(File::open(&b.path)?),
    );
    loop {
        let read = (
            handles.0.read(&mut buffers.0)?,
            handles.1.read(&mut buffers.1)?,
        );
        if read.0 == 0 && read.1 == 0 {
            //EOF was reached and no difference was found, they are duplicates
            return Ok(true);
        } else if read.0 != read.1 || buffers.0 != buffers.1 {
            //a difference was found, they're not duplicates
            return Ok(false);
        }
    }
}

pub fn solve_name_collisions(sources: &mut [SourceTexture]) {
    let mut names = HashMap::<String, Vec<usize>>::new();
    let mut collision;
    //loop until all conflicts are solved
    loop {
        names.clear();
        collision = false;
        //count the times each name appears
        for (idx, src) in sources.iter().enumerate() {
            names
                .entry(src.name.clone())
                .and_modify(|x| x.push(idx))
                .or_insert(vec![idx]);
        }
        //if there are no collisions, all hashmap entries
        //should have len() == 1. test for this condition
        for entry in names.drain().filter(|x| x.1.len() > 1) {
            collision = true;
            //try to fix the entry's path
            //skip the first, the 'original'
            entry
                .1
                .iter()
                .skip(1)
                .for_each(|x| sources[*x].specialize_name());
        }
        //if there were no collisions, break the loop
        if !collision {
            break;
        }
    }
}

pub fn deduplicate_textures(sources: &mut [SourceTexture]) -> utils::GeneralResult<()> {
    //create a hashmap to check for images with the exact same dimensions
    let mut sizes = HashMap::<(u32, u32), Vec<usize>>::new();
    //iterate over the sources, and group the indices using the image dimensions
    for (idx, src) in sources.iter().enumerate() {
        sizes
            .entry((src.dimensions.width, src.dimensions.height))
            .and_modify(|x| x.push(idx))
            .or_insert(vec![idx]);
    }
    for group in sizes
        .into_iter()
        .filter_map(|x| if x.1.len() > 1 { Some(x.1) } else { None })
    {
        for (idx, first) in group.iter().enumerate() {
            if sources[*first].replica_of.is_some() {
                continue;
            }
            for second in group.iter().skip(idx + 1) {
                if textures_are_duplicates(&sources[*first], &sources[*second])? {
                    sources[*second].replica_of = Some(sources[*first].name.clone())
                }
            }
        }
    }
    Ok(())
}

pub fn validate_dimensions(
    sources: &[SourceTexture], page_size: (u32, u32), spacing: u32
) -> utils::GeneralResult<()> {
    //build a collection of images that don't fit the provided page size
    let misfits: Vec<usize> = sources
        .iter()
        .enumerate()
        .filter_map(|(i, x)| {
            if x.dimensions.width + spacing > page_size.0
                || x.dimensions.height + spacing > page_size.1
            {
                Some(i)
            } else {
                None
            }
        })
        .collect();
    //if the collection is empty, all images fit the page size, success
    if misfits.is_empty() {
        Ok(())
    } else {
        //otherwise, build an error message to return
        let mut err = String::new();
        err.push_str("the following images can't be packed with the current settings:");
        err.push_str(
            format!(
                "\n\tpage size: {}x{}, spacing: {}px",
                page_size.0, page_size.1, spacing
            )
            .as_str(),
        );
        misfits.iter().for_each(|n| {
            let s = &sources[*n];
            let (name, w, h) = (&s.name, s.dimensions.width, s.dimensions.height);
            err.push_str(format!("\n\t{} [{}x{}]", name, w, h).as_str());
        });
        //sources.len() is always > 0, so it's safe to call unwrap() here
        let min = sources
            .iter()
            .map(|x| (x.dimensions.width + spacing, x.dimensions.height + spacing))
            .reduce(|req, n| (cmp::max(req.0, n.0), cmp::max(req.1, n.1)))
            .unwrap_or((0, 0));
        err.push_str(
            format!(
                "\nminimum required page size for these settings: {}x{}",
                min.0, min.1
            )
            .as_str(),
        );
        err.push_str("\nfailed to pack textures.");
        Err(err.into())
    }
}

pub fn report_duplicates(sources: &[SourceTexture]) -> Option<(usize, String)> {
    let count = sources.iter().filter(|x| x.replica_of.is_some()).count();
    match count {
        0 => None,
        _ => {
            let mut b = String::new();
            b.push_str(
                format!(
                    "found {} duplicate{}:",
                    count,
                    if count > 1 { "s" } else { "" }
                )
                .as_str(),
            );
            sources
                .iter()
                .filter(|x| x.replica_of.is_some())
                .for_each(|x| {
                    b.push_str(
                        format!("\n\t{} => {}", x.name, x.replica_of.as_ref().unwrap()).as_str(),
                    )
                });
            Some((count, b))
        }
    }
}
