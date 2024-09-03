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
    let start = Instant::now();
    let cmd = interface::Cli::parse();
    let quiet_mode = match cmd.command {
        interface::Commands::Pack(ref args) => args.quiet,
        interface::Commands::Unpack(ref args) => args.quiet,
        _ => false,
    };

    let mut log = match quiet_mode {
        true => None,
        false => Some(std::io::stdout()),
    };

    if let Err(msg) = match cmd.command {
        interface::Commands::Pack(args) => commands::pack(&args, &mut log),
        interface::Commands::Unpack(args) => commands::unpack(&args, &mut log),
        interface::Commands::Query(args) => commands::query(&args, &mut log),
        interface::Commands::Arrange(args) => commands::arrange(&args, &mut log)
    } {
        exit_with_error(&mut log, msg);
    }

    let end = Instant::now().duration_since(start);
    info_message(
        &mut log,
        format!("finished in {:.3} seconds.", end.as_secs_f64()),
    );
}
