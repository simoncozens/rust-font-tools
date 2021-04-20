use fonttools::font;
use fonttools::font::Table;
use itertools::Itertools;
use std::collections::{BTreeMap, HashSet};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    #[structopt(long)]
    drop_names: bool,
    input: String,
    output: String,
}

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
    let opts: Opt = Opt::from_args();
    let mut infont = font::load(&opts.input).expect("Could not parse font");
    let has_cff = infont.tables.contains_key(b"CFF ");
    let num_glyphs = infont.num_glyphs();
    let mut reversed_map = BTreeMap::new();

    if let Table::Cmap(cmap) = infont.tables.get_mut(b"cmap").expect("No cmap table found") {
        reversed_map = cmap.reversed();
    }
    if let Table::Post(post) = infont.tables.get_mut(b"post").expect("No post table found") {
        if opts.drop_names {
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
                println!("{:} = {:}", i, prod_name);
                glyphnames[i as usize] = prod_name;
            }
        }
    }

    infont.save(&opts.output);
}
