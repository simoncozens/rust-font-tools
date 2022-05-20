use crate::tables;
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};
use otspec_macros::{Deserialize, Serialize};

use std::cmp;
use std::convert::{TryFrom, TryInto};
use std::error::Error;
use std::io::Read;
use std::num::Wrapping;
use std::path::Path;

/// Magic number used to identify the font type
#[derive(Copy, Clone, Debug, PartialEq)]
pub enum SfntVersion {
    /// TrueType (generally containing glyf outlines)
    TrueType = 0x00010000,
    /// OpenType (generally containing CFF outlines)
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

/// Low-level structure used for serializing/deserializing entries in the table directory
#[derive(Serialize, Deserialize, Debug)]
struct TableRecord {
    tag: Tag,
    checksum: uint32,
    offset: uint32,
    length: uint32,
}
/// The header of the font's table directory
#[derive(Deserialize)]
#[allow(non_snake_case)]
struct TableHeader {
    sfntVersion: u32,
    numTables: u16,
    _searchRange: u16,
    _entrySelector: u16,
    _rangeShift: u16,
}

/// An OpenType font object
#[derive(Debug, PartialEq)]
#[allow(non_snake_case)]
pub struct Font {
    /// Font version (TrueType/OpenType)
    sfntVersion: SfntVersion,
    /// Dictionary of tables in the font
    pub tables: super::table_store::TableSet,
    _numGlyphs: Option<u16>,
}

impl Font {
    /// Attempt to load a font from disk.
    pub fn load(path: impl AsRef<Path>) -> Result<Self, Box<dyn Error>> {
        let bytes = std::fs::read(path.as_ref())?;
        Self::from_bytes(&bytes)
    }

    /// Attempt to load a font from a raw byte slice.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error>> {
        otspec::de::from_bytes(bytes).map_err(|e| e.into())
    }

    /// Attempt to load a font from any reader.
    pub fn from_reader(mut reader: impl std::io::Read) -> Result<Self, Box<dyn Error>> {
        let mut buf = Vec::new();
        let _ = reader.read_to_end(&mut buf)?;
        Self::from_bytes(&buf)
    }

    /// Create a new font, empty of a given version (TrueType/OpenType)
    pub fn new(sfnt_version: SfntVersion) -> Self {
        Self {
            sfntVersion: sfnt_version,
            tables: Default::default(),
            _numGlyphs: None,
        }
    }

    //FIXME: do we want to keep this? do we want top-level methods generally?
    /// Returns `true` if the font contains a table with this `Tag`.
    pub fn contains_table(&self, tag: Tag) -> bool {
        self.tables.contains(&tag)
    }

    /// Deserializes all tables in the font.
    ///
    /// This is done in the correct order (as some tables can only be deserialized
    /// after certain others have been processed), so is a helpful way of getting
    /// the font into a useful state before working on it.
    pub fn fully_deserialize(&self) {
        self.tables.fully_deserialize().unwrap()
    }

    /// Attempt to save the font to the provided path.
    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<(), Box<dyn Error>> {
        let file = std::fs::File::create(path)?;
        self.write(file)
    }

    /// Attempt to write the font into the provided [`Writer`][std::io::Write];
    pub fn write(&mut self, mut writer: impl std::io::Write) -> Result<(), Box<dyn Error>> {
        self.tables.compile_glyf_loca_maxp();
        self.tables.compile_gsub_gpos();
        let mut bytes = Vec::new();
        self.to_bytes(&mut bytes)?;
        writer.write_all(&bytes).map_err(Into::into)
    }

    /// Total number of glyphs in the font, from the maxp table.
    ///
    /// Deserializes the maxp table if this is not already done.
    pub fn num_glyphs(&mut self) -> u16 {
        if self._numGlyphs.is_none() {
            let maxp = self
                .tables
                .maxp()
                .expect("Error deserializing maxp")
                .expect("No maxp?");
            self._numGlyphs = Some(maxp.num_glyphs())
        }
        self._numGlyphs.unwrap()
    }
}

/// Loads a binary font from the given filehandle.
#[deprecated(since = "0.1.0", note = "use Font::load instead")]
pub fn load<T>(mut file: T) -> Result<Font, Box<dyn Error>>
where
    T: Read,
{
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    otspec::de::from_bytes(&buffer).map_err(|e| e.into())
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

/// Returns B-tree search range parameters.
///
/// Various OpenType tables (the font table header, `cmap` format 4 subtables)
/// contain fields which are intended to be used to optimize B-tree search
/// algorithms. These are generally not used by implementations in practice, as
/// trusting user-supplied data to determine the algorithm's activity is unwise.
/// However, we still want to correctly generate these values to produce
/// specification-compliant fonts.
pub fn get_search_range(n: u16, itemsize: u16) -> (u16, u16, u16) {
    let mut max_pow2: u16 = 0;
    while 1u16 << (max_pow2 + 1) <= n {
        max_pow2 += 1;
    }
    let search_range = (1 << max_pow2) * itemsize;
    let range_shift = cmp::max(search_range, n * itemsize) - search_range;
    (search_range, max_pow2, range_shift)
}

impl Serialize for Font {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let lenu16: u16 = self.tables.len().try_into().unwrap();
        let (search_range, max_pow2, range_shift) = get_search_range(lenu16, 16);

        let mut output: Vec<u8> = vec![];
        let mut output_tables: Vec<u8> = vec![];
        let mut temp = Vec::new();

        output.extend(&(self.sfntVersion as u32).to_be_bytes());
        output.extend(&lenu16.to_be_bytes());
        output.extend(&search_range.to_be_bytes());
        output.extend(&max_pow2.to_be_bytes());
        output.extend(&range_shift.to_be_bytes());
        let mut pos = 16 * self.tables.len() + 12;
        let mut head_pos: Option<usize> = None;
        for tag in self.tables.keys() {
            temp.clear();
            self.tables.write_table(tag, &mut temp)?;
            if tag == tables::head::TAG {
                head_pos = Some(pos);
                temp[8..12].fill(0);
            }
            let orig_len = temp.len();
            let orig_checksum = checksum(&temp);
            while (temp.len() % 4) != 0 {
                temp.push(0);
            }
            output.extend(tag.as_bytes());
            output.extend(&(orig_checksum as u32).to_be_bytes());
            output.extend(&(pos as u32).to_be_bytes());
            output.extend(&(orig_len as u32).to_be_bytes());
            pos += temp.len();
            output_tables.extend_from_slice(&temp);
        }
        output.extend(output_tables);
        // Compute full checksum and update head here.
        let full_checksum = (Wrapping(0xB1B0AFBA) - Wrapping(checksum(&output))).0;
        if let Some(head_pos) = head_pos {
            let start = head_pos + 8;
            output[start..start + 4].copy_from_slice(&full_checksum.to_be_bytes());
        }
        data.put(output)
    }
}

impl Deserialize for Font {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let header: TableHeader = c.de()?;
        let version = TryInto::<SfntVersion>::try_into(header.sfntVersion).map_err(|_| {
            DeserializationError("Font must begin with a valid version".to_string())
        })?;

        let mut raw_tables = crate::table_store::TableLoader::default();
        let mut table_records = Vec::with_capacity(header.numTables as usize);
        //TODO: is this allocation + sorting necessary? can't we just deserialize
        //each table directly as we encounter the header?

        for _ in 0..(header.numTables as usize) {
            let next: TableRecord = c.de()?;
            table_records.push(next)
        }
        table_records.sort_by_key(|tr| tr.offset);
        for tr in table_records {
            let start = tr.offset as usize;
            let this_table = &c.input[start..start + tr.length as usize];
            raw_tables.add(tr.tag, this_table.into());
        }
        Ok(Font {
            sfntVersion: version,
            tables: raw_tables.finish()?,
            _numGlyphs: None,
        })
    }
}

#[cfg(test)]
mod tests {

    use super::*;
    use crate::tables::head::head;
    use crate::tables::hhea::hhea;
    use crate::tables::maxp;
    use otspec::ser;
    use otspec::types::U16F16;

    #[test]
    fn test_checksum() {
        let binary_hhea = vec![
            0x00, 0x01, 0x00, 0x00, 0x02, 0xc1, 0xff, 0x4c, 0x00, 0x00, 0x05, 0x1f, 0xfe, 0x82,
            0xfe, 0x82, 0x04, 0xdd, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x5d,
        ];
        assert_eq!(checksum(&binary_hhea), 0x0623074B)
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
        let mut font = Font::new(SfntVersion::TrueType);
        font.tables.insert(fhead);
        font.tables.insert(fhhea);
        font.tables.insert(fmaxp);

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
        let deserialized: Font = otspec::de::from_bytes(&binary_font).unwrap();
        deserialized.fully_deserialize();
        pretty_assertions::assert_eq!(deserialized, font);
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
        let deserialized: Font = otspec::de::from_bytes(&binary_font).unwrap();
        let head = deserialized.tables.head().unwrap().unwrap();
        assert_eq!(head.indexToLocFormat, 0);
        let floca = deserialized.tables.loca().unwrap().unwrap();
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
