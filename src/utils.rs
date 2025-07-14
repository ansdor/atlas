use std::{
    fmt::Display,
    io::Write,
    path::{Path, PathBuf},
};

pub type GeneralError = Box<dyn std::error::Error + Send + Sync>;
pub type GeneralResult<T> = Result<T, GeneralError>;

pub fn info_message<S: Write, T: Display>(sink: &mut Option<S>, msg: T) {
    if let Some(sink) = sink {
        let _ = writeln!(sink, "[INFO] {msg}");
    }
}

pub fn exit_with_error<S: Write, T: Display>(sink: &mut Option<S>, msg: T) -> ! {
    if let Some(sink) = sink {
        let _ = writeln!(sink, "[ERROR] {msg}");
    }
    std::process::exit(1);
}

pub fn append_to_filename<T: AsRef<Path>>(path: T, suffix: &str) -> PathBuf {
    let path = path.as_ref();
    let mut r = path.to_owned();
    let name = r
        .file_stem()
        .unwrap_or(Path::new("").as_os_str())
        .to_string_lossy();
    r.set_file_name(String::from(name + suffix));
    if let Some(ext) = path.extension() {
        r.set_extension(ext);
    }
    r
}
