use otspec::layout::coverage::Coverage;
use otspec::layout::valuerecord::{coerce_to_same_format, ValueRecord, ValueRecordFlags};
use otspec::types::*;
use otspec::utils::is_all_the_same;
use otspec::Serialize;

use otspec::{DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError};

use otspec_macros::Serialize;
use std::collections::BTreeMap;

use crate::format_switching_lookup;

/* This struct is the user-facing representation of single-pos. A mapping of
GID -> valuerecord is a friendly way to represent what's going on. */
#[derive(Debug, PartialEq, Clone, Default)]
/// A single positioning subtable.
pub struct SinglePos {
    /// The mapping of input glyph IDs to value records.
    pub mapping: BTreeMap<GlyphID, ValueRecord>,
}

/*
When we serialize, we have to translate that friendly representation to
OpenType's binary, which has two potential formats. We will choose the best
one behind the scenes, but we need to represent them here as structs to
facilitate serialization.
*/

#[derive(Debug, PartialEq, Clone, Serialize)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct SinglePosFormat1 {
    #[otspec(offset_base)]
    pub posFormat: uint16,
    pub coverage: Offset16<Coverage>,
    pub valueFormat: ValueRecordFlags,
    #[otspec(embed)]
    pub valueRecord: ValueRecord,
}

#[derive(Debug, PartialEq, Clone, Serialize)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct SinglePosFormat2 {
    #[otspec(offset_base)]
    pub posFormat: uint16,
    pub coverage: Offset16<Coverage>,
    pub valueFormat: ValueRecordFlags,
    #[otspec(with = "Counted")]
    pub valueRecords: Vec<ValueRecord>,
}

/*
   This macro creates a SinglePosInternal enum, which
   gives us an entry point for the lower-level format
*/
format_switching_lookup!(SinglePos { Format1, Format2 });

/* We will load directly into our user-facing mapping. */

impl Deserialize for SinglePos {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let mut mapping = BTreeMap::new();
        let format: uint16 = c.de()?;
        let coverage: Offset16<Coverage> = c.de()?;
        let value_format: ValueRecordFlags = c.de()?;
        match format {
            1 => {
                let mut vr: ValueRecord = ValueRecord::from_bytes(c, value_format)?;
                vr.simplify();
                for glyph_id in &coverage.as_ref().unwrap().glyphs {
                    mapping.insert(*glyph_id, vr.clone());
                }
            }
            2 => {
                // Count is not even used because there's one for each glyph in coverage
                let _count: uint16 = c.de()?;
                for glyph_id in coverage.as_ref().unwrap().glyphs.iter() {
                    let mut vr: ValueRecord = ValueRecord::from_bytes(c, value_format)?;
                    vr.simplify();
                    mapping.insert(*glyph_id, vr);
                }
            }
            _ => panic!("Bad single pos format {:?}", format),
        }
        Ok(SinglePos { mapping })
    }
}

/* On serialization, move to the outgoing representation by choosing the best format */
impl From<&SinglePos> for SinglePosInternal {
    fn from(val: &SinglePos) -> Self {
        let mut mapping = val.mapping.clone();
        for (_, val) in mapping.iter_mut() {
            (*val).simplify()
        }
        let coverage = Coverage {
            glyphs: mapping.keys().copied().collect(),
        };
        if is_all_the_same(mapping.values()) {
            let vr = mapping.values().next().unwrap();
            SinglePosInternal::Format1(SinglePosFormat1 {
                posFormat: 1,
                coverage: Offset16::to(coverage),
                valueFormat: vr.flags(),
                valueRecord: vr.clone(),
            })
        } else {
            let vrs: Vec<ValueRecord> = mapping.values().cloned().collect();
            let vrs = coerce_to_same_format(vrs);
            SinglePosInternal::Format2(SinglePosFormat2 {
                posFormat: 2,
                coverage: Offset16::to(coverage),
                valueFormat: vrs[0].flags(),
                valueRecords: vrs,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use otspec::{btreemap, valuerecord};
    use std::iter::FromIterator;

    #[test]
    fn test_single_pos_1_1_serde() {
        let pos = SinglePos {
            mapping: btreemap!(66 => valuerecord!(xAdvance=10)),
        };
        let binary_pos = vec![
            0x00, 0x01, 0x00, 0x08, 0x00, 0x04, 0x00, 0x0a, 0x00, 0x01, 0x00, 0x01, 0x00, 66,
        ];
        let serialized = otspec::ser::to_bytes(&pos).unwrap();
        assert_eq!(serialized, binary_pos);
        let de: SinglePos = otspec::de::from_bytes(&binary_pos).unwrap();
        assert_eq!(de, pos);
    }

    #[test]
    fn test_single_pos_1_1_serde2() {
        let pos = SinglePos {
            mapping: btreemap!(66 => valuerecord!(xAdvance=10),
                67 => valuerecord!(xAdvance=10, yPlacement=0),
            ),
        };
        let binary_pos = vec![
            0x00, 0x01, 0x00, 0x08, 0x00, 0x04, 0x00, 0x0a, 0x00, 0x01, 0x00, 0x02, 0x00, 66, 0x00,
            67,
        ];
        let serialized = otspec::ser::to_bytes(&pos).unwrap();
        assert_eq!(serialized, binary_pos);
        let de: SinglePos = otspec::de::from_bytes(&binary_pos).unwrap();
        assert_eq!(
            de,
            SinglePos {
                mapping: btreemap!(66 => valuerecord!(xAdvance=10),
                    67 => valuerecord!(xAdvance=10), // This gets simplified
                ),
            }
        );
    }

    #[test]
    fn test_single_pos_1_2_serde() {
        let pos = SinglePos {
            mapping: btreemap!(66 => valuerecord!(xAdvance=10),
                67 => valuerecord!(xAdvance=-20),
            ),
        };
        let binary_pos = vec![
            0x00, 0x02, // format
            0x00, 0x0c, // offset to coverage
            0x00, 0x04, // coverage format
            0x00, 0x02, // count of VRs
            0x00, 0x0a, // VR 1
            0xff, 0xec, // VR 2
            0x00, 0x01, 0x00, 0x02, 0x00, 66, 0x00, 67,
        ];
        let serialized = otspec::ser::to_bytes(&pos).unwrap();
        assert_eq!(serialized, binary_pos);
        let de: SinglePos = otspec::de::from_bytes(&binary_pos).unwrap();
        assert_eq!(de, pos);
    }

    #[test]
    fn test_single_pos_1_2_serde2() {
        let pos = SinglePos {
            mapping: btreemap!(66 => valuerecord!(xAdvance=10),
                67 => valuerecord!(xPlacement=-20),
            ),
        };
        let binary_pos = vec![
            0x00, 0x02, // format
            0x00, 0x10, // offset to coverage
            0x00, 0x05, // coverage format (xAdvance|xPlacement)
            0x00, 0x02, // count of VRs
            0x00, 0x00, // VR 1 xPlacement
            0x00, 0x0a, // VR 1 xAdvance
            0xff, 0xec, // VR 2 xPlacement
            0x00, 0x00, // VR 2 xAdvance
            0x00, 0x01, 0x00, 0x02, 0x00, 66, 0x00, 67,
        ];
        let serialized = otspec::ser::to_bytes(&pos).unwrap();
        assert_eq!(serialized, binary_pos);
        let de: SinglePos = otspec::de::from_bytes(&binary_pos).unwrap();
        assert_eq!(de, pos);
    }
}
