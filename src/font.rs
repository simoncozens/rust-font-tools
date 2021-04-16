use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use serde::{Serialize, Serializer};
use std::convert::{TryFrom, TryInto};
use std::num::Wrapping;
extern crate otspec;
use crate::avar::avar;
use crate::head::head;
use crate::hhea::hhea;
use crate::maxp::maxp;
use indexmap::IndexMap;
use otspec::types::*;
use std::fs::File;
use std::io::Write;

#[derive(Debug, Serialize, PartialEq)]
#[serde(untagged)]
enum Table {
    Unknown(Vec<u8>),
    Avar(avar),
    Head(head),
    Hhea(hhea),
    Maxp(maxp),
}

#[derive(Copy, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub enum SfntVersion {
    TrueType = 0x00010000,
    OpenType = 0x4F54544F,
}

impl TryFrom<u32> for SfntVersion {
    type Error = ();

    fn try_from(v: u32) -> Result<Self, Self::Error> {
        match v {
            x if x == SfntVersion::TrueType as u32 => Ok(SfntVersion::TrueType),
            x if x == SfntVersion::OpenType as u32 => Ok(SfntVersion::OpenType),
            _ => Err(()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct TableRecord {
    tag: Tag,
    checksum: uint32,
    offset: uint32,
    length: uint32,
}
#[derive(Deserialize)]
struct TableHeader {
    sfntVersion: u32,
    numTables: u16,
    searchRange: u16,
    entrySelector: u16,
    rangeShift: u16,
}

#[derive(Debug)]
pub struct Font {
    sfntVersion: SfntVersion,
    tables: IndexMap<Tag, Table>,
}

use otspec::ser;

impl Font {
    pub fn new(sfntVersion: SfntVersion) -> Self {
        Self {
            sfntVersion,
            tables: IndexMap::new(),
        }
    }

    pub fn save(&self, filename: &str) {
        let serialized = ser::to_bytes(&self).unwrap();
        let mut buffer = File::create(filename).unwrap();
        buffer.write_all(&serialized).unwrap();
    }
}

use std::error::Error;
use std::fs;

pub fn load(filename: &str) -> Result<Font, Box<dyn Error>> {
    let buffer = fs::read(&filename)?;
    otspec::de::from_bytes(&buffer).map_err(|e| e.into())
}

impl PartialEq for Font {
    fn eq(&self, other: &Self) -> bool {
        if self.sfntVersion != other.sfntVersion || self.tables.len() != other.tables.len() {
            return false;
        }
        for ((k1, v1), (k2, v2)) in self.tables.iter().zip(other.tables.iter()) {
            if k1 != k2 || v1 != v2 {
                return false;
            }
        }
        true
    }
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

impl Serialize for Font {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let lenu16: u16 = self.tables.len().try_into().unwrap();
        let mut max_pow2: u16 = 0;
        while 1u16 << (max_pow2 + 1) <= lenu16 {
            max_pow2 += 1;
        }
        let searchRange: u16 = (1u16 << max_pow2) << 4;
        // let mut seq = serializer.serialize_seq(None)?;
        let mut output: Vec<u8> = vec![];
        let mut output_tables: Vec<u8> = vec![];
        output.extend(&(self.sfntVersion as u32).to_be_bytes());
        output.extend(&lenu16.to_be_bytes());
        output.extend(&searchRange.to_be_bytes());
        output.extend(&max_pow2.to_be_bytes());
        output.extend(&(lenu16 * 16 - searchRange).to_be_bytes());
        let mut pos = 16 * self.tables.len() + 12;
        let mut head_pos: Option<usize> = None;
        for (tag, value) in self.tables.iter() {
            let mut bytes = otspec::ser::to_bytes(&value).unwrap();
            if tag == b"head" {
                head_pos = Some(pos);
                bytes[8] = 0;
                bytes[9] = 0;
                bytes[10] = 0;
                bytes[11] = 0;
            }
            let orig_len = bytes.len();
            let orig_checksum = checksum(&bytes);
            while (bytes.len() % 4) != 0 {
                bytes.push(0);
            }
            output.extend(tag);
            output.extend(&(orig_checksum as u32).to_be_bytes());
            output.extend(&(pos as u32).to_be_bytes());
            output.extend(&(orig_len as u32).to_be_bytes());
            pos += bytes.len();
            output_tables.extend(bytes);
        }
        output.extend(output_tables);
        // Compute full checksum and update head here.
        let full_checksum = 0xB1B0AFBA - checksum(&output);
        let checksum_be = full_checksum.to_be_bytes();
        if let Some(head_pos) = head_pos {
            output[head_pos + 8] = checksum_be[0];
            output[head_pos + 9] = checksum_be[1];
            output[head_pos + 10] = checksum_be[2];
            output[head_pos + 11] = checksum_be[3];
        }
        serializer.serialize_bytes(&output)
    }
}

struct FontVisitor {
    _phantom: std::marker::PhantomData<Font>,
}

impl FontVisitor {
    fn new() -> Self {
        FontVisitor {
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'de> Visitor<'de> for FontVisitor {
    type Value = Font;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "A sequence of values")
    }

    fn visit_seq<A: SeqAccess<'de>>(mut self, mut seq: A) -> Result<Self::Value, A::Error> {
        let header = seq
            .next_element::<TableHeader>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a table header"))?;
        let version = TryInto::<SfntVersion>::try_into(header.sfntVersion)
            .map_err(|_| serde::de::Error::custom("Font must begin with a valid version"))?;

        let mut result = Font::new(version);
        let mut table_records = Vec::with_capacity(header.numTables as usize);
        for i in 0..(header.numTables as usize) {
            let next = seq
                .next_element::<TableRecord>()?
                .ok_or_else(|| serde::de::Error::invalid_length(i, &self))?;
            table_records.push(next)
        }
        /* This is not strictly correct. */
        table_records.sort_by_key(|f| f.offset); /* Really very not correct */
        let mut pos = (16 * table_records.len() + 12) as u32;
        for tr in table_records {
            while pos < tr.offset {
                seq.next_element::<u8>()?.ok_or_else(|| {
                    serde::de::Error::custom(format!(
                        "Could not find {:?} table",
                        String::from_utf8(tr.tag.to_vec())
                    ))
                })?;
                pos += 1;
            }
            let table =
                match &tr.tag {
                    b"hhea" => Table::Hhea(seq.next_element::<hhea>()?.ok_or_else(|| {
                        serde::de::Error::custom("Could not deserialize hhea table")
                    })?),
                    b"head" => Table::Head(seq.next_element::<head>()?.ok_or_else(|| {
                        serde::de::Error::custom("Could not deserialize head table")
                    })?),
                    b"maxp" => Table::Maxp(seq.next_element::<maxp>()?.ok_or_else(|| {
                        serde::de::Error::custom("Could not deserialize maxp table")
                    })?),
                    _ => Table::Unknown(
                        (0..tr.length)
                            .filter_map(|_| seq.next_element::<u8>().unwrap())
                            .collect(),
                    ),
                };
            result.tables.insert(tr.tag, table);
            pos += tr.length;
        }
        Ok(result)
    }
}

impl<'de> Deserialize<'de> for Font {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_seq(FontVisitor::new())
    }
}

#[cfg(test)]
mod tests {

    use crate::font;
    use crate::head::head;
    use crate::hhea::hhea;
    use crate::maxp::maxp;

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
            checksumAdjustment: 0x2da80ff7,
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
            0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x20, 0x00, 0x01, 0x00, 0x10, 0x68, 0x65,
            0x61, 0x64, 0x18, 0x62, 0x27, 0x9f, 0x00, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x00, 0x36,
            0x68, 0x68, 0x65, 0x61, 0x06, 0x23, 0x07, 0x4b, 0x00, 0x00, 0x00, 0x74, 0x00, 0x00,
            0x00, 0x24, 0x6d, 0x61, 0x78, 0x70, 0x04, 0x65, 0x00, 0x64, 0x00, 0x00, 0x00, 0x98,
            0x00, 0x00, 0x00, 0x20, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x2d, 0xa8,
            0x0f, 0xf7, 0x5f, 0x0f, 0x3c, 0xf5, 0x00, 0x03, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x00,
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
        assert_eq!(serialized, binary_font);
        let deserialized: font::Font = otspec::de::from_bytes(&binary_font).unwrap();
        assert_eq!(deserialized, font);
    }

    #[test]
    fn test_load() {
        let f = font::load("data/test1.ttf").unwrap();
        assert_eq!(f.tables.len(), 3);
        f.save("data/test2.ttf");
    }
}
