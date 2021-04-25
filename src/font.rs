use otspec::error::Error as OTSpecError;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::{Deserialize, Deserializer};
use serde::{Serialize, Serializer};
use std::convert::{TryFrom, TryInto};
use std::num::Wrapping;
extern crate otspec;
use crate::avar::avar;
use crate::cmap::cmap;
use crate::fvar::fvar;
use crate::gasp::gasp;
use crate::glyf;
use crate::gvar::gvar;
use crate::head::head;
use crate::hhea::hhea;
use crate::hmtx;
use crate::loca;
use crate::maxp::maxp;
use crate::post::post;
use indexmap::IndexMap;
use otspec::types::*;
use otspec::{deserialize_visitor, read_field};
use std::cmp;
use std::fs::File;
use std::io::Write;

#[derive(Debug, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Table {
    Unknown(Vec<u8>),
    Avar(avar),
    Cmap(cmap),
    Fvar(fvar),
    Gasp(gasp),
    Glyf(glyf::glyf),
    Head(head),
    Hhea(hhea),
    Hmtx(hmtx::hmtx),
    Loca(loca::loca),
    Maxp(maxp),
    Post(post),
    // Gvar(gvar),
}

macro_rules! table_unchecked {
    ($name: ident, $enum:ident, $t: ty) => {
        pub fn $name(&self) -> &$t {
            if let Table::$enum(thing) = self {
                return thing;
            }
            panic!("Asked for a {:} but found a {:?}", stringify!($t), self)
        }
    };
}

impl Table {
    table_unchecked!(avar_unchecked, Avar, avar);
    table_unchecked!(cmap_unchecked, Cmap, cmap);
    table_unchecked!(fvar_unchecked, Fvar, fvar);
    table_unchecked!(gasp_unchecked, Gasp, gasp);
    table_unchecked!(glyf_unchecked, Glyf, glyf::glyf);
    table_unchecked!(head_unchecked, Head, head);
    table_unchecked!(hhea_unchecked, Hhea, hhea);
    table_unchecked!(hmtx_unchecked, Hmtx, hmtx::hmtx);
    table_unchecked!(loca_unchecked, Loca, loca::loca);
    table_unchecked!(maxp_unchecked, Maxp, maxp);
    table_unchecked!(post_unchecked, Post, post);
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
    pub tables: IndexMap<Tag, Table>,
    _numGlyphs: Option<u16>,
}

use otspec::ser;

impl Font {
    fn _locaIs32Bit(&self) -> Option<bool> {
        let head = self.get_table_simple(b"head")?;
        if self._table_needs_deserializing(head) {
            return None;
            // panic!("Deserialize head before loca!")
        }
        if let Table::Head(head) = head {
            return Some(head.indexToLocFormat == 1);
        }
        panic!("Can't happen - head not a head table?!")
    }

    fn _locaOffsets(&self) -> Option<Vec<Option<u32>>> {
        let loca = self.get_table_simple(b"loca")?;
        if self._table_needs_deserializing(loca) {
            return None;
            // panic!("Deserialize loca before glyf!")
        }
        if let Table::Loca(loca) = loca {
            return Some(loca.indices.clone()); // XXX
        }
        panic!("Can't happen - loca not a loca table?!")
    }

    fn _numberOfHMetrics(&self) -> Option<u16> {
        let hhea = self.get_table_simple(b"hhea")?;
        if self._table_needs_deserializing(hhea) {
            return None;
            // panic!("Deserialize loca before glyf!")
        }
        if let Table::Hhea(hhea) = hhea {
            return Some(hhea.numberOfHMetrics);
        }
        panic!("Can't happen - hhea not a hhea table?!")
    }

    fn _deserialize(&self, tag: &Tag, binary: &[u8]) -> otspec::error::Result<Table> {
        match tag {
            b"cmap" => Ok(Table::Cmap(otspec::de::from_bytes(binary)?)),
            b"head" => Ok(Table::Head(otspec::de::from_bytes(binary)?)),
            b"hhea" => Ok(Table::Hhea(otspec::de::from_bytes(binary)?)),
            b"fvar" => Ok(Table::Fvar(otspec::de::from_bytes(binary)?)),
            b"gasp" => Ok(Table::Gasp(otspec::de::from_bytes(binary)?)),
            b"maxp" => Ok(Table::Maxp(otspec::de::from_bytes(binary)?)),
            b"post" => Ok(Table::Post(otspec::de::from_bytes(binary)?)),
            b"hmtx" => {
                let numberOfHMetrics = self._numberOfHMetrics();
                if numberOfHMetrics.is_none() {
                    return Err(OTSpecError::DeserializedInWrongOrder);
                }
                Ok(Table::Hmtx(hmtx::from_bytes(
                    binary,
                    numberOfHMetrics.unwrap(),
                )?))
            }
            b"loca" => {
                let locaIs32bit = self._locaIs32Bit();
                if locaIs32bit.is_none() {
                    return Err(OTSpecError::DeserializedInWrongOrder);
                }
                Ok(Table::Loca(loca::from_bytes(binary, locaIs32bit.unwrap())?))
            }
            b"glyf" => {
                let locaOffsets = self._locaOffsets();
                if locaOffsets.is_none() {
                    return Err(OTSpecError::DeserializedInWrongOrder);
                }
                Ok(Table::Glyf(glyf::from_bytes(binary, locaOffsets.unwrap())?))
            }
            _ => Ok(Table::Unknown(binary.to_vec())),
        }
    }

    pub fn new(sfntVersion: SfntVersion) -> Self {
        Self {
            sfntVersion,
            tables: IndexMap::new(),
            _numGlyphs: None,
        }
    }

    fn _table_needs_deserializing(&self, table: &Table) -> bool {
        // Also check here for known tables we can't deserialize.
        if let Table::Unknown(_binary) = table {
            return true;
        }
        false
    }

    fn get_table_simple<'a>(&'a self, tag: &Tag) -> Option<&'a Table> {
        if !self.tables.contains_key(tag) {
            return None;
        }
        Some(self.tables.get(tag).unwrap())
    }

    fn get_table_mut_simple<'a>(&'a mut self, tag: &Tag) -> Option<&'a mut Table> {
        if !self.tables.contains_key(tag) {
            return None;
        }
        Some(self.tables.get_mut(tag).unwrap())
    }

    pub fn get_table<'a>(&'a mut self, tag: &Tag) -> otspec::error::Result<Option<&'a mut Table>> {
        let table = self.get_table_simple(tag);
        // println!("Getting table {:?}", tag);
        if table.is_none() {
            // println!("Not found");
            return Ok(None);
        }
        let table = table.unwrap();

        // println!("It was {:?}", table);
        if let Table::Unknown(binary) = table {
            // println!("Was binary, deserializing");
            let newtable = self._deserialize(tag, binary)?;
            // println!("Inserting new table {:?}", newtable);
            self.tables.insert(*tag, newtable);
        }
        Ok(self.get_table_mut_simple(tag))
    }

    pub fn fully_deserialize(&mut self) {
        let keys: Vec<Tag> = self.tables.keys().copied().collect();
        for t in keys {
            self.get_table(&t).unwrap();
        }
    }

    pub fn save<T>(&mut self, file: &mut T)
    where
        T: Write,
    {
        self.compile_glyf_loca_maxp();
        let serialized = ser::to_bytes(&self).unwrap();
        file.write_all(&serialized).unwrap();
    }

    pub fn num_glyphs(&mut self) -> u16 {
        if self._numGlyphs.is_none() {
            let maxp = self
                .get_table(b"maxp")
                .expect("Error deserializing maxp")
                .expect("No maxp?")
                .maxp_unchecked();
            self._numGlyphs = Some(maxp.num_glyphs())
        }
        self._numGlyphs.unwrap()
    }

    pub fn compile_glyf_loca_maxp(&mut self) {
        let mut glyf_output: Vec<u8> = vec![];
        let mut loca_indices: Vec<u32> = vec![];
        let mut locaIs32bit = false;
        let maybe_glyf = self.get_table(b"glyf").unwrap();
        if maybe_glyf.is_none() {
            println!("Warning: no glyf table");
            return;
        }
        let glyf = maybe_glyf.unwrap().glyf_unchecked();
        let glyf_count = glyf.glyphs.len();
        for g in &glyf.glyphs {
            let cur_len: u32 = glyf_output.len().try_into().unwrap();
            if cur_len * 2 > (u16::MAX as u32) {
                locaIs32bit = true;
            }
            loca_indices.push(cur_len);
            if g.is_none() {
                continue;
            }
            let glyph = g.as_ref().unwrap();
            glyf_output.extend(otspec::ser::to_bytes(&glyph).unwrap());
            // Add multiple-of-four padding
            while glyf_output.len() % 4 != 0 {
                glyf_output.push(0);
            }
        }
        loca_indices.push(glyf_output.len().try_into().unwrap());

        let maxp_table = self.get_table(b"maxp").unwrap().unwrap();
        if let Table::Maxp(maxp) = maxp_table {
            maxp.set_num_glyphs(glyf_count as u16);
        }

        let head_table = self.get_table(b"head").unwrap().unwrap();
        if let Table::Head(head) = head_table {
            head.indexToLocFormat = if locaIs32bit { 1 } else { 0 };
        }

        self.tables.insert(*b"glyf", Table::Unknown(glyf_output));
        let loca_output: Vec<u8>;
        if locaIs32bit {
            loca_output = otspec::ser::to_bytes(&loca_indices).unwrap();
        } else {
            let converted: Vec<u16> = loca_indices.iter().map(|x| (*x / 2_u32) as u16).collect();
            loca_output = otspec::ser::to_bytes(&converted).unwrap();
        }
        self.tables.insert(*b"loca", Table::Unknown(loca_output));
    }
}

use std::error::Error;
use std::io::Read;

pub fn load<T>(mut file: T) -> Result<Font, Box<dyn Error>>
where
    T: Read,
{
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    otspec::de::from_bytes(&buffer)
        .map_err(|e| e.into())
        .map(|mut f: Font| {
            let _ = f.get_table(b"head");
            let _ = f.get_table(b"loca");
            f
        })
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

pub fn get_search_range(n: u16, itemsize: u16) -> (u16, u16, u16) {
    let mut max_pow2: u16 = 0;
    while 1u16 << (max_pow2 + 1) <= n {
        max_pow2 += 1;
    }
    let search_range = (1 << max_pow2) * itemsize;
    let range_shift = cmp::max(0, n * itemsize - search_range);
    (search_range, max_pow2, range_shift)
}

impl Serialize for Font {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let lenu16: u16 = self.tables.len().try_into().unwrap();
        let (searchRange, max_pow2, range_shift) = get_search_range(lenu16, 16);
        // let mut seq = serializer.serialize_seq(None)?;
        let mut output: Vec<u8> = vec![];
        let mut output_tables: Vec<u8> = vec![];
        output.extend(&(self.sfntVersion as u32).to_be_bytes());
        output.extend(&lenu16.to_be_bytes());
        output.extend(&searchRange.to_be_bytes());
        output.extend(&max_pow2.to_be_bytes());
        output.extend(&range_shift.to_be_bytes());
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
        let full_checksum = (Wrapping(0xB1B0AFBA) - Wrapping(checksum(&output))).0;
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

deserialize_visitor!(
    Font,
    FontVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let header = read_field!(seq, TableHeader, "table header");
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
        let pos = (16 * table_records.len() + 12) as u32;
        let max_offset = table_records
            .iter()
            .map(|x| (x.length + x.offset))
            .max()
            .ok_or_else(|| serde::de::Error::custom("No tables?"))?;
        let remainder: Vec<u8> = (0..(max_offset - pos))
            .filter_map(|_| seq.next_element::<u8>().unwrap())
            .collect();
        table_records.sort_by_key(|tr| tr.offset);
        for tr in table_records {
            let start = (tr.offset - pos) as usize;
            let this_table = &remainder[start..start + tr.length as usize];
            let table = Table::Unknown(this_table.into()); // Deserialize on read
            result.tables.insert(tr.tag, table);
        }
        Ok(result)
    }
);

#[cfg(test)]
mod tests {

    use crate::font;
    use crate::head::head;
    use crate::hhea::hhea;
    use crate::maxp;
    use otspec::types::U16F16;

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
    fn font_serde() {
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
        let fmaxp = maxp::maxp {
            version: U16F16::from_num(1.0),
            table: maxp::MaxpVariant::Maxp10(maxp::maxp10 {
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
            }),
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
        let mut deserialized: font::Font = otspec::de::from_bytes(&binary_font).unwrap();
        deserialized.fully_deserialize();
        assert_eq!(deserialized, font);
    }

    #[test]
    fn test_de_loca() {
        let binary_font = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x40, 0x00, 0x02, 0x00, 0x00, 0x68, 0x65,
            0x61, 0x64, 0x18, 0x6b, 0x5d, 0xde, 0x00, 0x00, 0x00, 0x4c, 0x00, 0x00, 0x00, 0x36,
            0x68, 0x68, 0x65, 0x61, 0x06, 0x23, 0x07, 0x4b, 0x00, 0x00, 0x00, 0x84, 0x00, 0x00,
            0x00, 0x24, 0x6c, 0x6f, 0x63, 0x61, 0x00, 0x5e, 0x00, 0x4c, 0x00, 0x00, 0x00, 0xc8,
            0x00, 0x00, 0x00, 0x0e, 0x6d, 0x61, 0x78, 0x70, 0x04, 0x65, 0x00, 0x64, 0x00, 0x00,
            0x00, 0xa8, 0x00, 0x00, 0x00, 0x20, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0xc0, 0x68, 0x3e, 0x6a, 0x5f, 0x0f, 0x3c, 0xf5, 0x00, 0x03, 0x03, 0xe8, 0x00, 0x00,
            0x00, 0x00, 0xda, 0x56, 0x58, 0xaa, 0x00, 0x00, 0x00, 0x00, 0xdc, 0xa5, 0xc0, 0x69,
            0x00, 0x09, 0x00, 0x00, 0x02, 0x50, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x06, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x02, 0xc1, 0xff, 0x4c,
            0x00, 0x00, 0x05, 0x1f, 0xfe, 0x82, 0xfe, 0x82, 0x04, 0xdd, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x5d,
            0x00, 0x01, 0x00, 0x00, 0x04, 0x5d, 0x00, 0x62, 0x00, 0x07, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x15, 0x00, 0x15, 0x00, 0x15, 0x00, 0x15,
            0x00, 0x22, 0x00, 0x34, 0x00, 0x00,
        ];
        let mut deserialized: font::Font = otspec::de::from_bytes(&binary_font).unwrap();
        let head = deserialized.get_table(b"head").unwrap().unwrap();
        if let crate::font::Table::Head(head) = head {
            assert_eq!(head.indexToLocFormat, 0);
        }
        let floca = deserialized
            .get_table(b"loca")
            .unwrap()
            .unwrap()
            .loca_unchecked();
        assert_eq!(
            floca.indices,
            vec![Some(0), None, None, None, Some(42), Some(68)]
        )
    }

    // #[test]
    // fn test_load() {
    //     let f = font::load("data/test1.ttf").unwrap();
    //     assert_eq!(f.tables.len(), 11);
    //     f.save("data/test2.ttf");
    // }
}
