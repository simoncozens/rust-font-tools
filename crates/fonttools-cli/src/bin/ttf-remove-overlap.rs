use fonttools::font;
use fonttools::glyf::{Glyph, Point};
use skia_safe::{simplify, Path};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct Opt {
    input: String,
    output: String,
}

fn draw_glyph(g: &Glyph) {
    if g.is_composite() || g.is_empty() {
        return;
    }
    let mut path = Path::default();
    for contour in g.contours.as_ref().unwrap() {
        path.move_to((contour[0].x as i32, contour[0].y as i32));
        let mut segment: Vec<&Point> = vec![];
        for pt in &contour[1..] {
            segment.push(pt);
            /* This is clearly bogus because of phantom points */
            if pt.on_curve {
                match segment.len() {
                    1 => {
                        path.line_to((segment[0].x as i32, segment[0].y as i32));
                    }
                    2 => {
                        path.quad_to(
                            (segment[0].x as i32, segment[0].y as i32),
                            (segment[1].x as i32, segment[1].y as i32),
                        );
                    }
                    3 => {
                        path.cubic_to(
                            (segment[0].x as i32, segment[0].y as i32),
                            (segment[1].x as i32, segment[1].y as i32),
                            (segment[2].x as i32, segment[2].y as i32),
                        );
                    }
                    _ => {}
                };
                segment = vec![];
            }
        }
        path.close();
    }
    if let Some(newpath) = simplify(&path) {
        if newpath != path {
            // println!("Removed overlap!");
        }
    }
}

fn main() {
    let opts: Opt = Opt::from_args();
    let mut infont = font::load(&opts.input).expect("Could not parse font");
    let names = infont
        .get_table(b"post")
        .unwrap()
        .unwrap()
        .post_unchecked()
        .glyphnames
        .as_ref()
        .unwrap()
        .clone();
    let glyf = infont.get_table(b"glyf").unwrap().unwrap().glyf_unchecked();
    for (i, glyph) in glyf.glyphs.iter().enumerate() {
        if let Some(glyph) = glyph {
            // println!("glyph ID: {:?} ({:})", i, names[i]);
            draw_glyph(glyph);
        }
    }

    infont.save(&opts.output);
}
