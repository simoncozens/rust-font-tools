use crate::glyf::Glyph;
use crate::otvar::Delta;
use otspec::types::*;
use std::collections::{HashMap, HashSet};

fn iup_segment(
    newdeltas: &mut Vec<(i16, i16)>,
    coords: &[(i16, i16)],
    rc1: (i16, i16),
    rd1: &Option<Delta>,
    rc2: (i16, i16),
    rd2: &Option<Delta>,
) {
    let rd1 = rd1.as_ref().unwrap().get_2d();
    let rd2 = rd2.as_ref().unwrap().get_2d();
    let mut out_arrays: Vec<Vec<i16>> = vec![vec![], vec![]];
    for j in 0..2 {
        let (mut x1, mut x2, mut d1, mut d2) = if j == 0 {
            (rc1.0, rc2.0, rd1.0, rd2.0)
        } else {
            (rc1.1, rc2.1, rd1.1, rd2.1)
        };
        if x1 == x2 {
            let n = coords.len();
            out_arrays[j].extend(std::iter::repeat(if d1 == d2 { d1 } else { 0 }).take(n));
            continue;
        }
        if x1 > x2 {
            std::mem::swap(&mut x2, &mut x1);
            std::mem::swap(&mut d2, &mut d1);
        }

        let scale = (d2 - d1) as f32 / (x2 - x1) as f32;

        for pair in coords {
            let x = if j == 0 { pair.0 } else { pair.1 };
            let d = if x <= x1 {
                d1
            } else if x >= x2 {
                d2
            } else {
                d1 + ((x - x1) as f32 * scale) as i16
            };
            out_arrays[j].push(d);
        }
    }
    newdeltas.extend(
        out_arrays[0]
            .iter()
            .zip(out_arrays[1].iter())
            .map(|(x, y)| (*x, *y)),
    );
}

/// Perform Interpolation of Unreferenced Points on a set of deltas and coordinates
pub fn iup_contour(
    newdeltas: &mut Vec<(i16, i16)>,
    deltas: &[Option<Delta>],
    coords: &[(i16, i16)],
) {
    if deltas.iter().all(|x| x.is_some()) {
        newdeltas.extend::<Vec<(i16, i16)>>(
            deltas
                .iter()
                .map(|x| x.as_ref().unwrap().get_2d())
                .collect(),
        );
        return;
    }
    let n = deltas.len();
    let indices: Vec<usize> = deltas
        .iter()
        .enumerate()
        .filter(|(_, d)| d.is_some())
        .map(|(i, _)| i)
        .collect();
    if indices.is_empty() {
        newdeltas.extend(std::iter::repeat((0, 0)).take(n));
        return;
    }
    let mut start = indices[0];
    let verystart = start;
    if start != 0 {
        let (i1, i2, ri1, ri2) = (0, start, start, *indices.last().unwrap());
        iup_segment(
            newdeltas,
            &coords[i1..i2],
            coords[ri1],
            &deltas[ri1],
            coords[ri2],
            &deltas[ri2],
        );
    }
    newdeltas.push(deltas[start].as_ref().unwrap().get_2d());
    for end in indices.iter().skip(1) {
        if *end - start > 1 {
            let (i1, i2, ri1, ri2) = (start + 1, *end, start, *end);
            iup_segment(
                newdeltas,
                &coords[i1..i2],
                coords[ri1],
                &deltas[ri1],
                coords[ri2],
                &deltas[ri2],
            );
        }
        newdeltas.push(deltas[*end].as_ref().unwrap().get_2d());
        start = *end;
    }
    if start != n - 1 {
        let (i1, i2, ri1, ri2) = (start + 1, n, start, verystart);
        iup_segment(
            newdeltas,
            &coords[i1..i2],
            coords[ri1],
            &deltas[ri1],
            coords[ri2],
            &deltas[ri2],
        );
    }
}

/// Optimize the deltas by removing deltas that be inferred using IUP
pub fn optimize_deltas(deltas: Vec<Option<Delta>>, glyph: &Glyph) -> Vec<Option<Delta>> {
    let (coords, ends): (Vec<(int16, int16)>, Vec<usize>) = glyph.gvar_coords_and_ends();

    let deltas_xy: Vec<(int16, int16)>;
    if !deltas.iter().all(|x| x.is_some()) {
        // Perhaps we're re-optimizing an optimized thing already.
        // Oh well, IUP it all first.
        let mut start = 0;
        let mut newdeltas = vec![];
        for end in &ends {
            let contour_delta = &deltas[start..end + 1];
            let contour_orig = &coords[start..end + 1];
            start = end + 1;
            iup_contour(&mut newdeltas, contour_delta, contour_orig);
        }
        deltas_xy = newdeltas;
    } else {
        deltas_xy = deltas
            .iter()
            .map(|o_d| {
                if let Delta::Delta2D((x, y)) = o_d.as_ref().unwrap() {
                    (*x, *y)
                } else {
                    panic!("Tried to IUP something that wasn't a 2d delta: {:?}", o_d);
                }
            })
            .collect();
    }
    // Again, ends has the phantom points in already
    let mut start = 0;
    let mut newdeltas: Vec<Option<Delta>> = vec![];
    // println!("Coords: {:?}", coords);
    // println!("Ends: {:?}", ends);
    for end in ends {
        // println!("Start={:} End={:}", start, end);
        // println!("Deltas: {:?}", &deltas_xy[start..end + 1]);
        let contour =
            iup_contour_optimize(&deltas_xy[start..end + 1], &coords[start..end + 1], 0.5);
        assert_eq!(contour.len(), end - start + 1);
        newdeltas.extend(contour);
        start = end + 1;
    }
    newdeltas
}

fn iup_contour_optimize(
    deltas_slice: &[(i16, i16)],
    coords_slice: &[(i16, i16)],
    tolerance: f32,
) -> Vec<Option<Delta>> {
    let mut deltas = deltas_slice.to_vec();
    let mut coords = coords_slice.to_vec();
    let n = deltas.len();
    let mut rv = vec![];
    if deltas
        .iter()
        .all(|(x, y)| (x.abs() as f32) <= tolerance && (y.abs() as f32) <= tolerance)
    {
        for _ in 0..n {
            rv.push(None);
        }
        return rv;
    }

    if n == 1 {
        return vec![Some(Delta::Delta2D(deltas[0]))];
    }

    let (first_x, first_y) = deltas[0];
    if deltas.iter().all(|(x, y)| *x == first_x && *y == first_y) {
        rv.push(Some(Delta::Delta2D(deltas[0])));
        for _ in 1..n {
            rv.push(None);
        }
        return rv;
    }

    let mut forced = _iup_contour_bound_forced_set(&deltas, &coords, tolerance);
    let mut output_deltas: Vec<Option<Delta>>;
    if !forced.is_empty() {
        let k: i16 = ((n - 1) - (*forced.iter().max().unwrap() as usize)) as i16;
        assert!(k >= 0);
        deltas = rotate_list(deltas, k);
        coords = rotate_list(coords, k);
        forced = rotate_set(forced, k, n);
        let (chain, _) = _iup_contour_optimize_dp(&deltas, &coords, &forced, tolerance, None);
        let mut solution: HashSet<i16> = HashSet::new();
        let mut i = n as i16 - 1;
        loop {
            solution.insert(i);
            let next = chain.get(&i);
            if let Some(n) = next {
                i = *n;
            } else {
                break;
            }
        }
        output_deltas = (0..n)
            .map(|ix| {
                if solution.contains(&(ix as i16)) {
                    Some(Delta::Delta2D(deltas[ix]))
                } else {
                    None
                }
            })
            .collect();
        output_deltas = rotate_list(output_deltas, -(k as i16));
    } else {
        deltas.extend(deltas.clone());
        coords.extend(coords.clone());
        let (chain, costs) =
            _iup_contour_optimize_dp(&deltas, &coords, &forced, tolerance, Some(n as i16));
        let mut best_cost = n as i16 + 1;
        let mut best_sol: HashSet<usize> = HashSet::new();
        for start in n - 1..2 * n + 1 {
            let mut solution: HashSet<usize> = HashSet::new();
            let mut i = start as i16;
            while i > (start as i16) - (n as i16) {
                solution.insert(i.rem_euclid(n as i16) as usize);
                let next = chain.get(&(i as i16));
                if next.is_some() {
                    i = *next.unwrap()
                } else {
                    break;
                }
            }
            if i == (start as i16) - (n as i16) {
                let cost = *costs.get(&(start as i16)).unwrap_or(&(n as i16 * 4))
                    - *costs
                        .get(&(start as i16 - n as i16))
                        .unwrap_or(&(n as i16 * 3));
                if cost <= best_cost {
                    best_sol = solution.clone();
                    best_cost = cost;
                }
            }
        }
        output_deltas = (0..n)
            .map(|ix| {
                if best_sol.contains(&ix) {
                    Some(Delta::Delta2D(deltas[ix]))
                } else {
                    None
                }
            })
            .collect();
    }
    // println!("Done {:?}", output_deltas);
    output_deltas
}

fn rotate_set(s: HashSet<i16>, mut k: i16, n: usize) -> HashSet<i16> {
    k %= n as i16;
    if k == 0 {
        return s;
    }
    s.iter().map(|v| ((*v + k) % (n as i16))).collect()
}

fn rotate_list<T: Clone + Sized + std::fmt::Debug>(l: Vec<T>, mut k: i16) -> Vec<T> {
    // println!("Rotating list {:?} by {:?}", l, k);
    let n = l.len();
    k = k.rem_euclid(n as i16);
    if k == 0 {
        return l;
    }
    let partition = (n as i16 - k) as usize;
    // println!("Partition at {:?}", partition);
    let mut first = l[partition..].to_vec();
    let second = &l[0..partition];
    first.extend(second.to_vec());
    first
}

fn _iup_contour_bound_forced_set(
    deltas: &[(i16, i16)],
    coords: &[(i16, i16)],
    tolerance: f32,
) -> HashSet<i16> {
    assert_eq!(deltas.len(), coords.len());
    let mut forced = HashSet::new();
    let mut nd = deltas[0];
    let mut nc = coords[0];
    let mut i = (deltas.len() - 1) as i16;
    let mut ld = deltas[i as usize];
    let mut lc = coords[i as usize];
    while i > -1 {
        let d = ld;
        let c = lc;
        // Use Euclidean remainders here to get i=0 case
        ld = deltas[((i - 1).rem_euclid(deltas.len() as i16)) as usize];
        lc = coords[((i - 1).rem_euclid(coords.len() as i16)) as usize];
        for j in 0..2 {
            let cj = if j == 0 { c.0 } else { c.1 } as f32;
            let dj = if j == 0 { d.0 } else { d.1 } as f32;
            let lcj = if j == 0 { lc.0 } else { lc.1 } as f32;
            let ldj = if j == 0 { ld.0 } else { ld.1 } as f32;
            let ncj = if j == 0 { nc.0 } else { nc.1 } as f32;
            let ndj = if j == 0 { nd.0 } else { nd.1 } as f32;
            let (c1, c2, d1, d2);

            if lcj <= ncj {
                c1 = lcj;
                c2 = ncj;
                d1 = ldj;
                d2 = ndj;
            } else {
                c1 = ncj;
                c2 = lcj;
                d1 = ndj;
                d2 = ldj;
            }
            let mut force = false;
            if c1 <= cj && cj <= c2 {
                if !(d1.min(d2) - tolerance <= dj && dj <= d1.max(d2) + tolerance) {
                    force = true;
                }
            } else {
                if (c1 - c2).abs() < f32::EPSILON {
                    if (d1 - d2).abs() < f32::EPSILON {
                        if (dj - d1).abs() > tolerance {
                            force = true;
                        }
                    } else {
                        if dj.abs() > tolerance {
                            // Not forced, surprisingly.
                        }
                    }
                } else if (d1 - d2).abs() > f32::EPSILON {
                    if cj < c1 {
                        if dj != d1 && ((dj - tolerance < d1) != (d1 < d2)) {
                            force = true;
                        }
                    } else {
                        if d2 != dj && ((d2 < dj + tolerance) != (d1 < d2)) {
                            force = true;
                        }
                    }
                }
            }
            if force {
                forced.insert(i);
                break;
            }
        }
        nd = d;
        nc = c;
        i -= 1;
    }
    forced
}

fn _iup_contour_optimize_dp(
    deltas: &[(i16, i16)],
    coords: &[(i16, i16)],
    forced: &HashSet<i16>,
    tolerance: f32,
    lookback_o: Option<i16>,
) -> (HashMap<i16, i16>, HashMap<i16, i16>) {
    let n = deltas.len();
    let lookback = lookback_o.unwrap_or(n as i16);
    let mut costs: HashMap<i16, i16> = HashMap::new();
    let mut chain: HashMap<i16, i16> = HashMap::new();
    // println!("Doing DP. Forced={:?}", forced);
    costs.insert(-1, 0);
    for i in 0..n {
        // println!(" i={:?}", i);
        let i_i16 = i as i16;
        let mut best_cost = costs.get(&(i_i16 - 1)).unwrap() + 1;
        costs.insert(i_i16, best_cost);
        chain.insert(i_i16, i_i16 - 1);
        // println!(" best_cost={:?}", best_cost);
        if forced.contains(&(i_i16 - 1)) {
            // println!(" Prev was forced");
            continue;
        }
        let mut j: i16 = i_i16 - 2;
        // println!(" j={:?}", j);
        while j > (i_i16 - lookback).max(-2) {
            let cost = costs.get(&j).unwrap() + 1;
            if cost < best_cost && can_iup_between(deltas, coords, j, i_i16, tolerance) {
                best_cost = cost;
                costs.insert(i_i16, best_cost);
                chain.insert(i_i16, j as i16);
            }
            if forced.contains(&j) {
                break;
            }
            j -= 1;
        }
    }
    // println!("Done, {:?}, {:?}", chain, costs);
    (chain, costs)
}

fn can_iup_between(
    deltas: &[(i16, i16)],
    coords: &[(i16, i16)],
    i_i16: i16,
    j_i16: i16,
    tolerance: f32,
) -> bool {
    assert!(j_i16 - i_i16 >= 2);
    let i = i_i16.rem_euclid((deltas.len()) as i16) as usize;
    let j = j_i16.rem_euclid((deltas.len()) as i16) as usize;
    let mut coord_portion: Vec<(i16, i16)>;
    let mut delta_portion: Vec<(i16, i16)>;
    if i + 1 > j {
        coord_portion = coords[i + 1..].to_vec();
        coord_portion.extend(coords[0..j].to_vec());
        delta_portion = deltas[i + 1..].to_vec();
        delta_portion.extend(deltas[0..j].to_vec());
    } else {
        coord_portion = coords[i + 1..j].to_vec();
        delta_portion = deltas[i + 1..j].to_vec();
    };
    // assert!(j - i >= 2);
    let mut interp = vec![];
    iup_segment(
        &mut interp,
        &coord_portion,
        coords[i],
        &Some(Delta::Delta2D(deltas[i])),
        coords[j],
        &Some(Delta::Delta2D(deltas[j])),
    );
    assert_eq!(interp.len(), delta_portion.len());
    let can_iup = delta_portion
        .iter()
        .zip(interp.iter())
        .all(|((x, y), (p, q))| {
            ((x - p) as f32 * (x - p) as f32 + (y - q) as f32 * (y - q) as f32) <= tolerance
        });
    can_iup
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_can_iup() {
        let coords = vec![(261, 611), (261, 113), (108, 113), (108, 611)];
        let deltas = vec![
            (38, 125),   // IUP
            (38, -125),  // given
            (-38, -125), // IUP
            (-38, 125),  // given
        ];
        assert!(can_iup_between(&deltas, &coords, 1, 3, 0.5));
        assert!(can_iup_between(&deltas, &coords, -1, 1, 0.5));
    }

    #[test]
    fn test_do_iup1_optimize() {
        let optimized = vec![
            Some(Delta::Delta2D((155, 0))),
            Some(Delta::Delta2D((123, 0))),
            Some(Delta::Delta2D((32, 0))),
            Some(Delta::Delta2D((64, 0))),
            None,
        ];
        let coords = vec![(751, 0), (433, 700), (323, 700), (641, 0), (751, 0)];
        let unoptimized = vec![(155, 0), (123, 0), (32, 0), (64, 0), (155, 0)];
        let mut newdeltas = vec![];
        iup_contour(&mut newdeltas, &optimized, &coords);
        assert_eq!(newdeltas, unoptimized);

        let check_optimized = iup_contour_optimize(&unoptimized, &coords, 0.5);
        assert_eq!(check_optimized, optimized);
    }

    #[test]
    fn test_do_iup2_optimize() {
        let optimized = vec![
            Some(Delta::Delta2D((38, 27))),
            None,
            Some(Delta::Delta2D((73, -13))),
            None,
            None,
        ];
        let coords = vec![(152, 284), (152, 204), (567, 204), (567, 284), (152, 284)];
        let unoptimized = vec![(38, 27), (38, -13), (73, -13), (73, 27), (38, 27)];
        let mut newdeltas = vec![];
        iup_contour(&mut newdeltas, &optimized, &coords);
        assert_eq!(newdeltas, unoptimized);

        let check_optimized = iup_contour_optimize(&unoptimized, &coords, 0.5);
        assert_eq!(check_optimized, optimized);
    }
}
