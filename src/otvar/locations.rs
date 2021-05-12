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
type Location = HashMap<Tag, f32>;
type AxisPoints = HashMap<Tag, HashSet<i16>>;

pub struct VariationModel {
    locations: Vec<Location>,
    sort_order: Permutation,
    supports: Vec<Support>,
    axis_order: Vec<Tag>,
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
        for loc in locations.iter().filter(|l| l.len() != 1) {
            if let Some((axis, value)) = loc.iter().next() {
                let entry = axis_points
                    .entry(*axis)
                    .or_insert(IntoIter::new([F2DOT14::pack(0.0)]).collect());
                entry.insert(F2DOT14::pack(*value));
            }
        }

        let sort_order = permutation::sort_by(&indices[..], |a_ix, b_ix| {
            let a = &locations[*a_ix];
            let b = &locations[*b_ix];
            if a.keys().len() != b.keys().len() {
                return a.keys().len().cmp(&b.keys().len());
            }
            let a_on_point = a
                .iter()
                .filter(|(&axis, &value)| {
                    axis_points.contains_key(&axis)
                        && axis_points
                            .get(&axis)
                            .unwrap()
                            .contains(&F2DOT14::pack(value))
                })
                .count();
            let b_on_point = b
                .iter()
                .filter(|(&axis, &value)| {
                    axis_points.contains_key(&axis)
                        && axis_points
                            .get(&axis)
                            .unwrap()
                            .contains(&F2DOT14::pack(value))
                })
                .count();
            if a_on_point != b_on_point {
                return b_on_point.cmp(&a_on_point);
            }
            // This is wrong
            unimplemented!()
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

    pub fn get_deltas(&self, master_values: &[f32]) -> Vec<f32> {
        assert_eq!(master_values.len(), self.delta_weights.len());
        let mut out = vec![];
        let reordered_masters = self.sort_order.apply_inv_slice(master_values);
        for (weights, &delta) in self.delta_weights.iter().zip(reordered_masters.iter()) {
            let mut delta = delta;
            for (&j, &weight) in weights.iter() {
                delta -= out[j] * weight;
            }
            out.push(delta);
        }
        out
    }
}
