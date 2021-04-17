use fonttools::font;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    input: String,
    output: String,
}

fn main() {
    let opts: Opt = Opt::from_args();
    let infont = font::load(&opts.input).unwrap();
    infont.save(&opts.output);
}
