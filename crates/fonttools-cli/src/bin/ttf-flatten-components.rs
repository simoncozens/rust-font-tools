use fonttools_cli::{open_font, read_args, save_font};

fn main() {
    env_logger::init_from_env(
        env_logger::Env::default().filter_or(env_logger::DEFAULT_FILTER_ENV, "warn"),
    );
    let matches = read_args("ttf-flatten-components", "Flattens components in TTF files");

    let mut infont = open_font(&matches);

    let mut glyf = infont
        .tables
        .glyf()
        .expect("Error reading glyf table")
        .expect("No glyf table found");
    glyf.flatten_components();
    infont.tables.insert(glyf);
    save_font(infont, &matches);
}
