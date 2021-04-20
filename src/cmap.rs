use crate::font::get_search_range;
use otspec::de::CountedDeserializer;
use otspec::ser;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::Deserializer;
use serde::Serializer;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::convert::TryInto;
extern crate otspec;
use otspec::deserialize_visitor;
use otspec::types::*;
use otspec_macros::tables;
use std::collections::{BTreeMap, HashSet};
use std::hash::{Hash, Hasher};

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
        Self {
            format: 0,
            length: 0,
            language: languageID,
            glyphIdArray: Vec::new(),
        }
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

fn is_contiguous_list(l: &[u16]) -> bool {
    for ab in l.windows(2) {
        if let [a, b] = ab {
            if *b != *a + 1 {
                return false;
            }
        }
    }
    true
}

fn split_range(
    start_code: u16,
    end_code: u16,
    map: &BTreeMap<uint32, uint16>,
) -> (Vec<u16>, Vec<u16>) {
    if start_code == end_code {
        return (vec![], vec![end_code]);
    }
    let mut last_id = map[&(start_code as u32)];
    let mut last_code = start_code;
    let mut in_order = None;
    let mut ordered_begin = None;
    let mut subranges = Vec::new();
    for code in (start_code + 1)..(end_code + 1) {
        let glyph_id = *map.get(&(code as u32)).unwrap_or(&0);
        if glyph_id - 1 == last_id {
            if in_order.is_none() || in_order == Some(0) {
                in_order = Some(1);
                ordered_begin = Some(last_code);
            }
        } else {
            if in_order == Some(1) {
                in_order = Some(0);
                subranges.push((ordered_begin, last_code));
                ordered_begin = None;
            }
        }
        last_id = glyph_id;
        last_code = code;
    }
    if in_order == Some(1) {
        subranges.push((ordered_begin, last_code));
    }
    assert_eq!(last_code, end_code);

    let mut new_ranges: Vec<(u32, u32)> = Vec::new();
    for (b, e) in subranges {
        let b = b.unwrap();

        if b == start_code && e == end_code {
            break;
        }
        let threshold = if b == start_code || e == end_code {
            4
        } else {
            8
        };
        if (e - b + 1) > threshold {
            new_ranges.push((b.into(), e.into()));
        }
    }

    if new_ranges.is_empty() {
        return (vec![], vec![end_code]);
    }

    if new_ranges[0].0 != (start_code as u32) {
        new_ranges.insert(0, (start_code.into(), new_ranges[0].0 - 1))
    }
    if new_ranges.last().unwrap().1 != (end_code as u32) {
        new_ranges.push((new_ranges.last().unwrap().1 + 1, end_code.into()));
    }
    let mut i = 1;
    while i < new_ranges.len() {
        if new_ranges[i - 1].1 + 1 != new_ranges[i].0 {
            new_ranges.insert(i, (new_ranges[i - 1].1 + 1, new_ranges[i].0 - 1));
            i += 1;
        }
        i += 1;
    }
    let mut start: Vec<u16> = Vec::new();
    let mut end: Vec<u16> = Vec::new();
    for (b, e) in new_ranges {
        start.push(b as u16);
        end.push(e as u16);
    }
    start.drain(0..1);
    assert_eq!(start.len() + 1, end.len());
    (start, end)
}

impl cmap4 {
    fn from_mapping(languageID: uint16, map: &BTreeMap<uint32, uint16>) -> Self {
        let mut char_codes: Vec<uint32> = map.keys().cloned().collect();
        char_codes.sort_unstable();
        let mut last_code = char_codes[0];
        let mut startCode: Vec<u16> = vec![last_code.try_into().unwrap()];
        let mut endCode: Vec<u16> = Vec::new();
        for char_code in &char_codes[1..] {
            if *char_code == last_code + 1 {
                last_code = *char_code;
                continue;
            }
            let (mut start, mut end) = split_range(
                *startCode.last().unwrap(),
                last_code.try_into().unwrap(),
                map,
            );
            // println!("Split_range called, returned {:?} {:?}", start, end);
            startCode.append(&mut start);
            endCode.append(&mut end);
            startCode.push((*char_code).try_into().unwrap());
            last_code = *char_code;
        }
        let (mut start, mut end) = split_range(
            *startCode.last().unwrap(),
            last_code.try_into().unwrap(),
            map,
        );
        startCode.append(&mut start);
        endCode.append(&mut end);
        startCode.push(0xffff);
        endCode.push(0xffff);
        // println!("Start code array: {:?} ", startCode);
        // println!("End code array: {:?}", endCode);
        let mut idDelta: Vec<i16> = Vec::new();
        let mut idRangeOffsets = Vec::new();
        let mut glyphIndexArray = Vec::new();
        for i in 0..(endCode.len() - 1) {
            let mut indices: Vec<u16> = Vec::new();
            for char_code in startCode[i]..endCode[i] + 1 {
                let gid = *map.get(&(char_code as u32)).unwrap_or(&0);
                indices.push(gid);
            }
            if is_contiguous_list(&indices) {
                // println!("Contiguous list {:?}", indices);
                idDelta.push((indices[0] as i16 - startCode[i] as i16) as i16);
                idRangeOffsets.push(0);
            } else {
                // println!("Non contiguous list {:?}", indices);
                idDelta.push(0);
                idRangeOffsets.push(2 * (endCode.len() + glyphIndexArray.len() - i) as u16);
                glyphIndexArray.append(&mut indices);
            }
        }
        // println!("ID Delta array: {:?}", idDelta);
        // println!("ID Range Offset array: {:?}", idRangeOffsets);
        idDelta.push(1);
        idRangeOffsets.push(0);
        let segcount = endCode.len() as u16;
        let (searchRange, entrySelector, rangeShift) = get_search_range(segcount, 2);
        Self {
            format: 4,
            length: (glyphIndexArray.len() * 2 + 16 + 2 * 4 * segcount as usize) as u16,
            language: languageID,
            segCountX2: segcount * 2,
            searchRange,
            entrySelector,
            rangeShift,
            endCode,
            reservedPad: 0,
            startCode,
            idDelta,
            idRangeOffsets,
            glyphIdArray: glyphIndexArray,
        }
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
                    // println!("Range offset/2: {:?}", range_offset / 2);
                    // println!("start: {:?}", start);
                    // println!("i: {:?}", i);
                    // println!("Range of idRangeOffsets: {:?}", self.idRangeOffsets.len());
                    let partial = (range_offset / 2) as i16
                        - (start as i16 + i as i16 - self.idRangeOffsets.len() as i16) as i16;
                    // println!("Partial: {:?}", partial);
                    let index = (char_code as i16 + partial) as usize;
                    // println!("Index: {:?}", index);
                    // println!("GlyphIdArray: {:?}", self.glyphIdArray.len());
                    // XXX
                    if index >= self.glyphIdArray.len() {
                        break;
                    }
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
        let _rangeShift = seq.next_element::<uint16>()?;
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
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
        let mut offsets: BTreeMap<u64, uint32> = BTreeMap::new();
        let mut output: Vec<u8> = Vec::new();
        let mut encoding_records: Vec<EncodingRecord> = Vec::new();
        let offset_base = (4 + self.subtables.len() * 8) as u32;
        for st in &self.subtables {
            let mut hash = DefaultHasher::new();
            st.mapping.hash(&mut hash);
            let hash_value = hash.finish();
            if !offsets.contains_key(&hash_value) {
                let offset = offset_base + output.len() as u32;
                output.extend(ser::to_bytes(&st).unwrap());
                offsets.insert(hash_value, offset);
            }
            encoding_records.push(EncodingRecord {
                platformID: st.platformID,
                encodingID: st.encodingID,
                subtableOffset: *offsets.get(&hash_value).unwrap(),
            });
        }
        let header = CmapHeader {
            version: 0,
            encodingRecords: encoding_records,
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
    fn cmap_ser() {
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
        let expected = vec![
            0x00, 0x00, 0x00, 0x02, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x14, 0x00, 0x03,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x14, 0x00, 0x04, 0x00, 0x30, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x08, 0x00, 0x02, 0x00, 0x00, 0x00, 0x20, 0x00, 0x41, 0x00, 0xa0, 0xff, 0xff,
            0x00, 0x00, 0x00, 0x20, 0x00, 0x41, 0x00, 0xa0, 0xff, 0xff, 0xff, 0xe1, 0xff, 0xc1,
            0xff, 0x61, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let serialized = otspec::ser::to_bytes(&fcmap).unwrap();
        assert_eq!(serialized, expected);
    }
    #[test]
    fn cmap_serde_notosansarmenian() {
        let binary_cmap = vec![
            0x00, 0x00, 0x00, 0x01, 0x00, 0x03, 0x00, 0x01, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x04,
            0x00, 0x70, 0x00, 0x00, 0x00, 0x18, 0x00, 0x10, 0x00, 0x03, 0x00, 0x08, 0x00, 0x00,
            0x00, 0x0d, 0x00, 0x20, 0x00, 0xa0, 0x05, 0x56, 0x05, 0x5f, 0x05, 0x87, 0x05, 0x8a,
            0x05, 0x8f, 0xfb, 0x17, 0xfe, 0xff, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0d,
            0x00, 0x20, 0x00, 0xa0, 0x05, 0x31, 0x05, 0x59, 0x05, 0x61, 0x05, 0x89, 0x05, 0x8f,
            0xfb, 0x13, 0xfe, 0xff, 0xff, 0xff, 0x00, 0x01, 0xff, 0xf5, 0xff, 0xe3, 0xff, 0x63,
            0xfa, 0xd3, 0xfa, 0xd1, 0xfa, 0xd0, 0xfa, 0xcf, 0xfa, 0xd0, 0x05, 0x47, 0x01, 0x02,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        let bt = btreemap!(0 => 1, 13 => 2, 32 => 3, 160 => 3, 1329 => 4, 1330 => 5, 1331 => 6, 1332 => 7, 1333 => 8, 1334 => 9, 1335 => 10, 1336 => 11, 1337 => 12, 1338 => 13, 1339 => 14, 1340 => 15, 1341 => 16, 1342 => 17, 1343 => 18, 1344 => 19, 1345 => 20, 1346 => 21, 1347 => 22, 1348 => 23, 1349 => 24, 1350 => 25, 1351 => 26, 1352 => 27, 1353 => 28, 1354 => 29, 1355 => 30, 1356 => 31, 1357 => 32, 1358 => 33, 1359 => 34, 1360 => 35, 1361 => 36, 1362 => 37, 1363 => 38, 1364 => 39, 1365 => 40, 1366 => 41, 1369 => 42, 1370 => 43, 1371 => 44, 1372 => 45, 1373 => 46, 1374 => 47, 1375 => 48, 1377 => 49, 1378 => 50, 1379 => 51, 1380 => 52, 1381 => 53, 1382 => 54, 1383 => 55, 1384 => 56, 1385 => 57, 1386 => 58, 1387 => 59, 1388 => 60, 1389 => 61, 1390 => 62, 1391 => 63, 1392 => 64, 1393 => 65, 1394 => 66, 1395 => 67, 1396 => 68, 1397 => 69, 1398 => 70, 1399 => 71, 1400 => 72, 1401 => 73, 1402 => 74, 1403 => 75, 1404 => 76, 1405 => 77, 1406 => 78, 1407 => 79, 1408 => 80, 1409 => 81, 1410 => 82, 1411 => 83, 1412 => 84, 1413 => 85, 1414 => 86, 1415 => 87, 1417 => 88, 1418 => 89, 1423 => 95, 64275 => 90, 64276 => 91, 64277 => 92, 64278 => 93, 64279 => 94, 65279 => 1);
        let fcmap = cmap::cmap {
            subtables: vec![cmap::CmapSubtable {
                format: 4,
                platformID: 3,
                encodingID: 1,
                languageID: 0,
                mapping: bt,
            }],
        };
        let deserialized: cmap::cmap = otspec::de::from_bytes(&binary_cmap).unwrap();
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(deserialized, fcmap);
        assert_eq!(serialized, binary_cmap);
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
