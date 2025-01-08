use std::{
    env, fs, io::Write, path::{Path, PathBuf}
};

use crate::utils;

pub enum PathType {
    Files,
    Directory
}

pub fn prepare_output_directory(path: &str, path_type: PathType, log: &mut Option<impl Write>) -> utils::GeneralResult<PathBuf> {
    //create a pathbuf from the string passed to this function
    let path = {
        let p = PathBuf::from(path);
        if p.is_relative() {
            env::current_dir()?.join(p)
        } else {
            p
        }
    };
    //retrieve the parent directory, where
    //the output files will be created
    let dir = match path_type {
        PathType::Files => match path.parent() {
            Some(p) => p,
            None => return Err(format!("invalid output path '{}'", path.display()).into())
        },
        PathType::Directory => &path
    };
    match dir {
        x if x.is_file() => Err(format!("invalid output directory: a file named '{}' already exists.", dir.display()).into()),
        x if !x.exists() => {
            utils::info_message(log, "output directory does not exist.");
            std::fs::create_dir_all(dir)?;
            utils::info_message(log, format!("directory '{}' created successfully.", dir.display()));
            Ok(fs::canonicalize(dir)?)
        }
        _ => Ok(fs::canonicalize(dir)?)
    }
}

pub fn notify_overwrite<P: AsRef<Path>>(path: P, overwrite_allowed: bool) -> utils::GeneralResult<Option<String>> {
    let p = path.as_ref();
    match (p.exists(), overwrite_allowed) {
        // no overwrite needed, return silent success
        (false, _) => Ok(None),
        // the program is allowed to overwrite
        (_, true) => Ok(Some(format!("overwriting file: {}", p.display()))),
        // the program is not allowed to overwrite
        (_, false) => Err(format!("file '{}' already exists. use the -o flag to enable overwriting.", p.display()).into())
    }
}

pub fn count_overwrites<P: AsRef<Path>>(paths: &[P]) -> u32 {
    paths.iter().map(|p| if p.as_ref().exists() { 1 } else { 0 }).sum()
}
