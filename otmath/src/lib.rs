/// Mathematical operations useful for OpenType fonts
use core::cmp::Ordering;
use core::ops::{Mul, Sub};
use permutation::Permutation;
use std::collections::{BTreeMap, HashSet};

/// Round a number to a 32 bit integer using OpenType rounding
///
/// The OpenType spec (in the section on ["normalization" of OpenType Font
/// Variations](https://docs.microsoft.com/en-us/typography/opentype/spec/otvaroverview#coordinate-scales-and-normalization>)
/// defines the required method for converting floating point values to
/// fixed-point. In particular it specifies the following rounding strategy:
/// > for fractional values of 0.5 and higher, take the next higher integer;
/// > for other fractional values, truncate.
/// This function rounds the floating-point value according to this strategy
/// in preparation for conversion to fixed-point.
pub fn ot_round<T>(value: T) -> i32
where
    T: Into<f64>,
{
    (value.into() as f32 + 0.5_f32).floor() as i32
}

/// Compare two floating point values using their OpenType fixed-point
/// equivalents.
pub fn ot_cmp(a: f32, b: f32) -> Ordering {
    ot_round(a * 16384.0).cmp(&ot_round(b * 16384.0))
}

/// Convert a floating point value in the range -1.0 .. 1.0 to its
/// OpenType fixed-point equivalent.
pub fn to_f2dot14(a: f32) -> i32 {
    ot_round(a * 16384.0)
}

/// Perform a piecewise linear mapping of a value.
///
/// This is most used to remap the design space of a variable font.
pub fn piecewise_linear_map(mapping: &[(f32, f32)], value: f32) -> f32 {
    if let Some(exact) = mapping
        .iter()
        .find(|(a, _b)| ot_cmp(*a, value) == Ordering::Equal)
    {
        return exact.1;
    }
    if mapping.is_empty() {
        return value;
    }
    let (min, mapped_min) = mapping.first().unwrap();
    if ot_cmp(value, *min) == Ordering::Less {
        return value + mapped_min - min;
    }
    let (max, mapped_max) = mapping.last().unwrap();
    if ot_cmp(value, *max) == Ordering::Greater {
        return value + mapped_max - max;
    }
    let (a, va) = mapping
        .iter()
        .filter(|(k, _v)| ot_cmp(*k, value) == Ordering::Less)
        .max_by(|(k1, _v1), (k2, _v2)| ot_cmp(*k1, *k2))
        .unwrap();

    let (b, vb) = mapping
        .iter()
        .filter(|(k, _v)| ot_cmp(*k, value) == Ordering::Greater)
        .min_by(|(k1, _v1), (k2, _v2)| ot_cmp(*k1, *k2))
        .unwrap();
    va + (vb - va) * (value - a) / (*b - *a)
}

/// Normalize a value along a min/default/max triplet to the range -1.0/0.0/1.0.
pub fn normalize_value(mut l: f32, min: f32, max: f32, default: f32) -> f32 {
    if l < min {
        l = min;
    }
    if l > max {
        l = max;
    }
    if l < default {
        -(default - l) / (default - min) as f32
    } else if l > default {
        (l - default) / (max - default) as f32
    } else {
        0_f32
    }
}

/// A region of the designspace, consisting of a set of per-axis triangular tents
pub type Support<T> = BTreeMap<T, (f32, f32, f32)>;
/// A location as a mapping of tags to user-space values
pub type Location<T> = BTreeMap<T, f32>;
type AxisPoints<T> = BTreeMap<T, HashSet<i32>>;

/// An OpenType variation model helps to determine and interpolate the correct
/// supports and deltasets when there are intermediate masters.
#[derive(Debug)]
pub struct VariationModel<T> {
    /// The rearranged list of master locations
    pub locations: Vec<Location<T>>,
    sort_order: Permutation,
    /// The supports computed for each master
    pub supports: Vec<Support<T>>,
    /// The axis order provided by the user
    pub axis_order: Vec<T>,
    /// The original, unordered list of locations
    pub original_locations: Vec<Location<T>>,
    delta_weights: Vec<BTreeMap<usize, f32>>,
}

/// Returns the contribution value of a region at a given location
pub fn support_scalar<T: Ord + Clone>(loc: &Location<T>, support: &Support<T>) -> f32 {
    let mut scalar = 1.0;
    for (axis, (lower, peak, upper)) in support.clone().into_iter() {
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

fn locations_to_regions<T>(locations: &[Location<T>]) -> Vec<Support<T>>
where
    T: Ord + Clone,
{
    let mut axis_minimum: BTreeMap<&T, f32> = BTreeMap::new();
    let mut axis_maximum: BTreeMap<&T, f32> = BTreeMap::new();
    for (tag, value) in locations.iter().flatten() {
        axis_maximum
            .entry(tag)
            .and_modify(|v| *v = v.max(*value))
            .or_insert(*value);
        axis_minimum
            .entry(tag)
            .and_modify(|v| *v = v.min(*value))
            .or_insert(*value);
    }
    locations
        .iter()
        .map(|loc| {
            loc.iter()
                .map(|(axis, loc_v)| {
                    (
                        axis.clone(),
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

impl<T> VariationModel<T>
where
    T: Ord + Eq + Clone + std::hash::Hash,
{
    /// Create a new OpenType variation model for the given list of master
    /// locations. Locations must be provided in normalized coordinates (-1..1)
    pub fn new(locations: Vec<Location<T>>, axis_order: Vec<T>) -> Self {
        let original_locations = locations.clone();
        let locations: Vec<Location<T>> = locations
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
                    .entry(axis)
                    .or_insert_with(|| vec![to_f2dot14(0.0)].into_iter().collect());
                entry.insert(to_f2dot14(*value));
            }
        }
        let on_point_count = |loc: &Location<T>| {
            loc.iter()
                .filter(|(axis, &value)| {
                    axis_points.contains_key(axis)
                        && axis_points.get(axis).unwrap().contains(&to_f2dot14(value))
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

            let mut a_ordered_axes: Vec<T> = a.keys().cloned().collect();
            let mut b_ordered_axes: Vec<T> = b.keys().cloned().collect();
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
            let loc_axes: HashSet<T> = region.keys().cloned().collect();
            let mut region_copy = region.clone();
            for prev_region in &regions[..i] {
                let prev_loc_axes: HashSet<T> = prev_region.keys().cloned().collect();
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
                let mut best_axes: Support<T> = Support::new();
                let mut best_ratio = -1_f32;
                for (axis, &(_, val, _)) in prev_region.iter() {
                    let &(lower, loc_v, upper) = region.get(axis).unwrap();
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
                        best_axes.insert(axis.clone(), (new_lower, loc_v, new_upper));
                    }
                }
                for (axis, triple) in best_axes.iter() {
                    region_copy.insert(axis.clone(), *triple);
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
    pub fn get_deltas_and_supports<U>(&self, master_values: &[Option<U>]) -> Vec<(U, Support<T>)>
    where
        U: Sub<Output = U> + Mul<f32, Output = U> + Clone,
    {
        let mut out: Vec<(U, Support<T>)> = vec![];
        let submodel = &VariationModel::new(
            self.original_locations
                .iter()
                .zip(master_values.iter())
                .filter_map(|(loc, value)| value.as_ref().map(|_| loc.clone()))
                .collect(),
            self.axis_order.clone(),
        );
        let master_values: Vec<&U> = master_values.iter().flatten().collect();
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

    pub fn get_scalars(&self, loc: &Location<T>) -> Vec<f32> {
        self.supports
            .iter()
            .map(|x| support_scalar(loc, x))
            .collect()
    }

    pub fn interpolate_from_deltas_and_scalars<U>(&self, deltas: &[U], scalars: &[f32]) -> Option<U>
    where
        U: Sub<Output = U>
            + Mul<f32, Output = U>
            + Clone
            + std::ops::Add<Output = U>
            + std::fmt::Debug,
    {
        let mut v = None;
        assert_eq!(deltas.len(), scalars.len());
        for (delta, &scalar) in deltas.iter().zip(scalars.iter()) {
            let contribution = delta.clone() * scalar;
            if v.is_none() {
                v = Some(contribution);
            } else {
                v = Some(v.unwrap() + contribution);
            }
        }
        v
    }

    pub fn interpolate_from_deltas<U>(&self, loc: &Location<T>, deltas: &[U]) -> Option<U>
    where
        U: Sub<Output = U>
            + Mul<f32, Output = U>
            + Clone
            + std::ops::Add<Output = U>
            + std::fmt::Debug,
    {
        self.interpolate_from_deltas_and_scalars(deltas, &self.get_scalars(loc))
    }

    pub fn interpolate_from_masters<U>(
        &self,
        loc: &Location<T>,
        master_values: &[Option<U>],
    ) -> Option<U>
    where
        U: Sub<Output = U>
            + Mul<f32, Output = U>
            + Clone
            + std::ops::Add<Output = U>
            + std::fmt::Debug,
    {
        let deltas_and_supports = self.get_deltas_and_supports(master_values);
        let deltas: Vec<U> = deltas_and_supports.into_iter().map(|(x, _y)| x).collect();
        // XXX This doesn't deal with sparse masters
        self.interpolate_from_deltas(loc, &deltas)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_approx_eq::assert_approx_eq;
    use std::iter::FromIterator;

    macro_rules! btreemap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::BTreeMap::<_, _>::from_iter([$(($k, $v),)*])
        };
    }

    #[test]
    fn test_support_scalar() {
        assert_approx_eq!(
            support_scalar(&Location::<&str>::new(), &Support::new()),
            1.0
        );
        assert_approx_eq!(
            support_scalar(&btreemap!( ("wght") => 0.2), &Support::new()),
            1.0
        );
        assert_approx_eq!(
            support_scalar(
                &btreemap!( ("wght") => 0.2),
                &btreemap!( ("wght") => (0_f32,2_f32,3_f32))
            ),
            0.1
        );
        assert_approx_eq!(
            support_scalar(
                &btreemap!( ("wght") => 2.5),
                &btreemap!( ("wght") => (0_f32,2_f32,4_f32))
            ),
            0.75
        );
    }

    #[test]
    fn test_variation_model() {
        let locations = vec![
            btreemap!(("wght") => 0.55, ("wdth") => 0.0),
            btreemap!(("wght") => -0.55, ("wdth") => 0.0),
            btreemap!(("wght") => -1.0, ("wdth") => 0.0),
            btreemap!(("wght") => 0.0, ("wdth") => 1.0),
            btreemap!(("wght") => 0.66, ("wdth") => 1.0),
            btreemap!(("wght") => 0.66, ("wdth") => 0.66),
            btreemap!(("wght") => 0.0, ("wdth") => 0.0),
            btreemap!(("wght") => 1.0, ("wdth") => 1.0),
            btreemap!(("wght") => 1.0, ("wdth") => 0.0),
        ];
        let axis_order = vec![("wght")];
        let vm = VariationModel::new(locations, axis_order);
        let expected_locations = vec![
            btreemap!(),
            btreemap!(("wght") => -0.55),
            btreemap!(("wght") => -1.0),
            btreemap!(("wght") => 0.55),
            btreemap!(("wght") => 1.0),
            btreemap!(("wdth") => 1.0),
            btreemap!(("wdth") => 1.0, ("wght") => 1.0),
            btreemap!(("wdth") => 1.0, ("wght") => 0.66),
            btreemap!(("wdth") => 0.66, ("wght") => 0.66),
        ];
        assert_eq!(vm.locations, expected_locations);

        let expected_supports = vec![
            btreemap!(),
            btreemap!(("wght") => (-1.0, -0.55, 0.0)),
            btreemap!(("wght") => (-1.0, -1.0, -0.55)),
            btreemap!(("wght") => (0.0, 0.55, 1.0)),
            btreemap!(("wght") => (0.55, 1.0, 1.0)),
            btreemap!(("wdth") => (0.0, 1.0, 1.0)),
            btreemap!(("wdth") => (0.0, 1.0, 1.0), ("wght") => (0.0, 1.0, 1.0)),
            btreemap!(("wdth") => (0.0, 1.0, 1.0), ("wght") => (0.0, 0.66, 1.0)),
            btreemap!(("wdth") => (0.0, 0.66, 1.0), ("wght") => (0.0, 0.66, 1.0)),
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
