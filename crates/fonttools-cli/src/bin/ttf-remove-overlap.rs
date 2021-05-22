use fonttools::font::Table;
use fonttools::glyf::{Glyph, Point};
use fonttools_cli::{open_font, read_args, save_font};

use skia_safe::{simplify, Path};

fn remove_overlap(g: &mut Glyph) {
    if g.has_components() || g.is_empty() {
        return;
    }
    let mut path = Path::default();
    g.insert_explicit_oncurves();
    for contour in &g.contours {
        path.move_to((contour[0].x as i32, contour[0].y as i32));
        let mut segment: Vec<&Point> = vec![];
        for pt in &contour[1..] {
            segment.push(pt);
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
        if !segment.is_empty() {
            path.quad_to(
                (segment[0].x as i32, segment[0].y as i32),
                (contour[0].x as i32, contour[0].y as i32),
            );
        }
        path.close();
    }
    if let Some(newpath) = simplify(&path) {
        g.contours = skia_to_glyf(newpath);
    }
}

fn skia_to_glyf(p: Path) -> Vec<Vec<Point>> {
    let points_count = p.count_points();
    let mut points = vec![skia_safe::Point::default(); points_count];
    let _count_returned = p.get_points(&mut points);

    let verb_count = p.count_verbs();
    let mut verbs = vec![0_u8; verb_count];
    let _count_returned_verbs = p.get_verbs(&mut verbs);
    let mut new_contour: Vec<Point> = vec![];
    let mut new_glyph: Vec<Vec<Point>> = vec![];
    let mut cur_pt = 0;
    for verb in verbs {
        if verb > 4 {
            new_contour.reverse();
            new_glyph.push(new_contour);
            new_contour = vec![];
            continue;
        }
        let point_count = if verb < 2 { 1 } else { 2 };
        for _ in 0..point_count {
            new_contour.push(Point {
                x: points[cur_pt].x as i16,
                y: points[cur_pt].y as i16,
                on_curve: true,
            });
            cur_pt += 1;
        }
    }
    new_glyph
}

fn main() {
    let matches = read_args("ttf-remove-overlap", "Removes overlap from TTF files");
    let mut infont = open_font(&matches);
    if let Table::Glyf(glyf) = infont.get_table(b"glyf").unwrap().unwrap() {
        for glyph in glyf.glyphs.iter_mut() {
            remove_overlap(glyph);
        }
        glyf.recalc_bounds();
    }
    save_font(infont, &matches);
}
