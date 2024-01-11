use std::{env, path::{PathBuf, Path}, fs};

use crate::utils;

pub fn prepare_output_directory(out_path: &str) -> utils::GeneralResult<PathBuf> {
    //unwrap the command line argument
    let path = PathBuf::from(out_path);
    //transform a relative path into absolute, if necessary
    let path = if path.is_relative() {
        env::current_dir()?.join(path)
    } else {
        path
    };
    //retrieve the parent directory, where
    //the output files will be created
    let out_dir = match path.parent() {
        Some(d) => d,
        None => return Err(format!("invalid output path '{}'", path.display()).into())
    };
    //if the path doesn't exist, try to create it
    if !out_dir.exists() {
        fs::create_dir_all(out_dir)?;
    }
    let out_dir = fs::canonicalize(out_dir)?;
    //return the correct absolute path
    Ok(out_dir)
}

pub fn check_overwrite<P: AsRef<Path>>(path: P, overwrite: bool) -> utils::GeneralResult<Option<String>> {
    let path = path.as_ref();
    match (path.exists(), overwrite) {
        //if the file exists and overwriting is allowed, return Ok with a message
        (true, true) => Ok(Some(format!("overwriting file: {}", path.display()))),
        //if the file exists and overwriting isn' allowed, return Err with a message
        (true, false) => Err(format!(
            "file '{}' already exists. use the -o flag to overwrite.",
            path.display()
        )
        .into()),
        //if the file does not exist, return Ok without a message
        (false, _) => Ok(None),
    }
}
