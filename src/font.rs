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
        // let header = TableHeader {
        //     sfntVersion: self.sfntVersion as u32,
        //     numTables: lenu16,
        //     searchRange,
        //     entrySelector: log_2(lenu16),
        //     rangeShift: lenu16 * 16 - searchRange,
        //     tableRecords: vec![],
        // };
        // header.serialize(serializer)
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&(self.sfntVersion as u32))?;
        seq.serialize_element(&lenu16)?;
        seq.serialize_element(&searchRange)?;
        seq.serialize_element(&log_2(lenu16))?;
        seq.serialize_element(&(lenu16 * 16 - searchRange))?;
        let mut output: Vec<u8> = vec![];
        let mut pos = 16 * self.tables.len() + 14;
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

    use otspec::ser;

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
        let favar = avar::avar {
            majorVersion: 1,
            minorVersion: 0,
            reserved: 0,
            axisSegmentMaps: vec![
                avar::SegmentMap::new(vec![
                    (-1.0, -1.0),
                    (0.0, 0.0),
                    (0.125, 0.11444),
                    (0.25, 0.2349),
                    (0.5, 0.3554),
                    (0.625, 0.5),
                    (0.75, 0.6566),
                    (0.875, 0.8193),
                    (1.0, 1.0),
                ]),
                avar::SegmentMap::new(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)]),
            ],
        };
        let mut font = font::Font::new(font::SfntVersion::TrueType);
        font.tables.insert(*b"head", font::Table::Head(fhead));
        font.tables.insert(*b"avar", font::Table::Avar(favar));
        let binary_font = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x10, 0x00, 0x01, 0x00, 0x10, 0x68, 0x65,
            0x61, 0x64, 0x00, 0x00, 0x0a, 0x9c, 0x00, 0x00, 0x00, 0x2e, 0x00, 0x00, 0x00, 0x36,
            0x61, 0x76, 0x61, 0x72, 0x00, 0x00, 0x07, 0x11, 0x00, 0x00, 0x00, 0x66, 0x00, 0x00,
            0x00, 0x3c, 0x00, 0x74, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x0a, 0xf8,
            0xfe, 0x61, 0x5f, 0x0f, 0x3c, 0xf5, 0x00, 0x03, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x00,
            0xda, 0x56, 0x58, 0xaa, 0x00, 0x00, 0x00, 0x00, 0xdc, 0x9c, 0x8a, 0x29, 0x00, 0x09,
            0x00, 0x00, 0x02, 0x50, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x06, 0x00, 0x02, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x02, 0x00, 0x09,
            0xc0, 0x00, 0xc0, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08, 0x00, 0x07, 0x53, 0x10, 0x00,
            0x0f, 0x09, 0x20, 0x00, 0x16, 0xbf, 0x28, 0x00, 0x20, 0x00, 0x30, 0x00, 0x2a, 0x06,
            0x38, 0x00, 0x34, 0x6f, 0x40, 0x00, 0x40, 0x00, 0x00, 0x03, 0xc0, 0x00, 0xc0, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x40, 0x00,
        ];
        let serialized = ser::to_bytes(&font).unwrap();
        // let mut buffer = File::create("test.ttf").unwrap();
        // buffer.write_all(&serialized).unwrap();
        assert_eq!(serialized, binary_font);
    }
}
