use std::{io::Write, path::PathBuf};

use super::pack;
use crate::{
    interface, outputs, packing::{self, TexturePacker}, sources::{self}, utils
};

pub fn arrange(
    args: &interface::ArrangeArguments, log: &mut Option<impl Write>,
) -> utils::GeneralResult<()> {
    let source_settings = sources::SourceSettings {
        sorting: sources::SortingMethod::LongSide,
        deduplicate: false
    };
    let packing_settings = packing::generate_arrange_settings(args)?;
    let sources = sources::prepare_sources(&args.sources, &pack::EXTENSIONS, Some(source_settings))?;
    let label = match PathBuf::from(&args.output).file_stem() {
        Some(stem) => stem.to_string_lossy().to_string(),
        None => return Err(format!("unable to extract filename from '{}'.", args.output).into()),
    };
    let packer = TexturePacker::new(&label, sources, packing_settings);
    let packer = pack::pack_with_progress_bar(packer, log)?;
    pack::print_packing_report(&packer, log);
    let destination = outputs::prepare_output_directory(&args.output)?;
    pack::generate_image_files(destination, packer, args.overwrite, log)
}
