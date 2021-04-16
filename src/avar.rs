#![allow(non_camel_case_types, non_snake_case)]

use otspec::types::*;
use otspec_macros::tables;
use serde::{Deserialize, Serialize};

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
    pub fn new(items: Vec<(f32, f32)>) -> Self {
        let maps = items
            .iter()
            .map(|i| AxisValueMap {
                fromCoordinate: i.0,
                toCoordinate: i.1,
            })
            .collect();
        SegmentMap {
            axisValueMaps: maps,
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::avar;
    use otspec::ser;

    /* All numbers here carefully chosen to avoid OT rounding errors... */
    #[test]
    fn avar_axis_value_map_serde() {
        let v = avar::AxisValueMap {
            fromCoordinate: 0.2999878,
            toCoordinate: 0.5,
        };
        let binary_avarmap = ser::to_bytes(&v).unwrap();
        let deserialized: avar::AxisValueMap = otspec::de::from_bytes(&binary_avarmap).unwrap();
        assert_eq!(deserialized, v);
    }

    #[test]
    fn avar_ser() {
        let favar = avar::avar {
            majorVersion: 1,
            minorVersion: 0,
            reserved: 0,
            axisSegmentMaps: vec![
                avar::SegmentMap::new(vec![
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
                avar::SegmentMap::new(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)]),
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

        let deserialized: avar::avar = otspec::de::from_bytes(&binary_avar).unwrap();
        assert_eq!(deserialized, favar);
    }
}
