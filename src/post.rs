use otspec::de::CountedDeserializer;
use otspec::{deserialize_visitor, read_field, read_field_counted};
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};

const APPLE_NAMES: &[&str] = &[
    ".notdef",
    ".null",
    "nonmarkingreturn",
    "space",
    "exclam",
    "quotedbl",
    "numbersign",
    "dollar",
    "percent",
    "ampersand",
    "quotesingle",
    "parenleft",
    "parenright",
    "asterisk",
    "plus",
    "comma",
    "hyphen",
    "period",
    "slash",
    "zero",
    "one",
    "two",
    "three",
    "four",
    "five",
    "six",
    "seven",
    "eight",
    "nine",
    "colon",
    "semicolon",
    "less",
    "equal",
    "greater",
    "question",
    "at",
    "A",
    "B",
    "C",
    "D",
    "E",
    "F",
    "G",
    "H",
    "I",
    "J",
    "K",
    "L",
    "M",
    "N",
    "O",
    "P",
    "Q",
    "R",
    "S",
    "T",
    "U",
    "V",
    "W",
    "X",
    "Y",
    "Z",
    "bracketleft",
    "backslash",
    "bracketright",
    "asciicircum",
    "underscore",
    "grave",
    "a",
    "b",
    "c",
    "d",
    "e",
    "f",
    "g",
    "h",
    "i",
    "j",
    "k",
    "l",
    "m",
    "n",
    "o",
    "p",
    "q",
    "r",
    "s",
    "t",
    "u",
    "v",
    "w",
    "x",
    "y",
    "z",
    "braceleft",
    "bar",
    "braceright",
    "asciitilde",
    "Adieresis",
    "Aring",
    "Ccedilla",
    "Eacute",
    "Ntilde",
    "Odieresis",
    "Udieresis",
    "aacute",
    "agrave",
    "acircumflex",
    "adieresis",
    "atilde",
    "aring",
    "ccedilla",
    "eacute",
    "egrave",
    "ecircumflex",
    "edieresis",
    "iacute",
    "igrave",
    "icircumflex",
    "idieresis",
    "ntilde",
    "oacute",
    "ograve",
    "ocircumflex",
    "odieresis",
    "otilde",
    "uacute",
    "ugrave",
    "ucircumflex",
    "udieresis",
    "dagger",
    "degree",
    "cent",
    "sterling",
    "section",
    "bullet",
    "paragraph",
    "germandbls",
    "registered",
    "copyright",
    "trademark",
    "acute",
    "dieresis",
    "notequal",
    "AE",
    "Oslash",
    "infinity",
    "plusminus",
    "lessequal",
    "greaterequal",
    "yen",
    "mu",
    "partialdiff",
    "summation",
    "product",
    "pi",
    "integral",
    "ordfeminine",
    "ordmasculine",
    "Omega",
    "ae",
    "oslash",
    "questiondown",
    "exclamdown",
    "logicalnot",
    "radical",
    "florin",
    "approxequal",
    "Delta",
    "guillemotleft",
    "guillemotright",
    "ellipsis",
    "nonbreakingspace",
    "Agrave",
    "Atilde",
    "Otilde",
    "OE",
    "oe",
    "endash",
    "emdash",
    "quotedblleft",
    "quotedblright",
    "quoteleft",
    "quoteright",
    "divide",
    "lozenge",
    "ydieresis",
    "Ydieresis",
    "fraction",
    "currency",
    "guilsinglleft",
    "guilsinglright",
    "fi",
    "fl",
    "daggerdbl",
    "periodcentered",
    "quotesinglbase",
    "quotedblbase",
    "perthousand",
    "Acircumflex",
    "Ecircumflex",
    "Aacute",
    "Edieresis",
    "Egrave",
    "Iacute",
    "Icircumflex",
    "Idieresis",
    "Igrave",
    "Oacute",
    "Ocircumflex",
    "apple",
    "Ograve",
    "Uacute",
    "Ucircumflex",
    "Ugrave",
    "dotlessi",
    "circumflex",
    "tilde",
    "macron",
    "breve",
    "dotaccent",
    "ring",
    "cedilla",
    "hungarumlaut",
    "ogonek",
    "caron",
    "Lslash",
    "lslash",
    "Scaron",
    "scaron",
    "Zcaron",
    "zcaron",
    "brokenbar",
    "Eth",
    "eth",
    "Yacute",
    "yacute",
    "Thorn",
    "thorn",
    "minus",
    "multiply",
    "onesuperior",
    "twosuperior",
    "threesuperior",
    "onehalf",
    "onequarter",
    "threequarters",
    "franc",
    "Gbreve",
    "gbreve",
    "Idotaccent",
    "Scedilla",
    "scedilla",
    "Cacute",
    "cacute",
    "Ccaron",
    "ccaron",
    "dcroat",
];

extern crate otspec;
use otspec::types::*;
use otspec_macros::tables;

tables!( postcore {
    Version16Dot16  version
    Fixed   italicAngle
    FWORD   underlinePosition
    FWORD   underlineThickness
    uint32  isFixedPitch
    uint32  minMemType42
    uint32  maxMemType42
    uint32  minMemType1
    uint32  maxMemType1
});

#[derive(Debug, PartialEq)]
pub struct post {
    pub version: U16F16,
    pub italicAngle: f32,
    pub underlinePosition: FWORD,
    pub underlineThickness: FWORD,
    pub isFixedPitch: uint32,
    pub minMemType42: uint32,
    pub maxMemType42: uint32,
    pub minMemType1: uint32,
    pub maxMemType1: uint32,
    pub glyphnames: Option<Vec<String>>,
}

impl post {
    pub fn set_version(&mut self, version: f32) {
        self.version = U16F16::from_num(version);
    }
}
impl Serialize for post {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let core = postcore {
            version: self.version,
            italicAngle: self.italicAngle,
            underlinePosition: self.underlinePosition,
            underlineThickness: self.underlineThickness,
            isFixedPitch: self.isFixedPitch,
            minMemType42: self.minMemType42,
            maxMemType42: self.maxMemType42,
            minMemType1: self.minMemType1,
            maxMemType1: self.maxMemType1,
        };
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&core)?;
        let mut glyph_name_table: Vec<u8> = Vec::new();
        let mut glyph_name_table_items = 0;
        if core.version == U16F16::from_num(2.0) {
            if let Some(v) = &self.glyphnames {
                seq.serialize_element(&(v.len() as u16))?;
                for name in v {
                    match APPLE_NAMES.iter().position(|&r| r == name) {
                        Some(index) => {
                            seq.serialize_element(&(index as u16))?;
                        }
                        None => {
                            seq.serialize_element(&((258 + glyph_name_table_items) as u16))?;
                            glyph_name_table.push(name.len() as u8);
                            glyph_name_table.extend(name.as_bytes());
                            glyph_name_table_items += 1;
                        }
                    }
                }
            }
            seq.serialize_element(&glyph_name_table)?;
        }
        seq.end()
    }
}
deserialize_visitor!(
    post,
    PostVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let core = read_field!(seq, postcore, "a post table");
        let mut glyphnames = None;
        if core.version == U16F16::from_num(2.0) {
            let numGlyphs = read_field!(seq, uint16, "a number of glyphs");
            let glyph_offsets: Vec<u16> = read_field_counted!(seq, numGlyphs, "glyph offsets");
            let mut glyphnames_vec = Vec::with_capacity(numGlyphs as usize);
            let mut glyph_name_table: Vec<String> = Vec::new();
            loop {
                let byte_count = seq.next_element::<u8>();
                if byte_count.is_err() {
                    break;
                }
                let byte_count = byte_count.unwrap();
                if byte_count.is_none() {
                    break;
                }
                let byte_count = byte_count.unwrap() as usize;
                let name: Vec<u8> = read_field_counted!(seq, byte_count, "glyph name");
                glyph_name_table.push(String::from_utf8(name).unwrap());
            }
            for i in 0..numGlyphs {
                let offset = glyph_offsets[i as usize] as usize;
                if offset < 258 {
                    glyphnames_vec.push(String::from(APPLE_NAMES[offset]));
                } else {
                    glyphnames_vec.push(glyph_name_table[offset - 258].clone());
                }
            }
            glyphnames = Some(glyphnames_vec);
        }
        Ok(post {
            version: core.version,
            italicAngle: core.italicAngle,
            underlinePosition: core.underlinePosition,
            underlineThickness: core.underlineThickness,
            isFixedPitch: core.isFixedPitch,
            minMemType42: core.minMemType42,
            maxMemType42: core.maxMemType42,
            minMemType1: core.minMemType1,
            maxMemType1: core.maxMemType1,
            glyphnames,
        })
    }
);

#[cfg(test)]
mod tests {
    use crate::post;
    use assert_approx_eq::assert_approx_eq;

    use otspec::ser;
    use otspec::types::U16F16;

    #[test]
    fn post_serde_v20() {
        let binary_post = vec![
            0x00, 0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0x9c, 0x00, 0x32, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x2a, 0x00, 0x00, 0x00, 0x03, 0x01, 0x02, 0x01, 0x03,
            0x01, 0x04, 0x01, 0x05, 0x01, 0x06, 0x01, 0x07, 0x01, 0x08, 0x01, 0x09, 0x01, 0x0a,
            0x01, 0x0b, 0x01, 0x0c, 0x01, 0x0d, 0x01, 0x0e, 0x01, 0x0f, 0x01, 0x10, 0x01, 0x11,
            0x01, 0x12, 0x01, 0x13, 0x01, 0x14, 0x01, 0x15, 0x01, 0x16, 0x01, 0x17, 0x01, 0x18,
            0x01, 0x19, 0x01, 0x1a, 0x01, 0x1b, 0x01, 0x1c, 0x01, 0x1d, 0x01, 0x1e, 0x01, 0x1f,
            0x01, 0x20, 0x01, 0x21, 0x01, 0x22, 0x01, 0x23, 0x01, 0x24, 0x01, 0x25, 0x01, 0x26,
            0x01, 0x27, 0x01, 0x28, 0x01, 0x29, 0x07, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x32, 0x37,
            0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x32, 0x37, 0x2e, 0x66, 0x69, 0x6e, 0x61, 0x07,
            0x75, 0x6e, 0x69, 0x30, 0x36, 0x36, 0x45, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x36,
            0x45, 0x2e, 0x66, 0x69, 0x6e, 0x61, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x36, 0x45,
            0x2e, 0x6d, 0x65, 0x64, 0x69, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x36, 0x45, 0x2e,
            0x69, 0x6e, 0x69, 0x74, 0x07, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x32, 0x38, 0x0c, 0x75,
            0x6e, 0x69, 0x30, 0x36, 0x32, 0x38, 0x2e, 0x66, 0x69, 0x6e, 0x61, 0x0c, 0x75, 0x6e,
            0x69, 0x30, 0x36, 0x32, 0x38, 0x2e, 0x6d, 0x65, 0x64, 0x69, 0x0c, 0x75, 0x6e, 0x69,
            0x30, 0x36, 0x32, 0x38, 0x2e, 0x69, 0x6e, 0x69, 0x74, 0x07, 0x75, 0x6e, 0x69, 0x30,
            0x36, 0x32, 0x41, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x32, 0x41, 0x2e, 0x66, 0x69,
            0x6e, 0x61, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x32, 0x41, 0x2e, 0x6d, 0x65, 0x64,
            0x69, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x32, 0x41, 0x2e, 0x69, 0x6e, 0x69, 0x74,
            0x07, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x32, 0x42, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36,
            0x32, 0x42, 0x2e, 0x66, 0x69, 0x6e, 0x61, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x32,
            0x42, 0x2e, 0x6d, 0x65, 0x64, 0x69, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x32, 0x42,
            0x2e, 0x69, 0x6e, 0x69, 0x74, 0x07, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x34, 0x34, 0x0c,
            0x75, 0x6e, 0x69, 0x30, 0x36, 0x34, 0x34, 0x2e, 0x66, 0x69, 0x6e, 0x61, 0x0c, 0x75,
            0x6e, 0x69, 0x30, 0x36, 0x34, 0x34, 0x2e, 0x6d, 0x65, 0x64, 0x69, 0x0c, 0x75, 0x6e,
            0x69, 0x30, 0x36, 0x34, 0x34, 0x2e, 0x69, 0x6e, 0x69, 0x74, 0x07, 0x75, 0x6e, 0x69,
            0x30, 0x36, 0x34, 0x36, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x34, 0x36, 0x2e, 0x66,
            0x69, 0x6e, 0x61, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x34, 0x36, 0x2e, 0x6d, 0x65,
            0x64, 0x69, 0x0c, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x34, 0x36, 0x2e, 0x69, 0x6e, 0x69,
            0x74, 0x07, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x42, 0x41, 0x0c, 0x75, 0x6e, 0x69, 0x30,
            0x36, 0x42, 0x41, 0x2e, 0x66, 0x69, 0x6e, 0x61, 0x0b, 0x64, 0x6f, 0x74, 0x61, 0x62,
            0x6f, 0x76, 0x65, 0x2d, 0x61, 0x72, 0x0b, 0x64, 0x6f, 0x74, 0x62, 0x65, 0x6c, 0x6f,
            0x77, 0x2d, 0x61, 0x72, 0x0c, 0x64, 0x6f, 0x74, 0x63, 0x65, 0x6e, 0x74, 0x65, 0x72,
            0x2d, 0x61, 0x72, 0x17, 0x74, 0x77, 0x6f, 0x64, 0x6f, 0x74, 0x73, 0x76, 0x65, 0x72,
            0x74, 0x69, 0x63, 0x61, 0x6c, 0x61, 0x62, 0x6f, 0x76, 0x65, 0x2d, 0x61, 0x72, 0x17,
            0x74, 0x77, 0x6f, 0x64, 0x6f, 0x74, 0x73, 0x76, 0x65, 0x72, 0x74, 0x69, 0x63, 0x61,
            0x6c, 0x62, 0x65, 0x6c, 0x6f, 0x77, 0x2d, 0x61, 0x72, 0x19, 0x74, 0x77, 0x6f, 0x64,
            0x6f, 0x74, 0x73, 0x68, 0x6f, 0x72, 0x69, 0x7a, 0x6f, 0x6e, 0x74, 0x61, 0x6c, 0x61,
            0x62, 0x6f, 0x76, 0x65, 0x2d, 0x61, 0x72, 0x19, 0x74, 0x77, 0x6f, 0x64, 0x6f, 0x74,
            0x73, 0x68, 0x6f, 0x72, 0x69, 0x7a, 0x6f, 0x6e, 0x74, 0x61, 0x6c, 0x62, 0x65, 0x6c,
            0x6f, 0x77, 0x2d, 0x61, 0x72, 0x15, 0x74, 0x68, 0x72, 0x65, 0x65, 0x64, 0x6f, 0x74,
            0x73, 0x64, 0x6f, 0x77, 0x6e, 0x61, 0x62, 0x6f, 0x76, 0x65, 0x2d, 0x61, 0x72, 0x15,
            0x74, 0x68, 0x72, 0x65, 0x65, 0x64, 0x6f, 0x74, 0x73, 0x64, 0x6f, 0x77, 0x6e, 0x62,
            0x65, 0x6c, 0x6f, 0x77, 0x2d, 0x61, 0x72, 0x13, 0x74, 0x68, 0x72, 0x65, 0x65, 0x64,
            0x6f, 0x74, 0x73, 0x75, 0x70, 0x61, 0x62, 0x6f, 0x76, 0x65, 0x2d, 0x61, 0x72, 0x13,
            0x74, 0x68, 0x72, 0x65, 0x65, 0x64, 0x6f, 0x74, 0x73, 0x75, 0x70, 0x62, 0x65, 0x6c,
            0x6f, 0x77, 0x2d, 0x61, 0x72, 0x07, 0x75, 0x6e, 0x69, 0x30, 0x36, 0x34, 0x45,
        ];
        let deserialized: post::post = otspec::de::from_bytes(&binary_post).unwrap();
        assert_eq!(deserialized.version, U16F16::from_num(2.0));
        assert_approx_eq!(deserialized.italicAngle, 0.0);
        assert_eq!(deserialized.underlinePosition, -100);
        assert_eq!(deserialized.underlineThickness, 50);
        assert_eq!(deserialized.isFixedPitch, 0);
        assert!(deserialized.glyphnames.is_some());
        if let Some(ref names) = deserialized.glyphnames {
            assert_eq!(names.len(), 42);
            assert_eq!(names[0], String::from(".notdef"));
            assert_eq!(names[1], String::from("space"));
            assert_eq!(names[2], String::from("uni0627"));
        }
        let serialized = ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_post);
    }
}
