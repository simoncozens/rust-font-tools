use serde::{Deserialize, Serialize};

extern crate otspec;

use otspec::types::*;
use otspec_macros::tables;

tables!(hhea {
    uint16 majorVersion
    uint16 minorVersion
    FWORD ascender
    FWORD descender
    FWORD lineGap
    UFWORD  advanceWidthMax
    FWORD   minLeftSideBearing
    FWORD   minRightSideBearing
    FWORD   xMaxExtent
    int16   caretSlopeRise
    int16   caretSlopeRun
    int16   caretOffset
    int16   reserved0
    int16   reserved1
    int16   reserved2
    int16   reserved3
    int16   metricDataFormat
    uint16  numberOfHMetrics
});

#[cfg(test)]
mod tests {
    use crate::hhea::hhea;
    use otspec::de;
    use otspec::ser;

    #[test]
    fn hhea_ser() {
        let fhhea = hhea {
            majorVersion: 1,
            minorVersion: 0,
            ascender: 705,
            descender: -180,
            lineGap: 0,
            advanceWidthMax: 1311,
            minLeftSideBearing: -382,
            minRightSideBearing: -382,
            xMaxExtent: 1245,
            caretSlopeRise: 1,
            caretSlopeRun: 0,
            caretOffset: 0,
            reserved0: 0,
            reserved1: 0,
            reserved2: 0,
            reserved3: 0,
            metricDataFormat: 0,
            numberOfHMetrics: 1117,
        };
        let binary_hhea = vec![
            0x00, 0x01, 0x00, 0x00, 0x02, 0xc1, 0xff, 0x4c, 0x00, 0x00, 0x05, 0x1f, 0xfe, 0x82,
            0xfe, 0x82, 0x04, 0xdd, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x5d,
        ];
        assert_eq!(ser::to_bytes(&fhhea).unwrap(), binary_hhea);
    }

    #[test]
    fn hhea_de() {
        let fhhea = hhea {
            majorVersion: 1,
            minorVersion: 0,
            ascender: 705,
            descender: -180,
            lineGap: 0,
            advanceWidthMax: 1311,
            minLeftSideBearing: -382,
            minRightSideBearing: -382,
            xMaxExtent: 1245,
            caretSlopeRise: 1,
            caretSlopeRun: 0,
            caretOffset: 0,
            reserved0: 0,
            reserved1: 0,
            reserved2: 0,
            reserved3: 0,
            metricDataFormat: 0,
            numberOfHMetrics: 1117,
        };
        let binary_hhea = vec![
            0x00, 0x01, 0x00, 0x00, 0x02, 0xc1, 0xff, 0x4c, 0x00, 0x00, 0x05, 0x1f, 0xfe, 0x82,
            0xfe, 0x82, 0x04, 0xdd, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x5d,
        ];
        let deserialized: hhea = otspec::de::from_bytes(&binary_hhea).unwrap();
        assert_eq!(deserialized, fhhea);
    }
}
