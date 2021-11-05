use crate::table_delegate;
use otspec::tables::avar::{avar as avar_ot, AxisValueMap, SegmentMap as SegmentMap_ot};
use otspec::types::*;
use otspec::{Deserialize, Deserializer, Serialize};

/// The 'avar' OpenType tag.
pub const TAG: Tag = crate::tag!("avar");

#[derive(Debug, PartialEq, Clone)]
pub struct SegmentMap(pub Vec<(f32, f32)>);

#[derive(Debug, PartialEq, Clone)]
pub struct avar {
    pub maps: Vec<SegmentMap>,
}

impl Into<avar_ot> for &avar {
    fn into(self) -> avar_ot {
        avar_ot {
            majorVersion: 1,
            minorVersion: 0,
            reserved: 0,
            axisSegmentMaps: self.maps.iter().map(|x| x.into()).collect(),
        }
    }
}

impl Into<avar> for avar_ot {
    fn into(self) -> avar {
        avar {
            maps: self
                .axisSegmentMaps
                .iter()
                .map(|x| x.clone().into())
                .collect(),
        }
    }
}

table_delegate!(avar, avar_ot);

impl Into<SegmentMap_ot> for &SegmentMap {
    fn into(self) -> SegmentMap_ot {
        let maps: Vec<AxisValueMap> = self
            .0
            .iter()
            .map(|i| AxisValueMap {
                fromCoordinate: i.0,
                toCoordinate: i.1,
            })
            .collect();
        SegmentMap_ot {
            axisValueMaps: maps,
        }
    }
}

impl Into<SegmentMap> for SegmentMap_ot {
    fn into(self) -> SegmentMap {
        SegmentMap(
            self.axisValueMaps
                .iter()
                .map(|avm| (avm.fromCoordinate, avm.toCoordinate))
                .collect(),
        )
    }
}
impl SegmentMap {
    /// Creates a new segment map from an array of tuples. These tuples
    /// must be in normalized coordinates, and *must* include entries for
    /// `-1.0,-1.0`, `0.0,0.0` and `1.0,1.0`.
    // XXX we should probably check this and insert them if not.
    pub fn new(items: Vec<(f32, f32)>) -> Self {
        let new_thing = SegmentMap(items);
        if !new_thing.is_valid() {
            panic!("Created an invalid segment map {:?}", new_thing);
        }
        new_thing
    }

    /// Map a (normalized, i.e. `-1.0<=val<=1.0`) value using this segment map.
    pub fn piecewise_linear_map(&self, val: f32) -> f32 {
        let from: Vec<f32> = self.0.iter().map(|x| x.0).collect();
        let to: Vec<f32> = self.0.iter().map(|x| x.1).collect();
        if val <= -1.0 {
            return -1.0;
        }
        if val >= 1.0 {
            return 1.0;
        }
        if let Some(ix) = from.iter().position(|&r| (r - val).abs() < f32::EPSILON) {
            return to[ix];
        }
        if let Some(ix) = from.iter().position(|&r| r > val) {
            let a = from[ix - 1];
            let b = from[ix];
            let va = to[ix - 1];
            let vb = to[ix];
            va + (vb - va) * (val - a) / (b - a)
        } else {
            panic!("Can't happen")
        }
    }

    /// Check that this segment map is valid
    /// This means that it contains entries for -1,0,1 and that the entries are in order
    pub fn is_valid(&self) -> bool {
        let mut saw_zero = 0;
        let mut saw_minus1 = 0;
        let mut saw_plus1 = 0;
        let mut prev_to_coordinate = -2.0;
        for map in &self.0 {
            let (from, to) = (map.0, map.1);
            if from == 0.0 && to == 0.0 {
                saw_zero += 1;
            }
            if (from - -1.0).abs() < f32::EPSILON && (to - -1.0).abs() < f32::EPSILON {
                saw_minus1 += 1;
            }
            if (from - 1.0).abs() < f32::EPSILON && (to - 1.0).abs() < f32::EPSILON {
                saw_plus1 += 1;
            }

            // Check for sortedness
            if to < prev_to_coordinate {
                return false;
            }
            prev_to_coordinate = to;
        }
        if saw_zero != 1 || saw_plus1 != 1 || saw_minus1 != 1 {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn test_piecewise_linear_map() {
        let seg = super::SegmentMap::new(vec![
            (-1.0, -1.0),
            (0.0, 0.0),
            (0.125, 0.11444092),
            (0.25, 0.23492432),
            (0.5, 0.3554077),
            (0.625, 0.5),
            (0.75, 0.6566162),
            (0.875, 0.8192749),
            (1.0, 1.0),
        ]);
        assert!((seg.piecewise_linear_map(-2.5) - -1.0).abs() < f32::EPSILON);
        assert!((seg.piecewise_linear_map(0.0) - 0.0).abs() < f32::EPSILON);
        assert!((seg.piecewise_linear_map(1.0) - 1.0).abs() < f32::EPSILON);
        assert!((seg.piecewise_linear_map(2.0) - 1.0).abs() < f32::EPSILON);
        assert!((seg.piecewise_linear_map(0.625) - 0.5).abs() < f32::EPSILON);
        assert!((seg.piecewise_linear_map(0.6) - 0.47108155).abs() < f32::EPSILON);
    }
}
