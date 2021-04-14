#![allow(non_camel_case_types, non_snake_case, clippy::upper_case_acronyms)]

mod error;
mod head;
mod ser;
mod types;

#[cfg(test)]
mod tests {
    use crate::head::head;
    use crate::ser;
    use crate::types::Fixed;

    #[test]
    fn head_ser() {
        let fhead = head {
            majorVersion: 1,
            minorVersion: 0,
            fontRevision: Fixed(1.0),
            checksumAdjustment: 0xaf8fe61,
            magicNumber: 0x5F0F3CF5,
            flags: 0b0000000000000011,
            unitsPerEm: 1000,
            created: chrono::NaiveDate::from_ymd(2020, 1, 28).and_hms(21, 31, 22),
            modified: chrono::NaiveDate::from_ymd(2021, 4, 14).and_hms(12, 01, 45),
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
}
