use std::{
    collections::HashSet,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use indicatif::ProgressBar;
use utils::info_message;

use crate::{atlas, images, interface, outputs, sources, utils};

type UnpackedAtlas = Vec<(String, Vec<sources::SourceTexture>)>;

pub fn unpack(
    args: &interface::UnpackArguments, log: &mut Option<impl Write>,
) -> utils::GeneralResult<()> {
    let (textures_path, textures) = gather_textures_from_source(args)?;
    let textures = check_missing_textures(&textures_path, textures, log)?;
    let textures = fix_name_conflicts(textures);
    let output_path = outputs::prepare_output_directory(
        &args.output_directory,
        outputs::PathType::Directory,
        log,
    )?;
    let texture_paths = {
        let mut t = vec![];
        // textures is a Vec of Vecs
        textures.iter().for_each(|x| {
            x.1.iter().for_each(|x| {
                t.push(output_path.join(&x.path));
            })
        });
        t
    };
    let overwrite_count = outputs::count_overwrites(&texture_paths);
    if overwrite_count > 0 {
        match args.overwrite {
            true => {
                info_message(
                    log,
                    format!("{} files will be overwritten.", overwrite_count),
                );
            }
            false => {
                return Err(
                    "files already exist in output directory. use the -o flag to overwrite.".into(),
                );
            }
        }
    }
    unpack_with_progress_bar((textures_path, output_path), textures, log)
}

fn gather_textures_from_source(
    args: &interface::UnpackArguments,
) -> utils::GeneralResult<(PathBuf, UnpackedAtlas)> {
    let source_path = {
        let s = PathBuf::from(&args.source);
        if s.is_relative() {
            std::env::current_dir()?.join(s)
        } else {
            s
        }
    };
    let source_text = std::fs::read_to_string(&source_path)?;
    let textures = match atlas::read_from_description(&source_text) {
        Some(x) => x,
        None => return Err("failed to parse description file.".into()),
    };
    let textures_path = source_path.parent().unwrap().to_owned();
    Ok((textures_path, textures))
}

fn check_missing_textures<P: AsRef<Path>>(
    path: P, textures: UnpackedAtlas, log: &mut Option<impl Write>,
) -> utils::GeneralResult<UnpackedAtlas> {
    let missing_textures = textures
        .iter()
        .filter_map(|x| match path.as_ref().join(&x.0).exists() {
            false => Some(String::from(&x.0)),
            true => None,
        })
        .collect::<Vec<String>>();
    let textures = if !missing_textures.is_empty() {
        let mut msg = String::new();
        msg.push_str("some images were not found and will be skipped:\n");
        missing_textures
            .iter()
            .for_each(|x| msg.push_str(format!("\t{}\n", x).as_str()));
        info_message(log, msg);
        textures
            .into_iter()
            .filter(|x| !missing_textures.contains(&x.0))
            .collect()
    } else {
        textures
    };
    Ok(textures)
}

fn fix_name_conflicts(mut textures: UnpackedAtlas) -> UnpackedAtlas {
    let texture_count: usize = textures.iter().map(|x| x.1.len()).sum();
    let mut unique_names = HashSet::new();
    'duplicates: loop {
        unique_names.clear();
        for page in textures.iter_mut().map(|x| &mut x.1) {
            for e in page.iter_mut() {
                if unique_names.contains(&e.path) {
                    e.path = utils::append_to_filename(&e.path, "_(copy)");
                } else {
                    unique_names.insert(e.path.clone());
                }
            }
        }
        if unique_names.len() == texture_count {
            break 'duplicates;
        }
    }
    textures
}

fn unpack_with_progress_bar(
    (src, dst): (PathBuf, PathBuf), pages: UnpackedAtlas, log: &mut Option<impl Write>,
) -> utils::GeneralResult<()> {
    let count: usize = pages.iter().map(|x| x.1.len()).sum();
    let (send, recv) = mpsc::channel::<u64>();
    let handle = thread::spawn(move || -> utils::GeneralResult<()> {
        for page in pages.iter() {
            let source = src.join(&page.0);
            images::unpack_page((&source, &dst), &page.1, Some(&send))?;
        }
        Ok(())
    });
    if log.is_some() {
        let bar = ProgressBar::new(count as u64);
        while let Ok(p) = recv.recv() {
            bar.set_position(bar.position() + p);
        }
        bar.finish_and_clear();
    }
    handle.join().unwrap()
}
