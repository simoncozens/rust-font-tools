use itertools::izip;
use otspec::de::CountedDeserializer;
use serde::de::SeqAccess;
use serde::de::Visitor;

use serde::Deserializer;
use serde::{Deserialize, Serialize};

extern crate otspec;
use otspec::deserialize_visitor;
use otspec::types::*;
use otspec_macros::tables;
use std::collections::HashMap;

tables!(

EncodingRecord {
        uint16 platformID
        uint16 encodingID
        uint32 subtableOffset
}

CmapHeader {
    uint16  version
    Counted(EncodingRecord) encodingRecords
}
);

#[derive(Debug, PartialEq)]
struct cmap0 {
    format: uint16,
    length: uint16,
    language: uint16,
    glyphIdArray: Vec<u8>,
}

impl cmap0 {
    fn to_mapping(self) -> HashMap<uint16, uint16> {
        return HashMap::new();
    }
}

deserialize_visitor!(
    cmap0,
    Cmap0Visitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let format = seq
            .next_element::<uint16>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap0 table"))?;
        let length = seq
            .next_element::<uint16>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap0 table"))?;
        let language = seq
            .next_element::<uint16>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap0 table"))?;
        let glyphIdArray = seq
            .next_element_seed(CountedDeserializer::with_len(length as usize))?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap0 table"))?;
        Ok(cmap0 {
            format,
            length,
            language,
            glyphIdArray,
        })
    }
);

#[derive(Debug, PartialEq)]
struct cmap4 {
    format: uint16,
    length: uint16,
    language: uint16,
    segcount: uint16,
    endCode: Vec<uint16>,
    startCode: Vec<uint16>,
    idDelta: Vec<int16>,
    idRangeOffsets: Vec<uint16>,
    glyphIdArray: Vec<uint16>,
}

impl cmap4 {
    fn to_mapping(&self) -> HashMap<uint16, uint16> {
        let mut map = HashMap::new();
        for (start, end, delta, offset) in izip!(
            &self.startCode,
            &self.endCode,
            &self.idDelta,
            &self.idRangeOffsets
        ) {
            if *offset != 0 {
                unimplemented!()
            }
            if *end == 0xffff {
                break;
            }
            for i in *start..(1 + *end) {
                map.insert(i, (i as i16 + *delta) as u16);
            }
        }
        map
    }
}
deserialize_visitor!(
    cmap4,
    Cmap4Visitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let format = seq
            .next_element::<uint16>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap4 table format"))?;
        let length = seq
            .next_element::<uint16>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap4 table length"))?;
        let language = seq
            .next_element::<uint16>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap4 table language"))?;
        let segcount = seq
            .next_element::<uint16>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap4 table segcount"))?
            / 2;
        // Ignore next few fields
        let _ = seq.next_element::<uint16>()?;
        let _ = seq.next_element::<uint16>()?;
        let _ = seq.next_element::<uint16>()?;
        // println!("segment count {:?}", segcount);
        let endCode: Vec<uint16> = seq
            .next_element_seed(CountedDeserializer::with_len(segcount as usize))?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap4 table endcode"))?;
        // println!("endcode {:?}", endCode);
        let _ = seq.next_element::<uint16>()?; // reserved padding
        let startCode: Vec<uint16> = seq
            .next_element_seed(CountedDeserializer::with_len(segcount as usize))?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap4 table startcode"))?;
        // println!("startCode {:?}", startCode);
        let idDelta: Vec<int16> = seq
            .next_element_seed(CountedDeserializer::with_len(segcount as usize))?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap4 table idDelta"))?;
        // println!("idDelta {:?}", idDelta);
        let idRangeOffsets: Vec<uint16> = seq
            .next_element_seed(CountedDeserializer::with_len(segcount as usize))?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap4 table idRangeOffset"))?;
        // println!("Offsets {:?}", idRangeOffsets);
        let lenSoFar = 16 + (segcount * 2 * 4);
        // println!("Reading {:?} bytes", (length - lenSoFar));
        let glyphIdArray: Vec<u16> = seq
            .next_element_seed(CountedDeserializer::with_len((length - lenSoFar) as usize))?
            .unwrap_or_default();
        Ok(cmap4 {
            format,
            length,
            language,
            segcount,
            endCode,
            startCode,
            idDelta,
            idRangeOffsets,
            glyphIdArray,
        })
    }
);

#[derive(Debug, PartialEq)]
struct CmapSubtable {
    format: uint16,
    platformID: uint16,
    encodingID: uint16,
    languageID: uint16,
    mapping: HashMap<uint16, uint16>,
}

#[derive(Debug, PartialEq)]
struct cmap {
    subtables: Vec<CmapSubtable>,
}

deserialize_visitor!(
    cmap,
    CmapVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let core = seq
            .next_element::<CmapHeader>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap table"))?;
        let remainder = seq
            .next_element::<Vec<u8>>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap table"))?;
        let offset_base = (4 + core.encodingRecords.len() * 8) as u32;
        let mut subtables = Vec::with_capacity(core.encodingRecords.len());
        for er in &core.encodingRecords {
            let subtable_bytes = &remainder[(er.subtableOffset - offset_base) as usize..];
            match subtable_bytes[0..2] {
                [0x0, 0x0] => {
                    let subtable: cmap0 = otspec::de::from_bytes(&subtable_bytes).unwrap();
                    subtables.push(CmapSubtable {
                        format: 0,
                        platformID: er.platformID,
                        encodingID: er.encodingID,
                        languageID: subtable.language,
                        mapping: subtable.to_mapping(),
                    });
                }
                [0x0, 0x04] => {
                    let subtable: cmap4 = otspec::de::from_bytes(&subtable_bytes).unwrap();
                    subtables.push(CmapSubtable {
                        format: 4,
                        platformID: er.platformID,
                        encodingID: er.encodingID,
                        languageID: subtable.language,
                        mapping: subtable.to_mapping(),
                    });
                }
                _ => {
                    println!("Unknown format",);
                }
            }
        }
        Ok(cmap { subtables })
    }
);

#[cfg(test)]
mod tests {
    use crate::cmap;
    use std::iter::FromIterator;

    macro_rules! hashmap {
		    ($($k:expr => $v:expr),* $(,)?) => {
		        std::collections::HashMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
		    };
		}
    #[test]
    fn cmap_de() {
        let fcmap = cmap::cmap {
            subtables: vec![
                cmap::CmapSubtable {
                    format: 4,
                    platformID: 0,
                    encodingID: 3,
                    languageID: 0,
                    mapping: hashmap!( 32 => 1, 160 => 1, 65 => 2 ),
                },
                cmap::CmapSubtable {
                    format: 4,
                    platformID: 3,
                    encodingID: 1,
                    languageID: 0,
                    mapping: hashmap!( 32 => 1, 160 => 1, 65 => 2 ),
                },
            ],
        };
        let binary_cmap = vec![
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x14, 0x00, 0x03,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x14, 0x00, 0x04, 0x00, 0x30, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x08, 0x00, 0x02, 0x00, 0x00, 0x00, 0x20, 0x00, 0x41, 0x00, 0xa0, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x20, 0x00, 0x41, 0x00, 0xa0, 0xff, 0xff, 0xff, 0xe1, 0xff, 0xc1,
            0xff, 0x61, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let deserialized: cmap::cmap = otspec::de::from_bytes(&binary_cmap).unwrap();
        assert_eq!(deserialized, fcmap);
    }
}
