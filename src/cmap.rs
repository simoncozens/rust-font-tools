use itertools::izip;
use otspec::de::CountedDeserializer;
use otspec::ser;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::Deserializer;
use serde::Serializer;
use serde::{Deserialize, Serialize};
extern crate otspec;
use otspec::deserialize_visitor;
use otspec::types::*;
use otspec_macros::tables;
use std::collections::{BTreeMap, HashSet};

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

#[derive(Debug, PartialEq, Serialize)]
struct cmap0 {
    format: uint16,
    length: uint16,
    language: uint16,
    glyphIdArray: Vec<u8>,
}

impl cmap0 {
    fn from_mapping(languageID: uint16, map: &BTreeMap<uint32, uint16>) -> Self {
        return Self {
            format: 0,
            length: 0,
            language: languageID,
            glyphIdArray: Vec::new(),
        };
    }
    fn to_mapping(self) -> BTreeMap<uint32, uint16> {
        return BTreeMap::new();
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

#[derive(Debug, PartialEq, Serialize)]
struct cmap4 {
    format: uint16,
    length: uint16,
    language: uint16,
    segCountX2: uint16,
    searchRange: uint16,
    entrySelector: uint16,
    rangeShift: uint16,
    endCode: Vec<uint16>,
    reservedPad: uint16,
    startCode: Vec<uint16>,
    idDelta: Vec<int16>,
    idRangeOffsets: Vec<uint16>,
    glyphIdArray: Vec<uint16>,
}

impl cmap4 {
    fn from_mapping(languageID: uint16, map: &BTreeMap<uint32, uint16>) -> Self {
        return Self {
            format: 4,
            length: 0,
            language: languageID,
            segCountX2: 0,
            searchRange: 0,
            entrySelector: 0,
            rangeShift: 0,
            endCode: Vec::new(),
            reservedPad: 0,
            startCode: Vec::new(),
            idDelta: Vec::new(),
            idRangeOffsets: Vec::new(),
            glyphIdArray: Vec::new(),
        };
    }

    fn to_mapping(&self) -> BTreeMap<uint32, uint16> {
        let mut map = BTreeMap::new();
        for i in 0..(self.startCode.len() - 1) {
            let start = self.startCode[i];
            let end = self.endCode[i];
            let delta = self.idDelta[i];
            let range_offset = self.idRangeOffsets[i];
            if end == 0xffff {
                break;
            }
            let range_char_codes = start..(1 + end);
            for char_code in range_char_codes {
                if range_offset == 0 {
                    map.insert(char_code as u32, (char_code as i16 + delta) as u16);
                } else {
                    let partial = range_offset / 2 - start + (i - self.idRangeOffsets.len()) as u16;
                    let index = (char_code + partial) as usize;
                    assert!(index < self.glyphIdArray.len());
                    if self.glyphIdArray[index] != 0 {
                        let glyph_id = self.glyphIdArray[index] as i16 + delta;
                        map.insert(char_code as u32, glyph_id as u16);
                    } else {
                        map.insert(char_code as u32, 0);
                    }
                }
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
        let searchRange = seq.next_element::<uint16>()?;
        let entrySelector = seq.next_element::<uint16>()?;
        let rangeShift = seq.next_element::<uint16>()?;
        // println!("segment count {:?}", segcount);
        let endCode: Vec<uint16> = seq
            .next_element_seed(CountedDeserializer::with_len(segcount as usize))?
            .ok_or_else(|| serde::de::Error::custom("Expecting a cmap4 table endcode"))?;
        // println!("endcode {:?}", endCode);
        let reservedPad = seq.next_element::<uint16>()?; // reserved padding
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
            segCountX2: segcount * 2,
            searchRange: 0,
            entrySelector: 0,
            rangeShift: 0,
            endCode,
            reservedPad: 0,
            startCode,
            idDelta,
            idRangeOffsets,
            glyphIdArray,
        })
    }
);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
struct CmapSubtable {
    format: uint16,
    platformID: uint16,
    encodingID: uint16,
    languageID: uint16,
    mapping: BTreeMap<uint32, uint16>,
}

impl CmapSubtable {
    pub fn is_unicode(&self) -> bool {
        self.platformID == 0
            || (self.platformID == 3
                && (self.encodingID == 0 || self.encodingID == 1 || self.encodingID == 10))
    }
    pub fn is_symbol(&self) -> bool {
        self.platformID == 3 && self.encodingID == 0
    }
}

impl Serialize for CmapSubtable {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        match self.format {
            0 => seq.serialize_element(&cmap0::from_mapping(self.languageID, &self.mapping)),
            4 => seq.serialize_element(&cmap4::from_mapping(self.languageID, &self.mapping)),
            _ => unimplemented!(),
        }?;
        seq.end()
    }
}

#[derive(Debug, PartialEq)]
pub struct cmap {
    subtables: Vec<CmapSubtable>,
}

impl Serialize for cmap {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut offsets: BTreeMap<&CmapSubtable, uint32> = BTreeMap::new();
        let mut output: Vec<u8> = Vec::new();
        let mut encoding_records: Vec<EncodingRecord> = Vec::new();
        let offset_base = (4 + self.subtables.len() * 8) as u32;
        for st in &self.subtables {
            if !offsets.contains_key(st) {
                let offset = offset_base + output.len() as u32;
                output.extend(ser::to_bytes(&st).unwrap());
                offsets.insert(st, offset);
            }
            encoding_records.push(EncodingRecord {
                platformID: st.platformID,
                encodingID: st.encodingID,
                subtableOffset: *offsets.get(&st).unwrap(),
            });
        }
        let header = CmapHeader {
            version: 0,
            encodingRecords: vec![],
        };
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element(&header)?;
        seq.serialize_element(&output)?;
        seq.end()
    }
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

impl cmap {
    pub fn getMapping(
        &self,
        platformID: u16,
        encodingID: u16,
    ) -> Option<&BTreeMap<uint32, uint16>> {
        for st in &self.subtables {
            if st.platformID == platformID && st.encodingID == encodingID {
                return Some(&st.mapping);
            }
        }
        None
    }
    pub fn getBestMapping(&self) -> Option<&BTreeMap<uint32, uint16>> {
        for (p, e) in &[
            (3, 10),
            (0, 6),
            (0, 4),
            (3, 1),
            (0, 3),
            (0, 2),
            (0, 1),
            (0, 0),
        ] {
            let maybe_map = self.getMapping(*p, *e);
            if maybe_map.is_some() {
                return maybe_map;
            }
        }
        None
    }

    pub fn reversed(&self) -> BTreeMap<u16, HashSet<u32>> {
        let mut res = BTreeMap::new();
        for subtable in &self.subtables {
            if subtable.is_unicode() {
                for (codepoint, id) in &subtable.mapping {
                    if !res.contains_key(id) {
                        res.insert(*id, HashSet::new());
                    }
                    res.get_mut(id).unwrap().insert(*codepoint);
                }
            }
        }
        res
    }
}

#[cfg(test)]
mod tests {
    use crate::cmap;
    use std::iter::FromIterator;

    macro_rules! btreemap {
		    ($($k:expr => $v:expr),* $(,)?) => {
		        std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
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
                    mapping: btreemap!( 32 => 1, 160 => 1, 65 => 2 ),
                },
                cmap::CmapSubtable {
                    format: 4,
                    platformID: 3,
                    encodingID: 1,
                    languageID: 0,
                    mapping: btreemap!( 32 => 1, 160 => 1, 65 => 2 ),
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

    #[test]
    fn cmap_reversed() {
        let fcmap = cmap::cmap {
            subtables: vec![
                cmap::CmapSubtable {
                    format: 4,
                    platformID: 0,
                    encodingID: 3,
                    languageID: 0,
                    mapping: btreemap!( 32 => 1, 160 => 1, 65 => 2 ),
                },
                cmap::CmapSubtable {
                    format: 4,
                    platformID: 3,
                    encodingID: 1,
                    languageID: 0,
                    mapping: btreemap!( 32 => 1, 160 => 1, 65 => 2 ),
                },
            ],
        };
        let revmap = fcmap.reversed();
        assert!(revmap.get(&2).unwrap().contains(&65));
    }
}
