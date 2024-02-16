use clap::{App, Arg};
use norad::Font;

mod diff;
use crate::diff::Diff;

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
    let diffs = ufo1.font_info.diff(&ufo2.font_info);
    // Print the comparison result
    print!("fontinfo.plist: ");
    if diffs.is_empty() {
        println!("No differences found");
    } else {
        println!();
        for (key, value) in diffs {
            println!("\t{:30}{}", key, value);
        }
    }

    let diffs = ufo1.lib.diff(&ufo2.lib);
    print!("\nlib.plist: ");
    if diffs.is_empty() {
        println!("No differences found");
    } else {
        println!();
        for (key, value) in diffs {
            println!("\t{:30}{}", key, value);
        }
    }

    let diffs = ufo1.layers.default_layer().diff(ufo2.default_layer());
    if diffs.is_empty() {
        println!("No differences found");
    } else {
        println!();
        for (key, value) in diffs {
            println!("\t{:30}{}", key, value);
        }
    }
    // TODO: Print the comparison result
}
