use clap::{App, Arg, SubCommand};
use fonttools::font;
use fonttools::font::Table;
use itertools::Itertools;
use std::collections::{BTreeMap, HashSet};
use std::fs::File;
use std::io;

fn build_production_name(name: &str, unicodes: Option<&HashSet<u32>>) -> String {
    if unicodes.is_none() {
        return name.to_string();
    }
    let first: u32 = *unicodes.unwrap().iter().sorted().next().unwrap();
    if first > 0xFFFF {
        format!("u{:04X}", first)
    } else {
        format!("uni{:04X}", first)
    }
}

fn main() {
    let matches = App::new("ttf-remove-overlap")
        .about("Removes overlap from TTF files")
        .arg(Arg::with_name("drop-names"))
        .arg(
            Arg::with_name("INPUT")
                .help("Sets the input file to use")
                .required(false),
        )
        .arg(
            Arg::with_name("OUTPUT")
                .help("Sets the output file to use")
                .required(false),
        )
        .get_matches();
    let mut infont = if matches.is_present("INPUT") {
        let filename = matches.value_of("INPUT").unwrap();
        let infile = File::open(filename).unwrap();
        font::load(infile)
    } else {
        font::load(io::stdin())
    }
    .expect("Could not parse font");
    let has_cff = infont.tables.contains_key(b"CFF ");
    let num_glyphs = infont.num_glyphs();
    let mut reversed_map = BTreeMap::new();

    if let Table::Cmap(cmap) = infont
        .get_table(b"cmap")
        .expect("Error reading cmap table")
        .expect("No cmap table found")
    {
        reversed_map = cmap.reversed();
    }
    if let Table::Post(post) = infont
        .get_table(b"post")
        .expect("Error reading post table")
        .expect("No post table found")
    {
        if matches.is_present("drop-names") {
            if has_cff {
                println!("Dropping glyph names from CFF 1.0 is a bad idea!");
            }
            post.set_version(3.0);
        } else {
            let glyphnames = post
                .glyphnames
                .as_mut()
                .expect("post table didn't have any names");
            for i in 0..num_glyphs {
                let prod_name =
                    build_production_name(&glyphnames[i as usize], reversed_map.get(&i));
                glyphnames[i as usize] = prod_name;
            }
        }
    }

    if matches.is_present("OUTPUT") {
        let mut outfile = File::create(matches.value_of("OUTPUT").unwrap())
            .expect("Could not open file for writing");
        infont.save(&mut outfile);
    } else {
        infont.save(&mut io::stdout());
    };
}
