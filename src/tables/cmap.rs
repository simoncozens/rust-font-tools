use crate::font::get_search_range;
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use otspec_macros::{tables, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashSet};
use std::convert::TryInto;
use std::hash::{Hash, Hasher};

/// The 'cmap' OpenType tag.
pub const TAG: Tag = crate::tag!("cmap");

tables!(

EncodingRecord {
        uint16 platformID
        uint16 encodingID
        uint32 subtableOffset
}

CmapHeader {
    [offset_base]
    uint16  version
    Counted(EncodingRecord) encodingRecords
}

);

#[derive(Clone, Debug, PartialEq, Serialize)]
#[allow(non_camel_case_types, non_snake_case)]
struct cmap0 {
    format: uint16,
    length: uint16,
    language: uint16,
    glyphIdArray: Vec<u8>,
}

impl cmap0 {
    fn from_mapping(_language_id: uint16, _map: &BTreeMap<uint32, uint16>) -> Self {
        unimplemented!();
        // Self {
        //     format: 0,
        //     length: 0,
        //     language: languageID,
        //     glyphIdArray: Vec::new(),
        // }
    }
    fn to_mapping(&self) -> BTreeMap<uint32, uint16> {
        BTreeMap::new()
    }
}

impl Deserialize for cmap0 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let format: uint16 = c.de()?;
        let length: uint16 = c.de()?;
        let language: uint16 = c.de()?;
        let records = 256.max(length - 6);
        let glyph_ids: Result<Vec<u8>, DeserializationError> =
            (0..records).map(|_| c.de()).collect();
        Ok(cmap0 {
            format,
            length,
            language,
            glyphIdArray: glyph_ids?,
        })
    }
}

#[allow(non_camel_case_types, non_snake_case)]
#[derive(Clone, Debug, PartialEq, Serialize)]
/// A format 4 cmap subtable, used for mapping Unicode characters in the
/// basic mutilingual plane.
pub struct cmap4 {
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
        if glyph_id > 0 && glyph_id - 1 == last_id {
            if in_order.is_none() || in_order == Some(0) {
                in_order = Some(1);
                ordered_begin = Some(last_code);
            }
        } else if in_order == Some(1) {
            in_order = Some(0);
            subranges.push((ordered_begin, last_code));
            ordered_begin = None;
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
    /// Creates a new cmap4 subtable for a given language ID, from a mapping of
    /// Unicode codepoints to glyph IDs
    pub fn from_mapping(language_id: uint16, map: &BTreeMap<uint32, uint16>) -> Self {
        let mut char_codes: Vec<uint32> = map.keys().cloned().collect();
        char_codes.sort_unstable();
        let mut last_code = char_codes[0];
        let mut start_code: Vec<u16> = vec![last_code.try_into().unwrap()];
        let mut end_code: Vec<u16> = Vec::new();
        for char_code in &char_codes[1..] {
            if *char_code == last_code + 1 {
                last_code = *char_code;
                continue;
            }
            let (mut start, mut end) = split_range(
                *start_code.last().unwrap(),
                last_code.try_into().unwrap(),
                map,
            );
            // println!("Split_range called, returned {:?} {:?}", start, end);
            start_code.append(&mut start);
            end_code.append(&mut end);
            start_code.push((*char_code).try_into().unwrap());
            last_code = *char_code;
        }
        let (mut start, mut end) = split_range(
            *start_code.last().unwrap(),
            last_code.try_into().unwrap(),
            map,
        );
        start_code.append(&mut start);
        end_code.append(&mut end);
        start_code.push(0xffff);
        end_code.push(0xffff);
        // println!("Start code array: {:?} ", startCode);
        // println!("End code array: {:?}", end_code);
        let mut id_delta: Vec<i16> = Vec::new();
        let mut id_range_offsets = Vec::new();
        let mut glyph_index_array = Vec::new();
        for i in 0..(end_code.len() - 1) {
            let mut indices: Vec<u16> = Vec::new();
            for char_code in start_code[i]..end_code[i] + 1 {
                let gid = *map.get(&(char_code as u32)).unwrap_or(&0);
                indices.push(gid);
            }
            if is_contiguous_list(&indices) {
                // println!("Contiguous list {:?}", indices);
                id_delta.push((indices[0] as i32 - start_code[i] as i32) as i16);
                id_range_offsets.push(0);
            } else {
                // println!("Non contiguous list {:?}", indices);
                id_delta.push(0);
                id_range_offsets.push(2 * (end_code.len() + glyph_index_array.len() - i) as u16);
                glyph_index_array.append(&mut indices);
            }
        }
        // println!("ID Delta array: {:?}", id_delta);
        // println!("ID Range Offset array: {:?}", id_range_offsets);
        id_delta.push(1);
        id_range_offsets.push(0);
        let segcount = end_code.len() as u16;
        let range_parameters = get_search_range(segcount, 2);
        Self {
            format: 4,
            length: (glyph_index_array.len() * 2 + 16 + 2 * 4 * segcount as usize) as u16,
            language: language_id,
            segCountX2: segcount * 2,
            searchRange: range_parameters.0,
            entrySelector: range_parameters.1,
            rangeShift: range_parameters.2,
            endCode: end_code,
            reservedPad: 0,
            startCode: start_code,
            idDelta: id_delta,
            idRangeOffsets: id_range_offsets,
            glyphIdArray: glyph_index_array,
        }
    }

    fn to_mapping(&self) -> BTreeMap<uint32, uint16> {
        let mut map = BTreeMap::new();
        for i in 0..(self.startCode.len() - 1) {
            let start = self.startCode[i];
            let end = self.endCode[i];
            let delta = self.idDelta[i];
            let range_offset = self.idRangeOffsets[i];
            let partial = ((range_offset / 2) as i32 - (start as i32) + (i as i32)
                - (self.idRangeOffsets.len() as i32)) as i32;
            if end == 0xffff {
                break;
            }
            let range_char_codes = start..(1 + end);
            for char_code in range_char_codes {
                if range_offset == 0 {
                    map.insert(char_code as u32, (char_code as i32 + delta as i32) as u16);
                } else {
                    let index = (char_code as i32 + partial) as usize;
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

impl Deserialize for cmap4 {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let format: uint16 = c.de()?;
        let length: uint16 = c.de()?;
        let language: uint16 = c.de()?;
        let seg_count_x2: uint16 = c.de()?;
        let segcount: usize = seg_count_x2 as usize / 2;
        c.skip(6);
        let end_code: Vec<uint16> = c.de_counted(segcount)?;
        c.skip(2);
        let start_code: Vec<uint16> = c.de_counted(segcount)?;
        let id_delta: Vec<int16> = c.de_counted(segcount)?;
        let id_range_offsets: Vec<uint16> = c.de_counted(segcount)?;
        let len_so_far = 16 + (segcount * 2 * 4);
        let remainder = length as usize - len_so_far;
        let glyph_id_array: Vec<u16> = c.de_counted(remainder / 2).unwrap_or_default();
        Ok(cmap4 {
            format,
            length,
            language,
            segCountX2: seg_count_x2,
            searchRange: 0,
            entrySelector: 0,
            rangeShift: 0,
            endCode: end_code,
            reservedPad: 0,
            startCode: start_code,
            idDelta: id_delta,
            idRangeOffsets: id_range_offsets,
            glyphIdArray: glyph_id_array,
        })
    }
}

tables!(
cmap12 {
    uint16 format
    uint16 reserved
    uint32 length
    uint32 language
    Counted32(SequentialMapGroup) groups
}

SequentialMapGroup {
    uint32  startCharCode
    uint32  endCharCode
    uint32  startGlyphID
}
);

fn in_run(gid: u16, last_gid: u16, code: u32, last_code: u32) -> bool {
    (gid == 1 + last_gid) && (code == 1 + last_code)
}
impl cmap12 {
    /// Creates a new cmap12 subtable for a given language ID, from a mapping of
    /// Unicode codepoints to glyph IDs
    pub fn from_mapping(language_id: uint16, map: &BTreeMap<uint32, uint16>) -> Self {
        let mut char_codes: Vec<uint32> = map.keys().cloned().collect();
        char_codes.sort_unstable();
        let mut iter = char_codes.iter();
        let mut start_code: u32 = *(iter.next().unwrap());
        if start_code == 0 {
            // Try again
            start_code = *(iter.next().unwrap());
        }
        let mut last_code = start_code - 1;
        let mut start_gid = map.get(&start_code).unwrap();
        let mut last_gid = start_gid - 1;
        let mut groups: Vec<SequentialMapGroup> = vec![];
        for &code in iter {
            let gid = map.get(&code).unwrap();
            if !in_run(*gid, last_gid, code, last_code) {
                groups.push(SequentialMapGroup {
                    startCharCode: start_code,
                    endCharCode: last_code,
                    startGlyphID: *start_gid as u32,
                });
                start_code = code;
                start_gid = gid;
            }
            last_gid = *gid;
            last_code = code;
        }
        groups.push(SequentialMapGroup {
            startCharCode: start_code,
            endCharCode: last_code,
            startGlyphID: *start_gid as u32,
        });
        cmap12 {
            format: 12,
            reserved: 0,
            length: (16 + 12 * groups.len()) as uint32,
            language: language_id as uint32,
            groups,
        }
    }
    fn to_mapping(&self) -> BTreeMap<uint32, uint16> {
        let mut map = BTreeMap::new();
        for group in &self.groups {
            let items = 1 + (group.endCharCode - group.startCharCode);
            for i in 0..items {
                map.insert(
                    group.startCharCode + i,
                    group.startGlyphID as u16 + i as u16,
                );
            }
        }
        map
    }
}

tables!(
    cmap14 {
        [offset_base]
        uint16  format
        uint32  length
        [embed]
        Counted32(VariationSelector) varSelector
    }
    VariationSelector [embedded] {
        uint24 varSelector
        Offset32(DefaultUVS) defaultUVS
        Offset32(NonDefaultUVS) nonDefaultUVS
    }
    DefaultUVS {
        Counted32(UnicodeRange) ranges
    }
    UnicodeRange {
        uint24  startUnicodeValue
        uint8   additionalCount
    }
    NonDefaultUVS {
        Counted32(UVSMapping) uvsMappings
    }
    UVSMapping {
        uint24  unicodeValue
        uint16  glyphID
    }
);

impl cmap14 {
    /// Create a format 14 cmap table from a mapping of Unicode variation sequences
    ///
    /// The input should be a map between `(codepoint of character, codepoint of selector)`
    /// and `glyph ID of selected glyph`.
    pub fn from_uvs_mapping(map: &Option<BTreeMap<(uint32, uint32), uint16>>) -> Self {
        let mut table = cmap14 {
            format: 14,
            length: 10,
            varSelector: vec![],
        };
        if map.is_none() {
            return table;
        }
        let mut total_length = 10;
        let map = map.as_ref().unwrap();

        let mut split_map: BTreeMap<uint32, Vec<(uint32, uint16)>> = BTreeMap::new();
        for ((codepoint, selector), gid) in map.iter() {
            split_map
                .entry(*selector)
                .or_insert_with(std::vec::Vec::new)
                .push((*codepoint, *gid));
        }

        for (selector, records) in split_map.iter() {
            let nduvs = NonDefaultUVS {
                uvsMappings: records
                    .iter()
                    .map(|&(unicode_value, glyph_id)| UVSMapping {
                        unicodeValue: unicode_value.into(),
                        glyphID: glyph_id,
                    })
                    .collect(),
            };
            table.varSelector.push(VariationSelector {
                varSelector: (*selector).into(),
                defaultUVS: Offset32::to_nothing(),
                nonDefaultUVS: Offset32::to(nduvs),
            });
            // Each UVS mapping is 5 bytes
            // Plus the 4-byte count in NonDefaultUVS
            // Plus the 11-byte VariationSelector structure itself
            total_length += 5 * records.len() as u32 + 4 + 11;
        }

        table.length = total_length;
        table
    }

    fn to_uvs_mapping(&self) -> BTreeMap<(uint32, uint32), uint16> {
        let mut map: BTreeMap<(uint32, uint32), uint16> = BTreeMap::new();
        for vs in &self.varSelector {
            let selector = vs.varSelector.into();
            if let Some(_default_map) = &vs.defaultUVS.link {
                // I think this doesn't convey any useful information?
            }
            if let Some(non_default_map) = &vs.nonDefaultUVS.link {
                for record in &non_default_map.uvsMappings {
                    map.insert((record.unicodeValue.into(), selector), record.glyphID);
                }
            }
        }
        map
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[allow(non_snake_case)]
/// A cmap subtable.
///
/// A cmap table can contain multiple mappings of characters
/// to glyphs, both because of differences in mapping based on platform,
/// encoding and language, but also because the mapping may best be expressed
/// by splitting it up into subtables in different formats. This struct
/// represents a mapping in a given format at a relatively high, format-independent
/// level. This subtable is converted to a format-specific subtable on serialize.
pub struct CmapSubtable {
    /// The format to be used to serialize this table. Generally speaking, you
    /// want format 4 or 6 for mappings within the BMP, 10 for higher Unicode
    /// planes, and 14 for Unicode Variation Sequences.
    pub format: uint16,
    /// The platform ID: Unicode = 0, Macintosh = 1, Windows = 3.
    pub platformID: uint16,
    /// The encoding ID; interpretation varies dependent on platform.
    pub encodingID: uint16,
    /// The language ID; interpretation varies dependent on platform and encoding.
    pub languageID: uint16,
    /// A mapping between Unicode codepoints and glyph IDs.
    pub mapping: BTreeMap<uint32, uint16>,
    /// A mapping of Unicode codepoints + glyph selectors to glyph IDs.
    pub uvs_mapping: Option<BTreeMap<(uint32, uint32), uint16>>,
}

impl CmapSubtable {
    /// Returns true if this subtable contains a mapping targetted at the
    /// Unicode platform or a Unicode encoding of the Windows platform.
    pub fn is_unicode(&self) -> bool {
        self.platformID == 0
            || (self.platformID == 3
                && (self.encodingID == 0 || self.encodingID == 1 || self.encodingID == 10))
    }
    /// Returns true if this subtable contains a mapping targetted at the
    /// Windows Symbol encoding.
    pub fn is_symbol(&self) -> bool {
        self.platformID == 3 && self.encodingID == 0
    }
}

impl Serialize for CmapSubtable {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        match self.format {
            0 => cmap0::from_mapping(self.languageID, &self.mapping).to_bytes(data),
            4 => cmap4::from_mapping(self.languageID, &self.mapping).to_bytes(data),
            12 => cmap12::from_mapping(self.languageID, &self.mapping).to_bytes(data),
            14 => cmap14::from_uvs_mapping(&self.uvs_mapping).to_bytes(data),
            _ => unimplemented!(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
/// cmap table. The cmap table is a collection of subtables, as described above.
pub struct cmap {
    /// The list of subtables
    pub subtables: Vec<CmapSubtable>,
}

impl Serialize for cmap {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let mut offsets: BTreeMap<u64, uint32> = BTreeMap::new();
        let mut output: Vec<u8> = Vec::new();
        let mut encoding_records: Vec<EncodingRecord> = Vec::new();
        let offset_base = (4 + self.subtables.len() * 8) as u32;
        for st in &self.subtables {
            let mut hash = DefaultHasher::new();
            st.mapping.hash(&mut hash);
            let hash_value = hash.finish();
            let entry = offsets.entry(hash_value).or_insert_with(|| {
                let offset = offset_base + output.len() as u32;
                st.to_bytes(&mut output).unwrap();
                offset
            });
            encoding_records.push(EncodingRecord {
                platformID: st.platformID,
                encodingID: st.encodingID,
                subtableOffset: *entry,
            });
        }
        let header = CmapHeader {
            version: 0,
            encodingRecords: encoding_records,
        };
        header.to_bytes(data)?;
        output.to_bytes(data)
    }
}

impl Deserialize for cmap {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let core: CmapHeader = c.de()?;
        let mut subtables = Vec::with_capacity(core.encodingRecords.len());
        for er in &core.encodingRecords {
            c.ptr = c.top_of_table() + er.subtableOffset as usize;

            match c.peek(2)? {
                [0x0, 0x0] => {
                    let subtable: cmap0 = c.de()?;
                    subtables.push(CmapSubtable {
                        format: 0,
                        platformID: er.platformID,
                        encodingID: er.encodingID,
                        languageID: subtable.language,
                        mapping: subtable.to_mapping(),
                        uvs_mapping: None,
                    });
                }
                [0x0, 0x04] => {
                    let subtable: cmap4 = c.de()?;
                    subtables.push(CmapSubtable {
                        format: 4,
                        platformID: er.platformID,
                        encodingID: er.encodingID,
                        languageID: subtable.language,
                        mapping: subtable.to_mapping(),
                        uvs_mapping: None,
                    });
                }
                [0x0, 0x0c] => {
                    let subtable: cmap12 = c.de()?;
                    subtables.push(CmapSubtable {
                        format: 12,
                        platformID: er.platformID,
                        encodingID: er.encodingID,
                        languageID: subtable.language as u16,
                        mapping: subtable.to_mapping(),
                        uvs_mapping: None,
                    })
                }
                [0x0, 0x0e] => {
                    let subtable: cmap14 = c.de()?;
                    subtables.push(CmapSubtable {
                        format: 14,
                        platformID: 0,
                        encodingID: 5,
                        languageID: 0, // ??
                        mapping: BTreeMap::new(),
                        uvs_mapping: Some(subtable.to_uvs_mapping()),
                    })
                }
                [a, b] => {
                    panic!("Unknown cmap format {:}{:}", a, b);
                }
                _ => {
                    panic!("Reading cmap format failed");
                }
            }
        }
        Ok(cmap { subtables })
    }
}

impl cmap {
    /// Tries to find a mapping targetted at the the given platform and
    /// encoding. Returns a `Some<map>` if one is found, or `None` otherwise.
    pub fn get_mapping(
        &self,
        platform_id: u16,
        encoding_id: u16,
    ) -> Option<&BTreeMap<uint32, uint16>> {
        for st in &self.subtables {
            if st.platformID == platform_id && st.encodingID == encoding_id {
                return Some(&st.mapping);
            }
        }
        None
    }

    /// Tries to return a "good" mapping by searching for common combinations
    /// of platform and encoding. Returns `None` if no such good mapping could
    /// be found.
    pub fn get_best_mapping(&self) -> Option<&BTreeMap<uint32, uint16>> {
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
            let maybe_map = self.get_mapping(*p, *e);
            if maybe_map.is_some() {
                return maybe_map;
            }
        }
        None
    }

    /// Returns a reverse map, mapping a glyph ID to a set of Unicode codepoints.
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
    use pretty_assertions::assert_eq;
    use std::collections::BTreeMap;
    use std::iter::FromIterator;

    macro_rules! btreemap {
		    ($($k:expr => $v:expr),* $(,)?) => {
		        std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
		    };
		}
    #[test]
    fn cmap_de() {
        let fcmap = super::cmap {
            subtables: vec![
                super::CmapSubtable {
                    format: 4,
                    platformID: 0,
                    encodingID: 3,
                    languageID: 0,
                    mapping: btreemap!( 32 => 1, 160 => 1, 65 => 2 ),
                    uvs_mapping: None,
                },
                super::CmapSubtable {
                    format: 4,
                    platformID: 3,
                    encodingID: 1,
                    languageID: 0,
                    mapping: btreemap!( 32 => 1, 160 => 1, 65 => 2 ),
                    uvs_mapping: None,
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
        let deserialized: super::cmap = otspec::de::from_bytes(&binary_cmap).unwrap();
        assert_eq!(deserialized, fcmap);
    }

    #[test]
    fn cmap_ser() {
        let fcmap = super::cmap {
            subtables: vec![
                super::CmapSubtable {
                    format: 4,
                    platformID: 0,
                    encodingID: 3,
                    languageID: 0,
                    mapping: btreemap!( 32 => 1, 160 => 1, 65 => 2 ),
                    uvs_mapping: None,
                },
                super::CmapSubtable {
                    format: 4,
                    platformID: 3,
                    encodingID: 1,
                    languageID: 0,
                    mapping: btreemap!( 32 => 1, 160 => 1, 65 => 2 ),
                    uvs_mapping: None,
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
        let fcmap = super::cmap {
            subtables: vec![super::CmapSubtable {
                format: 4,
                platformID: 3,
                encodingID: 1,
                languageID: 0,
                mapping: bt,
                uvs_mapping: None,
            }],
        };
        let deserialized: super::cmap = otspec::de::from_bytes(&binary_cmap).unwrap();
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(deserialized, fcmap);
        assert_eq!(serialized, binary_cmap);
    }
    #[test]
    fn cmap_reversed() {
        let fcmap = super::cmap {
            subtables: vec![
                super::CmapSubtable {
                    format: 4,
                    platformID: 0,
                    encodingID: 3,
                    languageID: 0,
                    mapping: btreemap!( 32 => 1, 160 => 1, 65 => 2 ),
                    uvs_mapping: None,
                },
                super::CmapSubtable {
                    format: 4,
                    platformID: 3,
                    encodingID: 1,
                    languageID: 0,
                    mapping: btreemap!( 32 => 1, 160 => 1, 65 => 2 ),
                    uvs_mapping: None,
                },
            ],
        };
        let revmap = fcmap.reversed();
        assert!(revmap.get(&2).unwrap().contains(&65));
    }

    #[test]
    fn cmap_deser_notosans() {
        let binary_cmap = vec![
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x04,
            0x00, 0x3c, 0x00, 0x00, 0x00, 0x08, 0x00, 0x08, 0x00, 0x02, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x0d, 0x00, 0x25, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0d, 0x00, 0x20,
            0xff, 0xff, 0x00, 0xeb, 0x00, 0x28, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x04, 0x00, 0x00, 0x07, 0x08, 0x04, 0x20, 0x06, 0x8b, 0x05, 0x7e, 0x03, 0x4f,
            0x06, 0x4c,
        ];
        let deserialized: super::cmap = otspec::de::from_bytes(&binary_cmap).unwrap();
        assert_eq!(deserialized.subtables[0].mapping.len(), 8);
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_cmap);
    }

    #[test]
    fn cmap_deser_14() {
        let binary_cmap = vec![
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x0e,
            0x00, 0x00, 0x00, 0x3c, 0x00, 0x00, 0x00, 0x02, 0x0e, 0x01, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x20, 0x0e, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x2e, 0x00, 0x00, 0x00, 0x02, 0x00, 0x53, 0xc9, 0x20, 0xb3, 0x00, 0x54, 0xac,
            0x21, 0x26, 0x00, 0x00, 0x00, 0x02, 0x00, 0x50, 0x26, 0x20, 0xb7, 0x00, 0x50, 0xc5,
            0x20, 0xb8,
        ];
        let deserialized: super::cmap = otspec::de::from_bytes(&binary_cmap).unwrap();
        assert_eq!(
            deserialized.subtables[0],
            super::CmapSubtable {
                format: 14,
                platformID: 0,
                encodingID: 5,
                languageID: 0,
                mapping: BTreeMap::new(),
                uvs_mapping: Some(
                    btreemap!( (20518, 917761) => 8375, (20677, 917761) => 8376, (21449, 917760) => 8371, (21676, 917760) => 8486 )
                )
            }
        );
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_cmap);
    }
}
