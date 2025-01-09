use std::{
    cmp,
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

pub fn pack(
    args: &interface::PackArguments, log: &mut Option<impl Write>,
) -> utils::GeneralResult<()> {
    let packer = pack_textures(args, log)?;
    print_packing_report(&packer, log);
    generate_output_files(args, packer, log)
}

pub fn pack_textures(
    args: &interface::PackArguments, log: &mut Option<impl Write>,
) -> utils::GeneralResult<TexturePacker> {
    //let source_settings = sources::generate_settings(args);
    let packing_settings = packing::generate_packing_settings(args)?;
    let sources = prepare_sources(&args.sources, &EXTENSIONS, &packing_settings)?;
    //check if page size is large enough to fit all the images
    if let Some(page_size) = packing_settings.page_size {
        sources::validate_dimensions(&sources, page_size, packing_settings.spacing)?;
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

pub fn pack_with_progress_bar(
    mut packer: TexturePacker, log: &mut Option<impl Write>,
) -> utils::GeneralResult<TexturePacker> {
    let (sources, duplicates) = (packer.count(), packer.duplicates());
    let (send, recv) = mpsc::channel::<u64>();
    let handle = thread::spawn(move || {
        let r = match packer.settings.arrange {
            Some(_) => packer.arrange_everything(Some(send)),
            None => packer.pack_everything(Some(send)),
        };
        match r {
            Ok(_) => Ok(packer),
            Err(msg) => Err(msg),
        }
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
    info_message(
        log,
        format!(
            "generated {} page{}, size {}x{}.",
            page_count,
            if page_count == 1 { "" } else { "s" },
            page_size.0,
            page_size.1
        ),
    );
    info_message(
        log,
        format!("packing efficiency: {:.2}%.", packer.efficiency()).as_str(),
    );
}

pub fn prepare_sources<P: AsRef<Path>>(
    sources: &[P], extensions: &[&str], settings: &packing::PackingSettings,
) -> utils::GeneralResult<Vec<sources::SourceTexture>> {
    // if settings were not provided, use the defaults
    let settings = match settings.source_treatment.as_ref() {
        Some(v) => v,
        None => &Default::default(),
    };
    let mut info = sources::source_list_from_paths(sources, extensions)?;

    use sources::SourceTexture;
    fn short_side_sort(a: &SourceTexture, b: &SourceTexture) -> cmp::Ordering {
        cmp::min(b.dimensions.width, b.dimensions.height)
            .cmp(&cmp::min(a.dimensions.width, a.dimensions.height))
    }

    fn long_side_sort(a: &SourceTexture, b: &SourceTexture) -> cmp::Ordering {
        cmp::max(b.dimensions.width, b.dimensions.height)
            .cmp(&cmp::max(a.dimensions.width, a.dimensions.height))
    }
    //sort the textures according to the settings
    match settings.sorting {
        packing::SortingMethod::ShortSide => info.sort_by(|a, b| {
            short_side_sort(a, b)
                .then(long_side_sort(a, b))
                .then(human_sort::compare(&a.name, &b.name))
        }),
        packing::SortingMethod::LongSide => info.sort_by(|a, b| {
            long_side_sort(a, b)
                .then(short_side_sort(a, b))
                .then(human_sort::compare(&a.name, &b.name))
        }),
    }
    sources::solve_name_collisions(&mut info);
    if settings.deduplicate {
        sources::deduplicate_textures(&mut info)?;
    }
    //return the vector with all the source texture information
    Ok(info)
}

pub fn generate_image_files<P: AsRef<Path>>(
    destination: P, packer: TexturePacker, overwrite: bool, log: &mut Option<impl Write>,
) -> utils::GeneralResult<()> {
    for page in packer.pages.into_iter() {
        let image_path = Path::new(destination.as_ref()).join(format!("{}.png", &page.name));
        if let Some(msg) = outputs::notify_overwrite(&image_path, overwrite)? {
            info_message(log, msg);
        }
        images::generate_image(page, &image_path)?;
    }
    Ok(())
}

fn generate_output_files(
    args: &interface::PackArguments, packer: TexturePacker, log: &mut Option<impl Write>,
) -> utils::GeneralResult<()> {
    let destination =
        outputs::prepare_output_directory(&args.output, outputs::PathType::Files, log)?;
    let extension = match args.format {
        Some(interface::OutputFormat::Text) => "txt",
        _ => "json",
    };
    let description_file = Path::new(&destination).join(format!("{}.{}", &packer.label, extension));
    if let Some(msg) = outputs::notify_overwrite(&description_file, args.overwrite)? {
        info_message(log, msg);
    }
    if let Some(description) = atlas::generate_description(args, &packer) {
        let mut description_handle = File::create(&description_file)?;
        description_handle.write_all(description.as_bytes())?;
    } else {
        return Err("unable to generate description file".into());
    }
    generate_image_files(destination, packer, args.overwrite, log)
}
