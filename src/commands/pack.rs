use std::{
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::mpsc,
    thread,
};

use indicatif::ProgressBar;
use packing::TexturePacker;
use utils::info_message;

use crate::{atlas, images, interface, outputs, packing, sources, utils};

pub const EXTENSIONS: [&str; 1] = ["png"];

pub fn pack(args: &interface::PackArguments, log: &mut Option<impl Write>) -> utils::GeneralResult<()> {
    let packer = pack_textures(args, log)?;
    print_packing_report(&packer, log);
    generate_output_files(args, packer, log)
}

pub fn pack_textures(args: &interface::PackArguments, log: &mut Option<impl Write>) -> utils::GeneralResult<TexturePacker> {
    let source_settings = sources::generate_settings(args);
    let packing_settings = packing::generate_settings(args)?;
    let sources = sources::acquire_sources(&args.sources, &EXTENSIONS, &source_settings)?;
    //check if page size is large enough to fit all the images
    if packing_settings.page_size.is_some() {
        sources::validate_dimensions(&sources, &packing_settings)?;
    }
    //if there are duplicate images among the sources, list them
    if let Some((_, msg)) = sources::report_duplicates(&sources) {
        info_message(log, msg);
    }
    //extract the filename stem from the output command line argument
    let label = match PathBuf::from(&args.output).file_stem() {
        Some(stem) => stem.to_string_lossy().to_string(),
        None => return Err(format!("unable to extract filename from '{}'.", args.output).into()),
    };
    let packer = TexturePacker::new(&label, sources, packing_settings);
    //perform the rectangle packing on a separate thread, return the packer on sucess
    pack_with_progress_bar(packer, log)
}

fn pack_with_progress_bar(mut packer: TexturePacker, log: &mut Option<impl Write>) -> utils::GeneralResult<TexturePacker> {
    let (sources, duplicates) = (packer.count(), packer.duplicates());
    let (send, recv) = mpsc::channel::<u64>();
    let handle = thread::spawn(move || match packer.pack_everything(Some(send)) {
        Ok(_) => Ok(packer),
        Err(msg) => Err(msg),
    });
    if log.is_some() {
        let bar = ProgressBar::new(sources.saturating_sub(duplicates) as u64);
        while let Ok(p) = recv.recv() {
            bar.set_position(bar.position() + p);
        }
        bar.finish_and_clear();
    }
    match handle.join() {
        Ok(handle_result) => handle_result,
        Err(_) => Err("failed to join threads.".into()),
    }
}

pub fn print_packing_report(packer: &TexturePacker, log: &mut Option<impl Write>) {
    let page_size = packer.page_size();
    let page_count = packer.pages.len();
    //report the data
    info_message(log, format!(
        "generated {} page{}, size {}x{}.",
        page_count,
        if page_count == 1 { "" } else { "s" },
        page_size.0,
        page_size.1
    ));
    info_message(log, format!("packing efficiency: {:.2}%.", packer.efficiency()).as_str());
}

fn generate_output_files(args: &interface::PackArguments, packer: TexturePacker, log: &mut Option<impl Write>) -> utils::GeneralResult<()> {
    let destination = outputs::prepare_output_directory(&args.output)?;
    let extension = match args.format {
        Some(interface::OutputFormat::Text) => "txt",
        _ => "json"
    };
    let description_file = Path::new(&destination).join(format!("{}.{}", &packer.label, extension));
    if let Some(msg) = outputs::check_overwrite(&description_file, args.overwrite)? {
        info_message(log, msg);
    }
    if let Some(description) = atlas::generate_description(args, &packer) {
        let mut description_handle = File::create(&description_file)?;
        description_handle.write_all(description.as_bytes())?;
    } else {
        return Err("unable to generate description file".into());
    }
    for page in packer.pages.into_iter() {
        let image_path = Path::new(&destination).join(format!("{}.png", &page.name));
        if let Some(msg) = outputs::check_overwrite(&image_path, args.overwrite)? {
            info_message(log, msg);
        }
        images::generate_image(page, &image_path)?;
    }
    Ok(())
}
