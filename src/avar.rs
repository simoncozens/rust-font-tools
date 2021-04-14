#![allow(non_camel_case_types, non_snake_case)]

use crate::types::{uint16, Counted, F2DOT14};
use serde::Serialize;

// #[derive(Serialize, Debug, PartialEq)]
// pub struct AxisValueMap {
//     pub fromCoordinate: F2DOT14,
//     pub toCoordinate: F2DOT14,
// }

#[derive(Serialize, Debug, PartialEq)]
pub struct SegmentMap {
    #[serde(with = "Counted")]
    pub axisValueMaps: Vec<(F2DOT14, F2DOT14)>,
}

#[derive(Serialize, Debug, PartialEq)]
pub struct avar {
    pub majorVersion: uint16,
    pub minorVersion: uint16,
    pub reserved: uint16,
    #[serde(with = "Counted")]
    pub axisSegmentMaps: Vec<SegmentMap>,
}

#[cfg(test)]
mod tests {
    use crate::avar;
    use crate::ser;
    use crate::types::F2DOT14;

    #[test]
    fn avar_ser() {
        let favar = avar::avar {
            majorVersion: 1,
            minorVersion: 0,
            reserved: 0,
            axisSegmentMaps: vec![
                avar::SegmentMap {
                    axisValueMaps: vec![
                        (F2DOT14(-1.0), F2DOT14(-1.0)),
                        (F2DOT14(0.0), F2DOT14(0.0)),
                        (F2DOT14(0.125), F2DOT14(0.11444)),
                        (F2DOT14(0.25), F2DOT14(0.2349)),
                        (F2DOT14(0.5), F2DOT14(0.3554)),
                        (F2DOT14(0.625), F2DOT14(0.5)),
                        (F2DOT14(0.75), F2DOT14(0.6566)),
                        (F2DOT14(0.875), F2DOT14(0.8193)),
                        (F2DOT14(1.0), F2DOT14(1.0)),
                    ],
                },
                avar::SegmentMap {
                    axisValueMaps: vec![
                        (F2DOT14(-1.0), F2DOT14(-1.0)),
                        (F2DOT14(0.0), F2DOT14(0.0)),
                        (F2DOT14(1.0), F2DOT14(1.0)),
                    ],
                },
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
    }
}
