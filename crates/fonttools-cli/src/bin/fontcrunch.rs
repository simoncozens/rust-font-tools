use fonttools::font::Table;
use fonttools::glyf::contourutils::{glyf_contour_to_kurbo_contour, kurbo_contour_to_glyf_contour};
use fonttools::glyf::Glyph;
use fonttools_cli::{open_font, read_args, save_font};
use kurbo::{BezPath, PathSeg, Point, QuadBez};
use rayon::iter::IndexedParallelIterator;
use std::collections::BTreeSet;

const NORM_LEVEL: i32 = 2;
const DIST_FACTOR: f64 = 0.005;
const ANGLE_FACTOR: f64 = 5.0;

use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;

fn f64_is_close(a: f64, b: f64) -> bool {
    (a - b).abs() < 1e-8 // f64::EPSILON
}

const HALF_STEP: bool = true;

fn rk4<T>(y: &mut Vec<f64>, x: f64, h: f64, derivs: &T)
where
    T: Apply,
{
    let mut dydx: Vec<f64> = std::iter::repeat(0_f64).take(y.len()).collect();
    let mut dyt: Vec<f64> = std::iter::repeat(0_f64).take(y.len()).collect();
    let mut dym: Vec<f64> = std::iter::repeat(0_f64).take(y.len()).collect();
    let mut yt: Vec<f64> = std::iter::repeat(0_f64).take(y.len()).collect();
    derivs.apply(&mut dydx, x, y);
    let hh = h * 0.5;
    let h6 = h / 6.0;
    for i in 0..y.len() {
        yt[i] = y[i] + hh * dydx[i]
    }
    derivs.apply(&mut dyt, x + hh, &mut yt);
    for i in 0..y.len() {
        yt[i] = y[i] + hh * dyt[i]
    }
    derivs.apply(&mut dym, x + hh, &mut yt);
    for i in 0..y.len() {
        yt[i] = y[i] + h * dym[i];
        dym[i] += dyt[i];
    }
    derivs.apply(&mut dyt, x + h, &mut yt);
    for i in 0..y.len() {
        y[i] += h6 * (dydx[i] + dyt[i] + 2.0 * dym[i]);
    }
}

fn intersect(p0: Point, dir0: Point, p1: Point, dir1: Point) -> Option<Point> {
    let det = dir0.x * dir1.y - dir0.y * dir1.x;
    if det.abs() < f64::EPSILON {
        return None;
    };
    let a = p0.y * dir0.x - p0.x * dir0.y;
    let b = p1.y * dir1.x - p1.x * dir1.y;
    Some(Point::new(
        (a * dir1.x - b * dir0.x) / det,
        (a * dir1.y - b * dir0.y) / det,
    ))
}

trait PointMonkeyPatch {
    fn is_close(&self, other: Self) -> bool;
    fn square_distance(&self, other: Self) -> f64;
    fn unitize(&self) -> Self;
}
impl PointMonkeyPatch for Point {
    fn is_close(&self, other: Self) -> bool {
        f64_is_close(self.x, other.x) && f64_is_close(self.y, other.y)
    }
    fn square_distance(&self, other: Self) -> f64 {
        let dx = self.x - other.x;
        let dy = self.y - other.y;
        dx * dx + dy * dy
    }
    fn unitize(&self) -> Self {
        let scale = 1.0 / self.distance(Point::ZERO);
        Point::new(self.x * scale, self.y * scale)
    }
}

trait QBMonkeyPatch {
    fn is_line(&self) -> bool;
    fn point_at_t(&self, t: f64) -> Point;
    fn arclen(&self) -> f64;
}
impl QBMonkeyPatch for QuadBez {
    fn is_line(&self) -> bool {
        self.p1.is_close(self.p0.lerp(self.p2, 0.5))
    }
    fn point_at_t(&self, t: f64) -> Point {
        let p01 = self.p0.lerp(self.p1, t);
        let p12 = self.p1.lerp(self.p2, t);
        p01.lerp(p12, t)
    }
    fn arclen(&self) -> f64 {
        let derivs = ArclenFunctor::new(self);
        let n = 10;
        let dt = 1.0 / 10.0;
        let mut t = 0.0;
        let mut y = vec![0.0];
        for _ in 0..n {
            rk4(&mut y, t, dt, &derivs);
            t += dt;
        }
        y[0]
    }
}

struct ArclenFunctor {
    dx0: f64,
    dy0: f64,
    dx1: f64,
    dy1: f64,
}

impl ArclenFunctor {
    fn new(q: &QuadBez) -> Self {
        ArclenFunctor {
            dx0: 2.0 * (q.p1.x - q.p0.x),
            dy0: 2.0 * (q.p1.y - q.p0.y),
            dx1: 2.0 * (q.p2.x - q.p1.x),
            dy1: 2.0 * (q.p2.y - q.p1.y),
        }
    }
    fn deriv(&self, t: f64) -> Point {
        Point::new(
            self.dx0 + t * (self.dx1 - self.dx0),
            self.dy0 + t * (self.dy1 - self.dy0),
        )
    }
}

trait Apply {
    fn apply(&self, dydx: &mut Vec<f64>, t: f64, y: &mut Vec<f64>);
}

impl Apply for ArclenFunctor {
    fn apply(&self, dydx: &mut Vec<f64>, t: f64, _y: &mut Vec<f64>) {
        let p = self.deriv(t);
        dydx[0] = p.distance(Point::ZERO);
    }
}

struct MeasureFunctor<'a> {
    curve: &'a Thetas,
    s0: f64,
    ss: f64,
    af: ArclenFunctor,
    q: &'a QuadBez,
}

impl Apply for MeasureFunctor<'_> {
    fn apply(&self, dydx: &mut Vec<f64>, t: f64, y: &mut Vec<f64>) {
        let dxy = self.af.deriv(t);
        dydx[0] = dxy.distance(Point::ZERO);
        let curvexy = self.curve.xy(self.s0 + y[0] * self.ss);
        let disterr = if NORM_LEVEL == 1 {
            self.q.point_at_t(t).distance(curvexy)
        } else {
            self.q.point_at_t(t).square_distance(curvexy)
        } * dydx[0];
        let dir = self.curve.dir(self.s0 + y[0] * self.ss);
        let angleerr_orig = dir.x * dxy.y - dir.y * dxy.x;
        let angleerr = if NORM_LEVEL == 1 {
            angleerr_orig.abs()
        } else {
            (angleerr_orig * angleerr_orig) / dydx[0]
        };
        dydx[1] = DIST_FACTOR * disterr + ANGLE_FACTOR * angleerr;
    }
}

#[derive(Debug)]
struct Thetas {
    xys: Vec<Point>,
    dirs: Vec<Point>,
    arclen: f64,
}
impl Thetas {
    fn new(qs: &[QuadBez]) -> Self {
        let mut xys = vec![];
        let mut dirs = vec![];
        let mut arclen = 0_f64;
        let mut ix = 0_f64;
        let mut lastxy = Point::new(0.0, 0.0);
        let mut lastd = Point::new(0.0, 0.0);
        let mut lasts = -1_f64;
        for q in qs {
            let derivs = ArclenFunctor::new(q);
            let n = 100;
            let dt = 1.0 / 100.0;
            let mut y = vec![arclen];
            let mut t = 0.0;
            for _ in 0..n {
                let thisxy = q.point_at_t(t);
                let thisd = derivs.deriv(t);
                while ix <= y[0] {
                    let u = (ix as f64 - lasts) / (y[0] - lasts);
                    xys.push(lastxy.lerp(thisxy, u));
                    dirs.push(lastd.lerp(thisd, u).unitize());
                    ix += 1.0;
                }
                lasts = y[0];
                rk4(&mut y, t, dt, &derivs);
                t += dt;
                lastxy = thisxy;
                lastd = thisd;
            }
            arclen = y[0]
        }
        let q = qs.last().unwrap();
        let thisxy = q.p2;
        let thisd = ArclenFunctor::new(q).deriv(1.0);
        while ix <= arclen + 1.0 {
            let u = (ix as f64 - lasts) / (arclen - lasts);
            xys.push(lastxy.lerp(thisxy, u));
            dirs.push(lastd.lerp(thisd, u).unitize());
            ix += 1.0;
        }
        Thetas { xys, dirs, arclen }
    }
    fn xy(&self, s: f64) -> Point {
        let bucket: usize = s as usize;
        let frac = s - s.floor();
        self.xys[bucket].lerp(self.xys[bucket + 1], frac)
    }
    fn dir(&self, s: f64) -> Point {
        let bucket: usize = s as usize;
        let frac = s - s.floor();
        self.dirs[bucket].lerp(self.dirs[bucket + 1], frac)
    }

    fn measure_quad(&self, s0: f64, s1: f64, q: &QuadBez) -> f64 {
        // println!("Q is {:?}", q);
        // println!("s0 is {:?}", s0);
        // println!("s1 is {:?}", s1);
        let derivs = ArclenFunctor::new(q);
        let ss = if f64_is_close(q.arclen(), 0.0) {
            0.0
        } else {
            (s1 - s0) / q.arclen()
        };
        let err = MeasureFunctor {
            curve: self,
            s0,
            ss,
            af: derivs,
            q,
        };
        let dt = 1.0 / 10.0;
        let mut t = 0.0;
        let mut y = vec![0.0, 0.0];
        for _ in 0..10 {
            rk4(&mut y, t, dt, &err);
            // println!(" rk round t={:?}, y={:?}", t, y);
            t += dt;
        }
        y[1]
    }

    fn find_breaks(&self) -> Option<Vec<Break>> {
        let mut breaks: Vec<Break> = vec![];
        let mut lastd = 0.0;
        let n = (10.0 * self.arclen).round() as i32;
        for i in 0..(n + 1) {
            let s = self.arclen * i as f64 / n as f64;
            if (s as usize) + 1 > self.xys.len() {
                return None;
            }
            let orig_p = self.xy(s);
            let p = if HALF_STEP {
                Point::new(
                    0.5 * (2.0 * orig_p.x).round(),
                    0.5 * (2.0 * orig_p.y).round(),
                )
            } else {
                orig_p.round()
            };
            let dist = p.distance(orig_p);
            if (i == 0) || !(p.is_close(breaks.last().unwrap().xy)) {
                // println!("Adding break at {:?}", p);
                let bk = Break {
                    s,
                    xy: p,
                    dir: self.dir(s),
                };
                breaks.push(bk);
                lastd = dist;
            } else if dist < lastd {
                breaks.pop();
                // println!("Removing break, adding one at {:?}", p);
                breaks.push(Break {
                    s,
                    xy: p,
                    dir: self.dir(s),
                })
            }
        }
        Some(breaks)
    }

    fn optimize(&self, penalty: f64) -> Option<Vec<QuadBez>> {
        let breaks = self.find_breaks();
        breaks.as_ref()?;
        let breaks = breaks.unwrap();
        let n = breaks.len() - 1;
        let mut states: Vec<State> = breaks.iter().map(|_| State::new()).collect();
        states[0].init = true;
        // println!("Try line quad {:?} -- {:?}", breaks[0], breaks[n]);
        try_line_quad(&mut states, 0, n, self, &breaks[0], &breaks[n], penalty);
        if states[n].sts[0].score > 3.0 * penalty {
            for i in 1..n {
                // println!("Trying a split {:}", i);
                try_line_quad(&mut states, 0, i, self, &breaks[0], &breaks[i], penalty);
                try_line_quad(&mut states, i, n, self, &breaks[i], &breaks[n], penalty);
                // println!("States[n] = {:?}", states[n]);
            }
            if states[n].sts[0].score > 4.0 * penalty {
                for i in 1..n + 1 {
                    let mut j = i - 1;
                    loop {
                        println!("{:?}, {:?}", i, j);
                        try_line_quad(&mut states, j, i, self, &breaks[j], &breaks[i], penalty);
                        if j == 0 {
                            break;
                        }
                        j -= 1;
                    }
                }
            }
        }
        let mut result: Vec<QuadBez> = vec![];
        let mut sl: &Statelet = &states[n].sts[0];
        // println!("All done, last state is {:?}", sl);
        loop {
            result.push(sl.quad);
            if sl.prev.is_none() {
                break;
            }
            sl = &sl.prev.as_ref().as_ref().unwrap();
        }
        result.reverse();
        Some(result)
    }
}

#[derive(Debug, Copy, Clone)]
struct Break {
    s: f64,
    xy: Point,
    dir: Point,
}

#[derive(Debug, Clone)]
struct Statelet {
    prev: Box<Option<Statelet>>,
    score: f64,
    quad: QuadBez,
}

impl Statelet {
    fn combine(&mut self, newprev: Option<&Statelet>, newscore: f64, newq: QuadBez, penalty: f64) {
        self.prev = Box::new(newprev.cloned());
        let pmul = if (newq.is_line())
            || (newprev.is_some()
                && !newprev.as_ref().unwrap().quad.is_line()
                && newprev
                    .as_ref()
                    .unwrap()
                    .quad
                    .p1
                    .lerp(newq.p1, 0.5)
                    .is_close(newq.p0))
        {
            1.0
        } else {
            2.0
        };

        self.score = newprev.map_or(0.0, |p| p.score) + penalty * pmul + newscore;
        self.quad = newq
    }
}

#[derive(Debug, Clone)]
struct State {
    sts: Vec<Statelet>,
    init: bool,
}

fn is_int(f: f64) -> bool {
    f64_is_close(f - f.floor(), 0.0)
}

impl State {
    fn new() -> Self {
        State {
            sts: vec![],
            init: false,
        }
    }
    fn combine(&mut self, prev: &State, score: f64, q: &QuadBez, penalty: f64) {
        let prev_sl = prev.sts.first();
        if !prev.init && prev_sl.is_none() {
            return;
        }
        let mut sl = Statelet {
            prev: Box::new(None),
            score: 0.0,
            quad: *q,
        };
        sl.combine(prev_sl, score, *q, penalty);
        if self.sts.is_empty() {
            self.sts.push(sl);
        } else if sl.score < self.sts[0].score {
            self.sts[0] = sl
        }
    }

    fn ok_for_half(&self, q: QuadBez) -> bool {
        if is_int(q.p0.x) && is_int(q.p0.y) {
            return true;
        }
        if q.is_line() {
            return false;
        }
        if let Some(prevsl) = self.sts.first() {
            if prevsl.quad.is_line() {
                return false;
            }
            prevsl.quad.p1.lerp(q.p1, 0.5).is_close(q.p0)
        } else {
            false
        }
    }
}

fn try_quad(
    states: &mut Vec<State>,
    prev: usize,
    this: usize,
    curve: &Thetas,
    bk0: &Break,
    bk1: &Break,
    q: &QuadBez,
    penalty: f64,
) {
    let score = curve.measure_quad(bk0.s, bk1.s, q);
    let prev = states[prev].clone();

    // println!("Pre combine prev {:?}", prev);
    // println!("Pre combine this {:?}", states[this]);
    states[this].combine(&prev, score, q, penalty);
    // println!("Post combine prev {:?}", prev);
    // println!("Post combine this {:?}", states[this]);
}

fn try_line_quad(
    states: &mut Vec<State>,
    prev: usize,
    this: usize,
    curve: &Thetas,
    bk0: &Break,
    bk1: &Break,
    penalty: f64,
) {
    if is_int(bk0.xy.x) && is_int(bk0.xy.y) {
        // println!("Can this be a line? bk0={:?}, bk1={:?}", bk0, bk1);
        let line = QuadBez::new(bk0.xy, bk0.xy.lerp(bk1.xy, 0.5), bk1.xy);
        try_quad(states, prev, this, curve, bk0, bk1, &line, penalty)
    }
    if let Some(pmid) = intersect(bk0.xy, bk0.dir, bk1.xy, bk1.dir) {
        let q = QuadBez::new(bk0.xy, pmid.round(), bk1.xy);
        if states[prev].ok_for_half(q) {
            try_quad(states, prev, this, curve, bk0, bk1, &q, penalty);
        }
    }
}

fn segment_sp(segs: &[PathSeg]) -> Vec<usize> {
    let mut res = BTreeSet::<usize>::new();
    let mut xsg = 0.0;
    let mut ysg = 0.0;
    for i in 0..2 * segs.len() {
        let imod = i % segs.len();
        let xsg1;
        let ysg1;
        match segs[imod] {
            PathSeg::Line(l) => {
                xsg1 = l.p1.x - l.p0.x;
                ysg1 = l.p1.y - l.p0.y;
            }
            PathSeg::Quad(q) => {
                xsg1 = q.p2.x - q.p0.x;
                ysg1 = q.p2.y - q.p0.y;
            }
            _ => panic!("That's not very TrueType"),
        }
        if xsg * xsg1 < 0.0 || ysg * ysg1 < 0.0 {
            res.insert(imod);
            xsg = xsg1;
            ysg = ysg1;
        } else {
            if f64_is_close(xsg, 0.0) {
                xsg = xsg1
            }
            if f64_is_close(ysg, 0.0) {
                ysg = ysg1
            }
        }
    }

    // Angle breaks
    for i in 0..segs.len() {
        let prev_ix = if i == 0 { segs.len() - 1 } else { i - 1 };
        let dx0;
        let dy0;
        let dx1;
        let dy1;
        match segs[prev_ix] {
            PathSeg::Line(l) => {
                dx0 = l.p1.x - l.p0.x;
                dy0 = l.p1.y - l.p0.y;
            }
            PathSeg::Quad(q) => {
                dx0 = q.p2.x - q.p1.x;
                dy0 = q.p2.y - q.p1.y;
            }
            _ => panic!("That's not very TrueType"),
        }
        match segs[i] {
            PathSeg::Line(l) => {
                dx1 = l.p1.x - l.p0.x;
                dy1 = l.p1.y - l.p0.y;
            }
            PathSeg::Quad(q) => {
                dx1 = q.p1.x - q.p0.x;
                dy1 = q.p1.y - q.p0.y;
            }
            _ => panic!("That's not very TrueType"),
        }
        let bend = dx1 * dy0 - dx0 * dy1;
        if (f64_is_close(dx0, 0.0) && f64_is_close(dy0, 0.0))
            || (f64_is_close(dx1, 0.0) && f64_is_close(dy1, 0.0))
        {
            res.insert(i);
        } else {
            let bend = bend / (dx0.hypot(dy0) * dx1.hypot(dy1));
            if bend.abs() > 0.02 {
                res.insert(i);
            }
        }
    }
    // println!("Breaks: {:?}", res);
    res.iter().cloned().collect()
}
fn crunch_contour(kurbo: BezPath) -> BezPath {
    let segs: Vec<PathSeg> = kurbo.segments().collect();
    let mut new_kurbo = BezPath::new();
    let breaks: Vec<usize> = segment_sp(&segs);
    let indices: Vec<usize> = (0..breaks.len()).collect();
    new_kurbo.push(kurbo.elements()[0]);
    // indices.push(0);

    for ixes in indices.windows(2) {
        if let [ix1, ix2] = *ixes {
            let bk0 = breaks[ix1];
            let bk1 = breaks[ix2];
            if bk1 != (bk0 + 1) % segs.len() || matches!(segs[bk0], PathSeg::Quad(_)) {
                let quadbezes: Vec<QuadBez> = segs[bk0..bk1]
                    .iter()
                    .map(|l| match l {
                        PathSeg::Line(l) => QuadBez::new(l.p0, l.p0.lerp(l.p1, 0.5), l.p1),
                        PathSeg::Quad(q) => *q,
                        _ => panic!("That's not very TrueType"),
                    })
                    .collect();
                let thetas = Thetas::new(&quadbezes);
                let new_quads = thetas.optimize(1.0);
                if new_quads.is_none() {
                    return kurbo;
                }
                for q in new_quads.unwrap() {
                    if q.is_line() {
                        new_kurbo.line_to(q.p2);
                    } else {
                        new_kurbo.quad_to(q.p1, q.p2);
                    }
                }
            } else {
                match segs[bk0] {
                    PathSeg::Quad(q) => new_kurbo.quad_to(q.p1, q.p2),
                    PathSeg::Line(l) => new_kurbo.line_to(l.p1),
                    _ => {}
                }
            }
        }
    }
    // println!("Crunched path: {:?}", new_kurbo);
    new_kurbo
}

fn crunch_glyph(glyph: &Glyph) -> Glyph {
    let mut new_glyph = glyph.clone();
    // println!("Crunching {:?}", new_glyph);
    new_glyph.contours = new_glyph
        .contours
        .iter()
        .map(|c| {
            let kurbo = crunch_contour(glyf_contour_to_kurbo_contour(c));
            // remove_explicit_oncurves(
            kurbo_contour_to_glyf_contour(&kurbo, 0.5)
            // )
        })
        .collect();
    // println!("New glyph {:?}", new_glyph);
    new_glyph
}

fn main() {
    let matches = read_args("fontcrunch", "Optimizes quadratic beziers in a font");
    let mut infont = open_font(&matches);
    if infont.tables.contains_key(b"gvar") {
        log::error!("fontcrunch may not be used on variable fonts (yet)");
        return;
    }
    let glyphnames = if let Table::Post(post) = infont
        .get_table(b"post")
        .expect("Error reading post table")
        .expect("No post table found")
    {
        &post.glyphnames
    } else {
        &None
    }
    .clone();

    if let Table::Glyf(glyf) = infont
        .get_table(b"glyf")
        .expect("Error reading glyf table")
        .expect("No glyf table found")
    {
        glyf.glyphs = glyf
            .glyphs
            .iter()
            .enumerate()
            .map(|(ix, g)| {
                let name = glyphnames.as_ref().map_or("", |gn| &gn[ix]);
                println!("Crunching {:}", name);
                // if name == "Aogonek" {
                crunch_glyph(g)
                // } else {
                // g.clone()
                // }
            })
            .collect();
    }
    save_font(infont, &matches);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arc_len() {
        let q = QuadBez::new(Point::new(100.0, 0.0), Point::ZERO, Point::new(0.0, 100.0));
        assert!(f64_is_close(q.arclen(), 162.32248241172945));
    }

    #[test]
    fn test_thetas_dir() {
        let qs = vec![
            QuadBez::new(
                Point::new(324.0, 714.0),
                Point::new(497.0, 713.0),
                Point::new(589.0, 625.0),
            ),
            QuadBez::new(
                Point::new(589.0, 625.0),
                Point::new(682.0, 538.0),
                Point::new(683.0, 372.0),
            ),
        ];
        let thetas = Thetas::new(&qs);
        let mut i: f64 = 0.0;
        println!("Arclen: {:?}", thetas.arclen);
        assert!((thetas.arclen - 564.284).abs() < 0.001);
        let mut dirs = vec![];
        while i < thetas.arclen {
            dirs.push(thetas.dir(i));
            i += 1.0;
        }
        assert!(dirs[0].is_close(Point::new(0.99998329, -0.0057802503)));
        assert!(dirs[100].is_close(Point::new(0.98258825, -0.18579647)));
        assert!(dirs[100].is_close(Point::new(0.98258825, -0.18579647)));
    }

    #[test]
    fn test_optimize() {
        let qs = vec![
            QuadBez::new(
                Point::new(324.0, 714.0),
                Point::new(497.0, 713.0),
                Point::new(589.0, 625.0),
            ),
            QuadBez::new(
                Point::new(589.0, 625.0),
                Point::new(682.0, 538.0),
                Point::new(683.0, 372.0),
            ),
        ];
        let thetas = Thetas::new(&qs);
        let out = thetas.optimize(1.0);
        let expected = [
            QuadBez {
                p0: Point::new(324.0, 714.0),
                p1: Point::new(495.0, 713.0),
                p2: Point::new(588.5, 625.5),
            },
            QuadBez {
                p0: Point::new(588.5, 625.5),
                p1: Point::new(682.0, 538.0),
                p2: Point::new(683.0, 372.0),
            },
        ];
        println!("{:?}", out);
        assert_eq!(out.unwrap(), expected);
    }

    #[test]
    fn test_segment_sp() {
        let path: Vec<PathSeg> = vec![
            PathSeg::Line(kurbo::Line::new((308.0, 0.0), (77.0, 0.0))),
            PathSeg::Line(kurbo::Line::new((77.0, 0.0), (77.0, 714.0))),
            PathSeg::Line(kurbo::Line::new((77.0, 714.0), (324.0, 714.0))),
            PathSeg::Quad(kurbo::QuadBez::new(
                (324.0, 714.0),
                (497.0, 713.0),
                (589.0, 625.0),
            )),
            PathSeg::Quad(kurbo::QuadBez::new(
                (589.0, 625.0),
                (682.0, 538.0),
                (683.0, 372.0),
            )),
            PathSeg::Quad(kurbo::QuadBez::new(
                (683.0, 372.0),
                (680.0, 185.0),
                (580.0, 92.0),
            )),
            PathSeg::Quad(kurbo::QuadBez::new(
                (580.0, 92.0),
                (480.0, 0.0),
                (308.0, 0.0),
            )),
        ];
        let res = segment_sp(&path);
        assert_eq!(res, vec![0, 1, 2, 3, 5, 7]);
    }
}
