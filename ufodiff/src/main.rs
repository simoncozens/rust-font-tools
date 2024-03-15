use std::collections::HashSet;

use clap::{App, Arg};
use norad::Font;

mod diff;
use crate::diff::Diff;

fn report_diffs(title: &str, diffs: diff::DiffResult) {
    if diffs.is_empty() {
        println!("{}: No differences found", title);
    } else {
        println!("{}: ", title);
        for (key, value) in diffs {
            println!("\t{:30} {}", key, value);
        }
    }
    println!();
}

fn main() {
    let matches = App::new("ufodiff")
        .version("1.0")
        .author("Simon Cozens")
        .about("Compare two UFO font files")
        .arg(
            Arg::with_name("ufo1")
                .required(true)
                .index(1)
                .help("Path to the first UFO font file"),
        )
        .arg(
            Arg::with_name("ufo2")
                .required(true)
                .index(2)
                .help("Path to the second UFO font file"),
        )
        .get_matches();

    let ufo1_path = matches.value_of("ufo1").unwrap();
    let ufo2_path = matches.value_of("ufo2").unwrap();

    // Open the UFO font files
    let ufo1 = Font::load(ufo1_path).expect("Failed to open UFO font file 1");
    let ufo2 = Font::load(ufo2_path).expect("Failed to open UFO font file 2");

    // Compare the UFO font files
    report_diffs("fontinfo.plist", ufo1.font_info.diff(&ufo2.font_info));
    report_diffs("lib.plist", ufo1.lib.diff(&ufo2.lib));
    report_diffs("groups.plist", ufo1.groups.diff(&ufo2.groups));
    report_diffs("kerning.plist", ufo1.kerning.diff(&ufo2.kerning));

    // Compare the features
    if ufo1.features != ufo2.features {
        println!("Features differ");
    }

    // Compare layers
    let all_layers: HashSet<&norad::Name> =
        ufo1.layers.names().chain(ufo2.layers.names()).collect();
    for layername in all_layers {
        let layer1 = ufo1.layers.get(layername);
        let layer2 = ufo2.layers.get(layername);
        if layer1.is_none() {
            println!("Layer {} is in UFO 2 but not in UFO 1", layername);
        } else if layer2.is_none() {
            println!("Layer {} is in UFO 1 but not in UFO 2", layername);
        } else {
            report_diffs(
                &format!("Layer {}", layername),
                layer1.unwrap().diff(layer2.unwrap()),
            );
        }
    }
}
