use fonttools::tag;
use fonttools_cli::{open_font, read_args, save_font};

fn main() {
    let matches = read_args(
        "ttf-optimize-gvar",
        "Optimizes the gvar table by omitting points which can be inferred",
    );
    let mut infont = open_font(&matches);

    if !infont.tables.contains(&tag!("gvar")) {
        save_font(infont, &matches);
        return;
    }
    let _head = infont.tables.head().unwrap().unwrap();
    let _maxp = infont.tables.maxp().unwrap().unwrap();
    let _loca_offsets = infont.tables.loca().unwrap().unwrap().indices.clone();

    let glyf = infont.tables.glyf().unwrap().unwrap();
    let gvar = infont.tables.gvar().unwrap().unwrap();
    infont
        .tables
        .insert_raw(tag!("gvar"), gvar.to_bytes(Some(&glyf)));
    save_font(infont, &matches);
}
