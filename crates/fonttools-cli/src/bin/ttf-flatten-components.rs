use fonttools::font::Table;
use fonttools_cli::{open_font, read_args, save_font};

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "warn"),
    );
    let matches = read_args("ttf-flatten-components", "Flattens components in TTF files");

    let mut infont = open_font(&matches);

    if let Table::Glyf(glyf) = infont
        .get_table(b"glyf")
        .expect("Error reading glyf table")
        .expect("No glyf table found")
    {
        glyf.flatten_components()
    }
    save_font(infont, &matches);
}
