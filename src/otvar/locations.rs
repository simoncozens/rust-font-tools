use core::ops::{Mul, Sub};
use otspec::types::{Tag, Tuple, F2DOT14};
use permutation::Permutation;
use std::array::IntoIter;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// Structs to store locations (user and normalized)

/// A location in the user's coordinate space (e.g. wdth=200,wght=15)
pub struct UserLocation(pub Tuple);

/// A location in the internal -1 <= 0 => 1 representation
pub struct NormalizedLocation(pub Tuple);

type Support = HashMap<Tag, (f32, f32, f32)>;
pub type Location = HashMap<Tag, f32>;
type AxisPoints = HashMap<Tag, HashSet<i16>>;

#[derive(Debug)]
pub struct VariationModel {
    pub locations: Vec<Location>,
    sort_order: Permutation,
    pub supports: Vec<Support>,
    pub axis_order: Vec<Tag>,
    // submodels: HashMap<[usize],
    delta_weights: Vec<HashMap<usize, f32>>,
}

fn support_scalar(loc: &Location, support: &Support) -> f32 {
    let mut scalar = 1.0;
    for (&axis, &(lower, peak, upper)) in support.iter() {
        if peak == 0.0 {
            continue;
        }
        if lower > peak || peak > upper {
            continue;
        }
        if lower < 0.0 && upper > 0.0 {
            continue;
        }
        let v: f32 = *loc.get(&axis).unwrap_or(&0.0);
        if (v - peak).abs() < f32::EPSILON {
            continue;
        }
        if v <= lower || upper <= v {
            scalar = 0.0;
            break;
        }
        if v < peak {
            scalar *= (v - lower) / (peak - lower)
        } else {
            scalar *= (v - upper) / (peak - upper)
        }
    }
    scalar
}

fn locations_to_regions(locations: &[Location]) -> Vec<Support> {
    let mut axis_minimum: HashMap<Tag, f32> = HashMap::new();
    let mut axis_maximum: HashMap<Tag, f32> = HashMap::new();
    for (tag, value) in locations.iter().flatten() {
        axis_maximum
            .entry(*tag)
            .and_modify(|v| *v = v.max(*value))
            .or_insert(*value);
        axis_minimum
            .entry(*tag)
            .and_modify(|v| *v = v.min(*value))
            .or_insert(*value);
    }
    locations
        .iter()
        .map(|loc| {
            loc.iter()
                .map(|(axis, locV)| {
                    (
                        *axis,
                        if *locV > 0.0 {
                            (0.0, *locV, *axis_maximum.get(axis).unwrap())
                        } else {
                            (*axis_minimum.get(axis).unwrap(), *locV, 0.0)
                        },
                    )
                })
                .collect()
        })
        .collect()
}

impl VariationModel {
    pub fn new(locations: Vec<Location>, axis_order: Vec<Tag>) -> Self {
        let locations: Vec<Location> = locations
            .iter()
            .map(|l| {
                let mut l2 = l.clone();
                l2.retain(|_, v| *v != 0.0);
                l2
            })
            .collect();
        let indices: Vec<usize> = (0..locations.len()).collect();
        let mut axis_points = AxisPoints::new();
        for loc in locations.iter().filter(|l| l.len() == 1) {
            if let Some((axis, value)) = loc.iter().next() {
                let entry = axis_points
                    .entry(*axis)
                    .or_insert_with(|| IntoIter::new([F2DOT14::pack(0.0)]).collect());
                entry.insert(F2DOT14::pack(*value));
            }
        }
        let on_point_count = |loc: &Location| {
            loc.iter()
                .filter(|(&axis, &value)| {
                    axis_points.contains_key(&axis)
                        && axis_points
                            .get(&axis)
                            .unwrap()
                            .contains(&F2DOT14::pack(value))
                })
                .count()
        };
        let sort_order = permutation::sort_by(&indices[..], |a_ix, b_ix| {
            let a = &locations[*a_ix];
            let b = &locations[*b_ix];
            if a.keys().len() != b.keys().len() {
                return a.keys().len().cmp(&b.keys().len());
            }

            let a_on_point = on_point_count(a);
            let b_on_point = on_point_count(b);
            if a_on_point != b_on_point {
                return b_on_point.cmp(&a_on_point);
            }

            let mut a_ordered_axes: Vec<Tag> = a.keys().copied().collect();
            let mut b_ordered_axes: Vec<Tag> = b.keys().copied().collect();
            a_ordered_axes.sort_by(|ka, kb| {
                if axis_order.contains(ka) && !axis_order.contains(kb) {
                    return Ordering::Less;
                }
                if axis_order.contains(kb) && !axis_order.contains(ka) {
                    return Ordering::Greater;
                }
                ka.cmp(kb)
            });
            b_ordered_axes.sort_by(|ka, kb| {
                if axis_order.contains(ka) && !axis_order.contains(kb) {
                    return Ordering::Less;
                }
                if axis_order.contains(kb) && !axis_order.contains(ka) {
                    return Ordering::Greater;
                }
                ka.cmp(kb)
            });
            for (left, right) in a_ordered_axes.iter().zip(b_ordered_axes.iter()) {
                let l_index = axis_order.iter().position(|ax| ax == left);
                let r_index = axis_order.iter().position(|ax| ax == right);

                if l_index.is_some() && r_index.is_none() {
                    return Ordering::Less;
                }
                if r_index.is_some() && l_index.is_none() {
                    return Ordering::Greater;
                }
                if l_index != r_index {
                    return l_index.cmp(&r_index);
                }
            }

            if let Some(axes_order) = a_ordered_axes.iter().partial_cmp(b_ordered_axes.iter()) {
                if axes_order != Ordering::Equal {
                    return axes_order;
                }
            }

            for (left, _) in a_ordered_axes.iter().zip(b_ordered_axes.iter()) {
                let a_sign = a.get(left).unwrap().signum();
                let b_sign = b.get(left).unwrap().signum();
                if (a_sign - b_sign).abs() > f32::EPSILON {
                    return a_sign.partial_cmp(&b_sign).unwrap();
                }
            }
            for (left, _) in a_ordered_axes.iter().zip(b_ordered_axes.iter()) {
                let a_abs = a.get(left).unwrap().abs();
                let b_abs = b.get(left).unwrap().abs();
                if (a_abs - b_abs).abs() > f32::EPSILON {
                    return a_abs.partial_cmp(&b_abs).unwrap();
                }
            }
            Ordering::Equal
        });

        let mut vm = VariationModel {
            locations: sort_order.apply_slice(&locations[..]),
            sort_order,
            axis_order,
            supports: vec![],
            delta_weights: vec![],
        };
        vm._compute_master_supports();
        vm._compute_delta_weights();
        vm
    }

    fn _compute_master_supports(&mut self) {
        let regions = locations_to_regions(&self.locations);
        self.supports.clear();
        for (i, region) in regions.iter().enumerate() {
            let loc_axes: HashSet<Tag> = region.keys().copied().collect();
            let mut region_copy = region.clone();
            for prev_region in &regions[..i] {
                let prev_loc_axes: HashSet<Tag> = prev_region.keys().copied().collect();
                if !prev_loc_axes.is_subset(&loc_axes) {
                    continue;
                }
                let mut relevant = true;
                for (axis, &(lower, peak, upper)) in region.iter() {
                    if !prev_region.contains_key(axis) {
                        relevant = false;
                        break;
                    }
                    let prev_peak = prev_region.get(axis).unwrap().1;
                    if !((prev_peak - peak).abs() < f32::EPSILON
                        || (lower < prev_peak && prev_peak < upper))
                    {
                        relevant = false;
                        break;
                    }
                }
                if !relevant {
                    continue;
                }
                let mut best_axes: Support = Support::new();
                let mut best_ratio = -1_f32;
                for (&axis, &(_, val, _)) in prev_region.iter() {
                    let &(lower, loc_v, upper) = region.get(&axis).unwrap();
                    let mut new_lower = lower;
                    let mut new_upper = upper;
                    let ratio: f32;
                    if val < loc_v {
                        new_lower = val;
                        ratio = (val - loc_v) / (lower - loc_v);
                    } else if loc_v < val {
                        new_upper = val;
                        ratio = (val - loc_v) / (upper - loc_v);
                    } else {
                        continue;
                    }
                    if ratio > best_ratio {
                        best_ratio = ratio;
                        best_axes.clear();
                    }
                    if (ratio - best_ratio).abs() < f32::EPSILON {
                        best_axes.insert(axis, (new_lower, loc_v, new_upper));
                    }
                }
                for (axis, triple) in best_axes.iter() {
                    region_copy.insert(*axis, *triple);
                }
            }
            self.supports.push(region_copy);
        }
    }

    fn _compute_delta_weights(&mut self) {
        self.delta_weights.clear();
        for (i, loc) in self.locations.iter().enumerate() {
            let mut delta_weight: HashMap<usize, f32> = HashMap::new();
            for (j, support) in self.supports[..i].iter().enumerate() {
                let scalar = support_scalar(loc, support);
                if scalar != 0.0 {
                    delta_weight.insert(j, scalar);
                }
            }
            self.delta_weights.push(delta_weight);
        }
    }

    pub fn get_deltas<T>(&self, master_values: &[T]) -> Vec<T>
    where
        T: Sub<Output = T> + Mul<f32, Output = T> + Clone,
    {
        assert_eq!(master_values.len(), self.delta_weights.len());
        let mut out: Vec<T> = vec![];
        for (ix, weights) in self.delta_weights.iter().enumerate() {
            let mut delta = master_values[self.sort_order.apply_inv_idx(ix)].clone();
            for (&j, &weight) in weights.iter() {
                delta = delta - out[j].clone() * weight;
            }
            out.push(delta);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_approx_eq::assert_approx_eq;
    use std::iter::FromIterator;

    macro_rules! hashmap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::HashMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }
    #[test]
    fn test_support_scalar() {
        assert_approx_eq!(support_scalar(&Location::new(), &Support::new()), 1.0);
        assert_approx_eq!(
            support_scalar(&hashmap!( *b"wght" => 0.2), &Support::new()),
            1.0
        );
        assert_approx_eq!(
            support_scalar(
                &hashmap!( *b"wght" => 0.2),
                &hashmap!( *b"wght" => (0_f32,2_f32,3_f32))
            ),
            0.1
        );
        assert_approx_eq!(
            support_scalar(
                &hashmap!( *b"wght" => 2.5),
                &hashmap!( *b"wght" => (0_f32,2_f32,4_f32))
            ),
            0.75
        );
    }

    #[test]
    fn test_variation_model() {
        let locations = vec![
            hashmap!(*b"wght" => 0.55, *b"wdth" => 0.0),
            hashmap!(*b"wght" => -0.55, *b"wdth" => 0.0),
            hashmap!(*b"wght" => -1.0, *b"wdth" => 0.0),
            hashmap!(*b"wght" => 0.0, *b"wdth" => 1.0),
            hashmap!(*b"wght" => 0.66, *b"wdth" => 1.0),
            hashmap!(*b"wght" => 0.66, *b"wdth" => 0.66),
            hashmap!(*b"wght" => 0.0, *b"wdth" => 0.0),
            hashmap!(*b"wght" => 1.0, *b"wdth" => 1.0),
            hashmap!(*b"wght" => 1.0, *b"wdth" => 0.0),
        ];
        let axis_order = vec![*b"wght"];
        let vm = VariationModel::new(locations, axis_order);
        let expected_locations = vec![
            hashmap!(),
            hashmap!(*b"wght" => -0.55),
            hashmap!(*b"wght" => -1.0),
            hashmap!(*b"wght" => 0.55),
            hashmap!(*b"wght" => 1.0),
            hashmap!(*b"wdth" => 1.0),
            hashmap!(*b"wdth" => 1.0, *b"wght" => 1.0),
            hashmap!(*b"wdth" => 1.0, *b"wght" => 0.66),
            hashmap!(*b"wdth" => 0.66, *b"wght" => 0.66),
        ];
        assert_eq!(vm.locations, expected_locations);

        let expected_supports = vec![
            hashmap!(),
            hashmap!(*b"wght" => (-1.0, -0.55, 0.0)),
            hashmap!(*b"wght" => (-1.0, -1.0, -0.55)),
            hashmap!(*b"wght" => (0.0, 0.55, 1.0)),
            hashmap!(*b"wght" => (0.55, 1.0, 1.0)),
            hashmap!(*b"wdth" => (0.0, 1.0, 1.0)),
            hashmap!(*b"wdth" => (0.0, 1.0, 1.0), *b"wght" => (0.0, 1.0, 1.0)),
            hashmap!(*b"wdth" => (0.0, 1.0, 1.0), *b"wght" => (0.0, 0.66, 1.0)),
            hashmap!(*b"wdth" => (0.0, 0.66, 1.0), *b"wght" => (0.0, 0.66, 1.0)),
        ];
        assert_eq!(vm.supports, expected_supports);

        assert_eq!(vm.delta_weights[0], hashmap!());
        assert_eq!(vm.delta_weights[1], hashmap!(0 => 1.0));
        assert_eq!(vm.delta_weights[2], hashmap!(0 => 1.0));
        assert_eq!(vm.delta_weights[3], hashmap!(0 => 1.0));
        assert_eq!(vm.delta_weights[4], hashmap!(0 => 1.0));
        assert_eq!(vm.delta_weights[5], hashmap!(0 => 1.0));
        assert_eq!(vm.delta_weights[6], hashmap!(0 => 1.0, 4 => 1.0, 5 => 1.0));
        assert_approx_eq!(vm.delta_weights[7].get(&3).unwrap(), 0.755_555_57);
        assert_approx_eq!(vm.delta_weights[7].get(&4).unwrap(), 0.244_444_49);
        assert_approx_eq!(vm.delta_weights[7].get(&5).unwrap(), 1.0);
        assert_approx_eq!(vm.delta_weights[7].get(&6).unwrap(), 0.66);
    }
}
