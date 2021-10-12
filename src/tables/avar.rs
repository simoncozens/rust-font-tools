use otspec::types::*;
use otspec::Deserializer;
use otspec_macros::tables;

/// The 'avar' OpenType tag.
pub const TAG: Tag = crate::tag!("avar");

tables!(
    AxisValueMap {
        F2DOT14 fromCoordinate
        F2DOT14 toCoordinate
    }
    SegmentMap {
        Counted(AxisValueMap) axisValueMaps
    }

    avar {
        uint16 majorVersion
        uint16 minorVersion
        uint16 reserved
        Counted(SegmentMap) axisSegmentMaps
    }
);

impl SegmentMap {
    /// Creates a new segment map from an array of tuples. These tuples
    /// must be in normalized coordinates, and *must* include entries for
    /// `-1.0,-1.0`, `0.0,0.0` and `1.0,1.0`.
    // XXX we should probably check this and insert them if not.
    pub fn new(items: Vec<(f32, f32)>) -> Self {
        let maps: Vec<AxisValueMap> = items
            .iter()
            .map(|i| AxisValueMap {
                fromCoordinate: i.0,
                toCoordinate: i.1,
            })
            .collect();
        let new_thing = SegmentMap {
            axisValueMaps: maps,
        };
        if !new_thing.is_valid() {
            panic!("Created an invalid segment map {:?}", new_thing);
        }
        new_thing
    }

    /// Map a (normalized, i.e. `-1.0<=val<=1.0`) value using this segment map.
    pub fn piecewise_linear_map(&self, val: f32) -> f32 {
        let from: Vec<f32> = self
            .axisValueMaps
            .iter()
            .map(|x| x.fromCoordinate)
            .collect();
        let to: Vec<f32> = self.axisValueMaps.iter().map(|x| x.toCoordinate).collect();
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
        for axm in &self.axisValueMaps {
            if axm.fromCoordinate == 0.0 && axm.toCoordinate == 0.0 {
                saw_zero += 1;
            }
            if (axm.fromCoordinate - -1.0).abs() < f32::EPSILON
                && (axm.toCoordinate - -1.0).abs() < f32::EPSILON
            {
                saw_minus1 += 1;
            }
            if (axm.fromCoordinate - 1.0).abs() < f32::EPSILON
                && (axm.toCoordinate - 1.0).abs() < f32::EPSILON
            {
                saw_plus1 += 1;
            }

            // Check for sortedness
            if axm.toCoordinate < prev_to_coordinate {
                return false;
            }
            prev_to_coordinate = axm.toCoordinate;
        }
        if saw_zero != 1 || saw_plus1 != 1 || saw_minus1 != 1 {
            return false;
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use otspec::ser;

    /* All numbers here carefully chosen to avoid OT rounding errors... */
    #[test]
    fn avar_axis_value_map_serde() {
        let v = super::AxisValueMap {
            fromCoordinate: 0.2999878,
            toCoordinate: 0.5,
        };
        let binary_avarmap = ser::to_bytes(&v).unwrap();
        let deserialized: super::AxisValueMap = otspec::de::from_bytes(&binary_avarmap).unwrap();
        assert_eq!(deserialized, v);
    }

    #[test]
    fn avar_ser() {
        let favar = super::avar {
            majorVersion: 1,
            minorVersion: 0,
            reserved: 0,
            axisSegmentMaps: vec![
                super::SegmentMap::new(vec![
                    (-1.0, -1.0),
                    (0.0, 0.0),
                    (0.125, 0.11444092),
                    (0.25, 0.23492432),
                    (0.5, 0.3554077),
                    (0.625, 0.5),
                    (0.75, 0.6566162),
                    (0.875, 0.8192749),
                    (1.0, 1.0),
                ]),
                super::SegmentMap::new(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)]),
            ],
        };
        let binary_avar = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x09, 0xc0, 0x00, 0xc0, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x07, 0x53, 0x10, 0x00, 0x0f, 0x09, 0x20, 0x00,
            0x16, 0xbf, 0x28, 0x00, 0x20, 0x00, 0x30, 0x00, 0x2a, 0x06, 0x38, 0x00, 0x34, 0x6f,
            0x40, 0x00, 0x40, 0x00, 0x00, 0x03, 0xc0, 0x00, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x40, 0x00, 0x40, 0x00,
        ];
        assert_eq!(ser::to_bytes(&favar).unwrap(), binary_avar);

        let deserialized: super::avar = otspec::de::from_bytes(&binary_avar).unwrap();
        assert_eq!(deserialized, favar);
    }

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
