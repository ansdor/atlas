use std::time::Instant;

use clap::Parser;
use utils::{exit_with_error, info_message};

mod atlas;
mod commands;
mod formatting;
mod images;
mod interface;
mod outputs;
mod packing;
mod rectangle;
mod sources;
mod utils;

fn main() {
    use interface::{Cli, Commands::*};
    let start = Instant::now();
    let cmd = Cli::parse();
    let quiet_mode = match cmd.command {
        Pack(ref args) => args.quiet,
        Unpack(ref args) => args.quiet,
        Arrange(ref args) => args.quiet,
        _ => false,
    };

    let mut log = match quiet_mode {
        true => None,
        false => Some(std::io::stdout()),
    };

    if let Err(msg) = match cmd.command {
        Pack(args) => commands::pack(&args, &mut log),
        Unpack(args) => commands::unpack(&args, &mut log),
        Query(args) => commands::query(&args, &mut log),
        Arrange(args) => commands::arrange(&args, &mut log),
    } {
        exit_with_error(&mut log, msg);
    }

    let end = Instant::now().duration_since(start);
    info_message(
        &mut log,
        format!("finished in {:.3} seconds.", end.as_secs_f64()),
    );
}
