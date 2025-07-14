use std::io::Write;

use super::pack;
use crate::{interface, utils};

pub fn query(
    args: &interface::QueryArguments, log: &mut Option<impl Write>,
) -> utils::GeneralResult<()> {
    let mut results = Vec::new();
    let pack_arguments_list = generate_mock_packing_arguments(args);
    for p in pack_arguments_list.into_iter() {
        let mut sink = if log.is_some() {
            Some(std::io::sink())
        } else {
            None
        };
        utils::info_message(log, format!("packing with {}", describe_settings(&p)));
        let packer = pack::pack_textures(&p, &mut sink)?;
        let efficiency = packer.efficiency();
        results.push((p, packer, efficiency));
    }
    //sort results by efficiency
    results.sort_unstable_by(|a, b| b.2.total_cmp(&a.2));
    //build a string buffer with the report
    let ruler_width = 72;
    let mut report = String::new();
    report += format!(
        "\n{0:<40}{1:>16}{2:>16}\n",
        "SETTINGS", "SIZE", "EFFICIENCY"
    )
    .as_str();
    report += format!("{}\n", "-".repeat(ruler_width)).as_str();
    results.iter().for_each(|x| {
        let (settings, packer, efficiency) = x;
        let (count, (w, h)) = (packer.pages.len(), packer.page_size());
        report += format!(
            "{0:<40}{1:>16}{2:>16}\n",
            describe_settings(settings),
            format!("{}p, {}x{}", count, w, h),
            format!("{:.2}%", efficiency)
        )
        .as_str();
    });
    report += format!("{}\n", "-".repeat(ruler_width)).as_str();
    report += "for the most efficient packing of these sources, use this command:\n";
    report += format!(
        "\tatlas pack {}[sources] [output]\n\n",
        describe_args(&results[0].0)
    )
    .as_str();
    report += "if texture rotation is not allowed, use this command:\n";
    report += format!(
        "\tatlas pack {}[sources] [output]\n",
        describe_args(&results.iter().find(|x| { !x.0.rotate }).unwrap().0)
    )
    .as_str();
    report += "-".repeat(ruler_width).as_str();
    utils::info_message(log, report);
    Ok(())
}

fn describe_args(args: &interface::PackArguments) -> String {
    let mut r = String::new();
    r += if args.short_side_sort { "--short " } else { "" };
    r += if args.pack_by_area { "--area " } else { "" };
    r += if args.rotate { "--rotate " } else { "" };
    r
}

fn describe_settings(args: &interface::PackArguments) -> String {
    let sorting = match args.short_side_sort {
        true => "short side",
        false => "long side",
    };
    let packing = match args.pack_by_area {
        true => "area",
        false => "distance",
    };
    let rotation = match args.rotate {
        true => "rotation",
        false => "no rotation",
    };
    format!("{sorting}, {packing}, {rotation}")
}

fn generate_mock_packing_arguments(
    query_args: &interface::QueryArguments,
) -> Vec<interface::PackArguments> {
    let sorting_options = [Some(false), Some(true)];
    let packing_options = [false, true];
    let rotate_options = [false, true];
    let default_settings = interface::PackArguments {
        sources: query_args.sources.clone(),
        output: String::from("query"),
        overwrite: false,
        spacing: query_args.spacing,
        page_size: query_args.page_size.clone(),
        quiet: false,
        format: None,
        pack_by_area: false,
        short_side_sort: false,
        rotate: false,
        power_of_two: false,
        include_duplicates: query_args.include_duplicates,
    };

    let mut r = Vec::new();
    for sort in sorting_options {
        for pack in packing_options {
            for rotation in rotate_options {
                r.push(interface::PackArguments {
                    //this is always Some()
                    short_side_sort: sort.unwrap(),
                    pack_by_area: pack,
                    rotate: rotation,
                    ..default_settings.clone()
                })
            }
        }
    }
    r
}
