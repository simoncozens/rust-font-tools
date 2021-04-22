use fonttools_cli::{open_font, read_args, save_font};

fn main() {
    let matches = read_args(
        "ttf-fix-checksum",
        "Ensures TTF files have correct checksum",
    );
    let infont = open_font(&matches);
    save_font(infont, &matches);
}
