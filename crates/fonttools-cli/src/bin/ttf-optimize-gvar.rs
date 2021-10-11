use fonttools::font::Table;
use fonttools::tables::{glyf, gvar};
use fonttools::tag;
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
    infont.get_table(tag!("head")).unwrap();
    infont.get_table(tag!("maxp")).unwrap();
    let loca_offsets = infont
        .get_table(tag!("loca"))
        .unwrap()
        .unwrap()
        .loca_unchecked()
        .indices
        .clone();

    // This mad dance is necessary to avoid mutably deserializing twice.
    if let Table::Unknown(binary_gvar) = infont.tables.get(&tag!("gvar")).unwrap() {
        if let Table::Unknown(binary_glyf) = infont.tables.get(&tag!("glyf")).unwrap() {
            let glyf =
                glyf::from_bytes(binary_glyf, loca_offsets).expect("Could not read glyf table");
            let coords_and_ends = glyf
                .glyphs
                .iter()
                .map(|g| g.gvar_coords_and_ends())
                .collect();
            let gvar = gvar::from_bytes(&binary_gvar, coords_and_ends)
                .expect("Couldn't deserialize gvar table");

            // Passing in the glyf table here is what causes the IUP optimization
            let new_gvar = Table::Unknown(gvar.to_bytes(Some(&glyf)));
            infont.tables.insert(tag!("gvar"), new_gvar);
        }
    }
    save_font(infont, &matches);
}
