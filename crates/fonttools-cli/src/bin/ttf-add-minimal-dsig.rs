use fonttools::font::Table;
use fonttools_cli::{open_font, read_args, save_font};

fn main() {
    let matches = read_args(
        "ttf-add-minimal-dsig",
        "Adds a minimal DSIG table if one is not present",
    );
    let mut infont = open_font(&matches);

    if !infont.tables.contains_key(b"DSIG") {
        infont.tables.insert(
            *b"DSIG",
            Table::Unknown(vec![0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00]),
        );
    }
    save_font(infont, &matches);
}
