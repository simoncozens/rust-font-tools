use core::ops::{Mul, Sub};
use otspec::types::{Tag, Tuple, F2DOT14};
use permutation::Permutation;
use std::array::IntoIter;
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};

/// Structs to store locations (user and normalized)

/// A location in the internal -1 <= 0 => 1 representation
#[derive(Debug)]
pub struct NormalizedLocation(pub Tuple);

/// A region of the designspace, consisting of a set of per-axis triangular tents
pub type Support = BTreeMap<Tag, (f32, f32, f32)>;
/// A location as a mapping of tags to user-space values
pub type Location = BTreeMap<Tag, f32>;
type AxisPoints = BTreeMap<Tag, HashSet<F2DOT14>>;

/// An OpenType variation model helps to determine and interpolate the correct
/// supports and deltasets when there are intermediate masters.
#[derive(Debug)]
pub struct VariationModel {
    /// The rearranged list of master locations
    pub locations: Vec<Location>,
    sort_order: Permutation,
    /// The supports computed for each master
    pub supports: Vec<Support>,
    /// The axis order provided by the user
    pub axis_order: Vec<Tag>,
    /// The original, unordered list of locations
    pub original_locations: Vec<Location>,
    delta_weights: Vec<BTreeMap<usize, f32>>,
}

/// Returns the contribution value of a region at a given location
pub fn support_scalar(loc: &Location, support: &Support) -> f32 {
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
    let mut axis_minimum: BTreeMap<Tag, f32> = BTreeMap::new();
    let mut axis_maximum: BTreeMap<Tag, f32> = BTreeMap::new();
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
                .map(|(axis, loc_v)| {
                    (
                        *axis,
                        if *loc_v > 0.0 {
                            (0.0, *loc_v, *axis_maximum.get(axis).unwrap())
                        } else {
                            (*axis_minimum.get(axis).unwrap(), *loc_v, 0.0)
                        },
                    )
                })
                .collect()
        })
        .collect()
}

impl VariationModel {
    /// Create a new OpenType variation model for the given list of master
    /// locations. Locations must be provided in normalized coordinates (-1..1)
    pub fn new(locations: Vec<Location>, axis_order: Vec<Tag>) -> Self {
        let original_locations = locations.clone();
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
                    .or_insert_with(|| IntoIter::new([F2DOT14::from(0.0)]).collect());
                entry.insert(F2DOT14::from(*value));
            }
        }
        let on_point_count = |loc: &Location| {
            loc.iter()
                .filter(|(&axis, &value)| {
                    axis_points.contains_key(&axis)
                        && axis_points
                            .get(&axis)
                            .unwrap()
                            .contains(&F2DOT14::from(value))
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
            original_locations,
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
            let mut delta_weight: BTreeMap<usize, f32> = BTreeMap::new();
            for (j, support) in self.supports[..i].iter().enumerate() {
                let scalar = support_scalar(loc, support);
                if scalar != 0.0 {
                    delta_weight.insert(j, scalar);
                }
            }
            self.delta_weights.push(delta_weight);
        }
    }

    /// Retrieve the deltas, together with their support regions, for a given
    /// set of master values. Values may be provided for a subset of the model's
    /// locations, although a value must be provided for the default location.
    pub fn get_deltas_and_supports<T>(&self, master_values: &[Option<T>]) -> Vec<(T, Support)>
    where
        T: Sub<Output = T> + Mul<f32, Output = T> + Clone,
    {
        let mut out: Vec<(T, Support)> = vec![];
        let submodel = &VariationModel::new(
            self.original_locations
                .iter()
                .zip(master_values.iter())
                .filter_map(|(loc, value)| value.as_ref().map(|_| loc.clone()))
                .collect(),
            self.axis_order.clone(),
        );
        let master_values: Vec<&T> = master_values.iter().flatten().collect();
        assert_eq!(master_values.len(), submodel.delta_weights.len());
        for (ix, weights) in submodel.delta_weights.iter().enumerate() {
            let support = &submodel.supports[ix];
            let mut delta = master_values[submodel.sort_order.apply_inv_idx(ix)].clone();
            for (&j, &weight) in weights.iter() {
                delta = delta - out[j].0.clone() * weight;
            }
            out.push((delta, support.clone()));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tag;
    use assert_approx_eq::assert_approx_eq;
    use otspec::btreemap;
    use std::iter::FromIterator;

    #[test]
    fn test_support_scalar() {
        assert_approx_eq!(support_scalar(&Location::new(), &Support::new()), 1.0);
        assert_approx_eq!(
            support_scalar(&btreemap!( tag!("wght") => 0.2), &Support::new()),
            1.0
        );
        assert_approx_eq!(
            support_scalar(
                &btreemap!( tag!("wght") => 0.2),
                &btreemap!( tag!("wght") => (0_f32,2_f32,3_f32))
            ),
            0.1
        );
        assert_approx_eq!(
            support_scalar(
                &btreemap!( tag!("wght") => 2.5),
                &btreemap!( tag!("wght") => (0_f32,2_f32,4_f32))
            ),
            0.75
        );
    }

    #[test]
    fn test_variation_model() {
        let locations = vec![
            btreemap!(tag!("wght") => 0.55, tag!("wdth") => 0.0),
            btreemap!(tag!("wght") => -0.55, tag!("wdth") => 0.0),
            btreemap!(tag!("wght") => -1.0, tag!("wdth") => 0.0),
            btreemap!(tag!("wght") => 0.0, tag!("wdth") => 1.0),
            btreemap!(tag!("wght") => 0.66, tag!("wdth") => 1.0),
            btreemap!(tag!("wght") => 0.66, tag!("wdth") => 0.66),
            btreemap!(tag!("wght") => 0.0, tag!("wdth") => 0.0),
            btreemap!(tag!("wght") => 1.0, tag!("wdth") => 1.0),
            btreemap!(tag!("wght") => 1.0, tag!("wdth") => 0.0),
        ];
        let axis_order = vec![tag!("wght")];
        let vm = VariationModel::new(locations, axis_order);
        let expected_locations = vec![
            btreemap!(),
            btreemap!(tag!("wght") => -0.55),
            btreemap!(tag!("wght") => -1.0),
            btreemap!(tag!("wght") => 0.55),
            btreemap!(tag!("wght") => 1.0),
            btreemap!(tag!("wdth") => 1.0),
            btreemap!(tag!("wdth") => 1.0, tag!("wght") => 1.0),
            btreemap!(tag!("wdth") => 1.0, tag!("wght") => 0.66),
            btreemap!(tag!("wdth") => 0.66, tag!("wght") => 0.66),
        ];
        assert_eq!(vm.locations, expected_locations);

        let expected_supports = vec![
            btreemap!(),
            btreemap!(tag!("wght") => (-1.0, -0.55, 0.0)),
            btreemap!(tag!("wght") => (-1.0, -1.0, -0.55)),
            btreemap!(tag!("wght") => (0.0, 0.55, 1.0)),
            btreemap!(tag!("wght") => (0.55, 1.0, 1.0)),
            btreemap!(tag!("wdth") => (0.0, 1.0, 1.0)),
            btreemap!(tag!("wdth") => (0.0, 1.0, 1.0), tag!("wght") => (0.0, 1.0, 1.0)),
            btreemap!(tag!("wdth") => (0.0, 1.0, 1.0), tag!("wght") => (0.0, 0.66, 1.0)),
            btreemap!(tag!("wdth") => (0.0, 0.66, 1.0), tag!("wght") => (0.0, 0.66, 1.0)),
        ];
        assert_eq!(vm.supports, expected_supports);

        assert_eq!(vm.delta_weights[0], btreemap!());
        assert_eq!(vm.delta_weights[1], btreemap!(0 => 1.0));
        assert_eq!(vm.delta_weights[2], btreemap!(0 => 1.0));
        assert_eq!(vm.delta_weights[3], btreemap!(0 => 1.0));
        assert_eq!(vm.delta_weights[4], btreemap!(0 => 1.0));
        assert_eq!(vm.delta_weights[5], btreemap!(0 => 1.0));
        assert_eq!(vm.delta_weights[6], btreemap!(0 => 1.0, 4 => 1.0, 5 => 1.0));
        assert_approx_eq!(vm.delta_weights[7].get(&3).unwrap(), 0.755_555_57);
        assert_approx_eq!(vm.delta_weights[7].get(&4).unwrap(), 0.244_444_49);
        assert_approx_eq!(vm.delta_weights[7].get(&5).unwrap(), 1.0);
        assert_approx_eq!(vm.delta_weights[7].get(&6).unwrap(), 0.66);
    }
}
