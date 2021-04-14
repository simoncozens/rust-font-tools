use serde::Serialize;

extern crate otspec;

use otspec::types::*;
use otspec_macros::table;

table!(head {
    uint16 majorVersion
    uint16 minorVersion
    Fixed fontRevision
    uint32 checksumAdjustment
    uint32 magicNumber
    uint16 flags
    uint16 unitsPerEm
    LONGDATETIME created
    LONGDATETIME modified
    int16 xMin
    int16 yMin
    int16 xMax
    int16 yMax
    uint16 macStyle
    uint16 lowestRecPPEM
    int16 fontDirectionHint
    int16 indexToLocFormat
    int16 glyphDataFormat
});

#[cfg(test)]
mod tests {
    use crate::head::head;
    use otspec::ser;

    #[test]
    fn head_ser() {
        let fhead = head {
            majorVersion: 1,
            minorVersion: 0,
            fontRevision: 1.0,
            checksumAdjustment: 0xaf8fe61,
            magicNumber: 0x5F0F3CF5,
            flags: 0b0000000000000011,
            unitsPerEm: 1000,
            created: chrono::NaiveDate::from_ymd(2020, 1, 28).and_hms(21, 31, 22),
            modified: chrono::NaiveDate::from_ymd(2021, 4, 14).and_hms(12, 1, 45),
            xMin: 9,
            yMin: 0,
            xMax: 592,
            yMax: 1000,
            macStyle: 0,
            lowestRecPPEM: 6,
            fontDirectionHint: 2,
            indexToLocFormat: 1,
            glyphDataFormat: 0,
        };
        let binary_head = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x0a, 0xf8, 0xfe, 0x61, 0x5f, 0x0f,
            0x3c, 0xf5, 0x00, 0x03, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x00, 0xda, 0x56, 0x58, 0xaa,
            0x00, 0x00, 0x00, 0x00, 0xdc, 0x9c, 0x8a, 0x29, 0x00, 0x09, 0x00, 0x00, 0x02, 0x50,
            0x03, 0xe8, 0x00, 0x00, 0x00, 0x06, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00,
        ];
        assert_eq!(ser::to_bytes(&fhead).unwrap(), binary_head);
    }

    // #[test]
    // fn head_de() {
    //     let fhead = head {
    //         majorVersion: 1,
    //         minorVersion: 0,
    //         fontRevision: Fixed(1.0),
    //         checksumAdjustment: 0xaf8fe61,
    //         magicNumber: 0x5F0F3CF5,
    //         flags: 0b0000000000000011,
    //         unitsPerEm: 1000,
    //         created: chrono::NaiveDate::from_ymd(2020, 1, 28).and_hms(21, 31, 22),
    //         modified: chrono::NaiveDate::from_ymd(2021, 4, 14).and_hms(12, 1, 45),
    //         xMin: 9,
    //         yMin: 0,
    //         xMax: 592,
    //         yMax: 1000,
    //         macStyle: 0,
    //         lowestRecPPEM: 6,
    //         fontDirectionHint: 2,
    //         indexToLocFormat: 1,
    //         glyphDataFormat: 0,
    //     };
    //     let binary_head = vec![
    //         0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x0a, 0xf8, 0xfe, 0x61, 0x5f, 0x0f,
    //         0x3c, 0xf5, 0x00, 0x03, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x00, 0xda, 0x56, 0x58, 0xaa,
    //         0x00, 0x00, 0x00, 0x00, 0xdc, 0x9c, 0x8a, 0x29, 0x00, 0x09, 0x00, 0x00, 0x02, 0x50,
    //         0x03, 0xe8, 0x00, 0x00, 0x00, 0x06, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00,
    //     ];
    //     let deserialized: head = crate::de::from_bytes(&binary_head).unwrap();
    //     assert_eq!(deserialized, fhead);
    // }
}
