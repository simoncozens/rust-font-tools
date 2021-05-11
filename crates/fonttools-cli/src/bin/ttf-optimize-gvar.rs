use fonttools::font::Table;
use fonttools_cli::{open_font, read_args, save_font};

fn main() {
    let matches = read_args(
        "ttf-optimize-gvar",
        "Optimizes the gvar table by omitting points which can be inferred",
    );
    let mut infont = open_font(&matches);

    if !infont.tables.contains_key(b"gvar") {
        save_font(infont, &matches);
        return;
    }
    infont.get_table(b"head").unwrap();
    infont.get_table(b"maxp").unwrap();
    infont.get_table(b"loca").unwrap();
    infont.get_table(b"glyf").unwrap();

    let gvar = infont
        .get_table(b"gvar")
        .expect("Couldn't load glyf table")
        .unwrap()
        .gvar_unchecked()
        .clone();
    let glyf = infont
        .get_table(b"glyf")
        .expect("Couldn't load glyf table")
        .unwrap()
        .glyf_unchecked();
    let new_gvar = Table::Unknown(gvar.to_bytes(Some(glyf)));
    infont.tables.insert(*b"gvar", new_gvar);
    save_font(infont, &matches);
}
