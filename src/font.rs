use serde::{Serialize, Serializer};
use std::convert::TryInto;
use std::num::Wrapping;
extern crate otspec;
use crate::avar::avar;
use crate::head::head;
use crate::hhea::hhea;
use crate::maxp::maxp;
use indexmap::IndexMap;
use otspec::types::*;

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum Table {
    Unknown(Vec<u8>),
    Avar(avar),
    Head(head),
    Hhea(hhea),
    Maxp(maxp),
}

#[derive(Copy, Clone)]
enum SfntVersion {
    TrueType = 0x00010000,
    OpenType = 0x4F54544F,
}

#[derive(Serialize)]
struct TableRecord {
    tag: Tag,
    checksum: uint32,
    offset: uint32,
    length: uint32,
}
#[derive(Serialize)]
struct TableHeader {
    sfntVersion: u32,
    numTables: u16,
    searchRange: u16,
    entrySelector: u16,
    rangeShift: u16,
    tableRecords: Vec<TableRecord>,
}

struct Font {
    sfntVersion: SfntVersion,
    tables: IndexMap<Tag, Table>,
}

impl Font {
    pub fn new(sfntVersion: SfntVersion) -> Self {
        Self {
            sfntVersion,
            tables: IndexMap::new(),
        }
    }
}

fn log_2(x: u16) -> u16 {
    assert!(x > 0);
    (16 - x.leading_zeros() - 1).try_into().unwrap()
}

fn checksum(x: &[u8]) -> u32 {
    let mut sum = Wrapping(0u32);
    for slice in x.chunks(4) {
        if slice.len() == 4 {
            let maybe_array: [u8; 4] = slice.try_into().unwrap();
            sum += Wrapping(u32::from_be_bytes(maybe_array));
        } else {
            let mut final_bit = [0u8; 4];
            for (&x, p) in slice.iter().zip(final_bit.iter_mut()) {
                *p = x;
            }
            sum += Wrapping(u32::from_be_bytes(final_bit));
        }
    }
    sum.0
}

use serde::ser::SerializeSeq;
impl Serialize for Font {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let lenu16: u16 = self.tables.len().try_into().unwrap();
        let searchRange: u16 = 16 * log_2(lenu16).pow(2);
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&(self.sfntVersion as u32))?;
        seq.serialize_element(&lenu16)?;
        seq.serialize_element(&searchRange)?;
        seq.serialize_element(&log_2(lenu16))?;
        seq.serialize_element(&(lenu16 * 16 - searchRange))?;
        let mut output: Vec<u8> = vec![];
        let mut pos = 16 * self.tables.len() + 12;
        for (tag, value) in &self.tables {
            let mut bytes = otspec::ser::to_bytes(&value).unwrap();
            let orig_len = bytes.len();
            let orig_checksum = checksum(&bytes);
            while (bytes.len() % 4) != 0 {
                bytes.push(0);
            }
            seq.serialize_element(&tag)?;
            seq.serialize_element(&(orig_checksum as u32))?;
            seq.serialize_element(&(pos as u32))?;
            seq.serialize_element(&(orig_len as u32))?;
            pos += bytes.len();
            output.extend(bytes);
        }
        // Compute full checksum and update head here.
        seq.serialize_element(&output)?;
        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use crate::avar;
    use crate::font;
    use crate::head::head;
    use crate::hhea::hhea;
    use crate::maxp::maxp;
    use std::fs::File;
    use std::io::Write;

    use otspec::ser;

    #[test]
    fn test_checksum() {
        let binary_hhea = vec![
            0x00, 0x01, 0x00, 0x00, 0x02, 0xc1, 0xff, 0x4c, 0x00, 0x00, 0x05, 0x1f, 0xfe, 0x82,
            0xfe, 0x82, 0x04, 0xdd, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x5d,
        ];
        assert_eq!(font::checksum(&binary_hhea), 0x0623074B)
    }

    #[test]
    fn font_ser() {
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
        let fmaxp = maxp {
            version: 1.0,
            numGlyphs: 1117,
            maxPoints: 98,
            maxContours: 7,
            maxCompositePoints: 0,
            maxCompositeContours: 0,
            maxZones: 2,
            maxTwilightPoints: 0,
            maxStorage: 0,
            maxFunctionDefs: 0,
            maxInstructionDefs: 0,
            maxStackElements: 0,
            maxSizeOfInstructions: 0,
            maxComponentElements: 0,
            maxComponentDepth: 0,
        };
        let mut font = font::Font::new(font::SfntVersion::TrueType);
        font.tables.insert(*b"head", font::Table::Head(fhead));
        font.tables.insert(*b"hhea", font::Table::Hhea(fhhea));
        font.tables.insert(*b"maxp", font::Table::Maxp(fmaxp));

        let binary_font = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x10, 0x00, 0x01, 0x00, 0x20, 0x68, 0x65,
            0x61, 0x64, 0x23, 0x5b, 0x26, 0x00, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x00, 0x36,
            0x68, 0x68, 0x65, 0x61, 0x06, 0x23, 0x07, 0x4b, 0x00, 0x00, 0x00, 0x74, 0x00, 0x00,
            0x00, 0x24, 0x6d, 0x61, 0x78, 0x70, 0x04, 0x65, 0x00, 0x64, 0x00, 0x00, 0x00, 0x98,
            0x00, 0x00, 0x00, 0x20, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x0a, 0xf8,
            0xfe, 0x61, 0x5f, 0x0f, 0x3c, 0xf5, 0x00, 0x03, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x00,
            0xda, 0x56, 0x58, 0xaa, 0x00, 0x00, 0x00, 0x00, 0xdc, 0x9c, 0x8a, 0x29, 0x00, 0x09,
            0x00, 0x00, 0x02, 0x50, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x06, 0x00, 0x02, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x02, 0xc1, 0xff, 0x4c, 0x00, 0x00,
            0x05, 0x1f, 0xfe, 0x82, 0xfe, 0x82, 0x04, 0xdd, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x5d, 0x00, 0x01,
            0x00, 0x00, 0x04, 0x5d, 0x00, 0x62, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00,
        ];
        let serialized = ser::to_bytes(&font).unwrap();
        // let mut buffer = File::create("test.ttf").unwrap();
        // buffer.write_all(&serialized).unwrap();
        assert_eq!(serialized, binary_font);
    }
}
