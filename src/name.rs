#![allow(non_camel_case_types, non_snake_case)]

use encoding::all::{
    BIG5_2003, GBK, MAC_CYRILLIC, MAC_ROMAN, UTF_16BE, WINDOWS_1252, WINDOWS_31J, WINDOWS_949,
};
use encoding::{DecoderTrap, EncoderTrap, EncodingRef};
use otspec::types::*;
use otspec::{deserialize_visitor, read_field, read_field_counted, read_remainder};
use otspec_macros::tables;
use serde::de::{SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::Serializer;
use serde::{Deserialize, Deserializer, Serialize};

fn get_encoding(platform_id: u16, encoding_id: u16) -> EncodingRef {
    if platform_id == 0 {
        return UTF_16BE;
    }
    if platform_id == 1 {
        if encoding_id == 7 {
            return MAC_CYRILLIC;
        } else {
            return MAC_ROMAN; // XXX NO THIS IS WRONG.
        }
    }
    if platform_id == 2 {
        match encoding_id {
            0 => return WINDOWS_1252,
            1 => return UTF_16BE,
            2 => return WINDOWS_1252,
            _ => unimplemented!(),
        };
    }
    if platform_id == 3 {
        match encoding_id {
            0 => return UTF_16BE,
            1 => return UTF_16BE,
            2 => return WINDOWS_31J,
            3 => return GBK,
            4 => return BIG5_2003,
            5 => return WINDOWS_949,
            6 => unimplemented!(),
            _ => return UTF_16BE,
        };
    }
    unimplemented!()
}

/// Descriptive names of the name table nameID entries
#[derive(Copy, Clone)]
pub enum NameRecordID {
    /// Copyright notice
    Copyright,
    /// Font Family name
    FontFamilyName,
    /// Font Subfamily name
    FontSubfamilyName,
    /// Unique font identifier
    UniqueID,
    /// Full font name that reflects all family and relevant subfamily descriptors
    FullFontName,
    /// Version string
    Version,
    /// PostScript name for the font
    PostscriptName,
    /// Trademark
    Trademark,
    /// Manufacturer Name
    Manufacturer,
    /// Designer
    Designer,
    /// Description
    Description,
    /// URL Vendor
    ManufacturerURL,
    /// URL Designer
    DesignerURL,
    /// License Description
    License,
    /// License Info URL
    LicenseURL,
    /// Reserved
    Reserved,
    /// Typographic Family name
    PreferredFamilyName,
    /// Typographic Subfamily name
    PreferredSubfamilyName,
    /// Compatible Full (Macintosh only)
    CompatibleFullName,
    /// Sample text
    SampleText,
    /// PostScript CID findfont name
    PostScriptCID,
    /// WWS Family Name
    WWSFamilyName,
    /// WWS Subfamily Name
    WWSSubfamilyName,
    /// Light Background Palette
    LightBackgroundPalette,
    /// Dark Background Palette
    DarkBackgroundPalette,
    /// Variations PostScript Name Prefix
    VariationsPostScriptNamePrefix,
}

impl From<NameRecordID> for u16 {
    fn from(namerecord: NameRecordID) -> u16 {
        namerecord as u16
    }
}

tables!(
    NameRecordInternal {
        uint16 platformID
        uint16 encodingID
        uint16 languageID
        uint16 nameID
        uint16 length
        uint16 stringOffset
    }
);

/// A single name record to be placed inside the name table
#[derive(Debug, PartialEq)]
pub struct NameRecord {
    /// Platform ID (0=Unicode, 1=Macintosh, 3=Windows)
    pub platformID: uint16,
    /// Identifier for encoding of string content. Platform-specific.
    pub encodingID: uint16,
    /// Identifier for language of string content. Platform-specific.
    pub languageID: uint16,
    /// The numeric identifier representing the type of data. See NameRecordID.
    pub nameID: uint16,
    /// The actual content
    pub string: String,
}

impl NameRecord {
    /// Create a new name record for the Windows platform in Unicode encoding
    /// (3,10,0x409)
    pub fn windows_unicode<T, U>(n: T, s: U) -> NameRecord
    where
        T: Into<u16>,
        U: Into<String>,
    {
        NameRecord {
            platformID: 3,
            encodingID: 10,
            languageID: 0x409,
            nameID: n.into(),
            string: s.into(),
        }
    }
}

/// Represents a font's name (Naming) table
#[derive(Debug, PartialEq)]
pub struct name {
    pub records: Vec<NameRecord>,
}

deserialize_visitor!(
    name,
    NameVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let _version = read_field!(seq, uint16, "a name table version");
        let count = read_field!(seq, uint16, "a count of name records");
        let _offset = read_field!(seq, uint16, "an offset");
        let internal_records: Vec<NameRecordInternal> =
            read_field_counted!(seq, count, "name records");
        let remainder = read_remainder!(seq, "a name table string pool");
        let mut records: Vec<NameRecord> = Vec::with_capacity(count.into());
        for ir in internal_records {
            let string_as_bytes =
                &remainder[ir.stringOffset as usize..(ir.stringOffset + ir.length) as usize];
            let encoding = get_encoding(ir.platformID, ir.encodingID);
            let string: String = encoding
                .decode(string_as_bytes, DecoderTrap::Replace)
                .unwrap();

            records.push(NameRecord {
                string,
                platformID: ir.platformID,
                encodingID: ir.encodingID,
                languageID: ir.languageID,
                nameID: ir.nameID,
            })
        }

        Ok(name { records })
    }
);

impl Serialize for name {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut string_pool: Vec<u8> = Vec::new();
        let mut seq = serializer.serialize_seq(None)?;
        let offset = 6 + 12 * self.records.len() as uint16;
        seq.serialize_element(&(0 as uint16))?;
        seq.serialize_element(&(self.records.len() as uint16))?;
        seq.serialize_element(&offset)?;
        for record in &self.records {
            let encoder = get_encoding(record.platformID, record.encodingID);
            let encoded = encoder
                .encode(&record.string, EncoderTrap::Replace)
                .unwrap();
            let nri = NameRecordInternal {
                platformID: record.platformID,
                encodingID: record.encodingID,
                languageID: record.languageID,
                nameID: record.nameID,
                length: encoded.len() as uint16,
                stringOffset: string_pool.len() as uint16,
            };
            seq.serialize_element(&nri)?;
            string_pool.extend(encoded);
        }
        seq.serialize_element(&string_pool)?;
        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use crate::name;
    use crate::name::NameRecord;
    #[test]
    fn name_serde() {
        let fname = name::name {
            records: vec![
                NameRecord {
                    platformID: 1,
                    encodingID: 0,
                    languageID: 0,
                    nameID: 17,
                    string: "Regular".to_string(),
                },
                NameRecord {
                    platformID: 1,
                    encodingID: 0,
                    languageID: 0,
                    nameID: 256,
                    string: "weight".to_string(),
                },
                NameRecord {
                    platformID: 1,
                    encodingID: 0,
                    languageID: 0,
                    nameID: 257,
                    string: "slant".to_string(),
                },
                NameRecord {
                    platformID: 3,
                    encodingID: 1,
                    nameID: 17,
                    languageID: 0x409,
                    string: "Regular".to_string(),
                },
                NameRecord {
                    platformID: 3,
                    encodingID: 1,
                    nameID: 256,
                    languageID: 0x409,
                    string: "weight".to_string(),
                },
                NameRecord {
                    platformID: 3,
                    encodingID: 1,
                    nameID: 257,
                    languageID: 0x409,
                    string: "slant".to_string(),
                },
            ],
        };
        let binary_name = vec![
            0x00, 0x00, 0x00, 0x06, 0x00, 0x4e, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x11,
            0x00, 0x07, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x06,
            0x00, 0x07, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x05, 0x00, 0x0d,
            0x00, 0x03, 0x00, 0x01, 0x04, 0x09, 0x00, 0x11, 0x00, 0x0e, 0x00, 0x12, 0x00, 0x03,
            0x00, 0x01, 0x04, 0x09, 0x01, 0x00, 0x00, 0x0c, 0x00, 0x20, 0x00, 0x03, 0x00, 0x01,
            0x04, 0x09, 0x01, 0x01, 0x00, 0x0a, 0x00, 0x2c, 0x52, 0x65, 0x67, 0x75, 0x6c, 0x61,
            0x72, 0x77, 0x65, 0x69, 0x67, 0x68, 0x74, 0x73, 0x6c, 0x61, 0x6e, 0x74, 0x00, 0x52,
            0x00, 0x65, 0x00, 0x67, 0x00, 0x75, 0x00, 0x6c, 0x00, 0x61, 0x00, 0x72, 0x00, 0x77,
            0x00, 0x65, 0x00, 0x69, 0x00, 0x67, 0x00, 0x68, 0x00, 0x74, 0x00, 0x73, 0x00, 0x6c,
            0x00, 0x61, 0x00, 0x6e, 0x00, 0x74,
        ];
        let deserialized: name::name = otspec::de::from_bytes(&binary_name).unwrap();
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(deserialized, fname);
        assert_eq!(serialized, binary_name);
    }
}
