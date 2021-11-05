use crate::layout::classdef::ClassDef;
use crate::layout::coverage::Coverage;
use crate::layout::device::Device;
use crate::otvar::ItemVariationStore;
use otspec::types::*;
use otspec::Serializer;
use std::iter::FromIterator;

use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use otspec_macros::tables;
use std::collections::{BTreeMap, BTreeSet};

/// The 'GDEF' OpenType tag.
pub const TAG: Tag = crate::tag!("GDEF");

// Having version-specific tables makes it so much easier to keep track of
// the offset fields
tables!(
    gdefcore11 {
        uint16 majorVersion
        uint16 minorVersion
        Offset16(ClassDef) glyphClassDef
        Offset16(AttachList) attachList
        Offset16(LigCaretList) ligCaretList
        Offset16(ClassDef) markAttachClassDef
    }

    gdefcore12 {
        uint16 majorVersion
        uint16 minorVersion
        Offset16(ClassDef) glyphClassDef
        Offset16(AttachList) attachList
        Offset16(LigCaretList) ligCaretList
        Offset16(ClassDef) markAttachClassDef
        Offset16(MarkGlyphSets) markGlyphSets
    }

    gdefcore13 {
        uint16 majorVersion
        uint16 minorVersion
        Offset16(ClassDef) glyphClassDef
        Offset16(AttachList) attachList
        Offset16(LigCaretList) ligCaretList
        Offset16(ClassDef) markAttachClassDef
        Offset16(MarkGlyphSets) markGlyphSets
        Offset32(ItemVariationStore) itemVarStore
    }

    AttachList {
        [offset_base]
        Offset16(Coverage) coverage
        CountedOffset16(AttachPoint) attachPoints
    }

    AttachPoint {
        Counted(uint16) pointIndices
    }

    LigCaretList {
        [offset_base]
        Offset16(Coverage) coverage
        CountedOffset16(LigGlyph) ligGlyph
    }

    LigGlyph {
        [offset_base]
        CountedOffset16(CaretValue) caretValue
    }

    MarkGlyphSets {
        uint16 format
        CountedOffset32(Coverage) coverage
    }
);

#[allow(non_snake_case)]
#[derive(Debug, Clone, PartialEq)]
/// A low-level caret value in a GDEF table
pub enum CaretValue {
    /// A format 1 caret value
    Format1 {
        /// X or Y value, in design units
        coordinate: int16,
    },
    /// A format 2 caret value
    Format2 {
        /// Contour point index on glyph
        pointIndex: uint16,
    },
    /// A format 3 caret value
    Format3 {
        /// X or Y value, in design units
        coordinate: int16,
        ///  Device table (non-variable font) / Variation Index table (variable font) for X or Y value
        device: Offset16<Device>,
    },
}

impl Serialize for CaretValue {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        match &self {
            Self::Format1 { coordinate } => {
                data.put(1_u16)?;
                data.put(coordinate)
            }
            Self::Format2 { pointIndex } => {
                data.put(2_u16)?;
                data.put(pointIndex)
            }
            Self::Format3 { coordinate, device } => {
                data.put(3_u16)?;
                data.put(coordinate)?;
                device.to_bytes(data)
            }
        }
    }

    fn offset_fields(&self) -> Vec<&dyn OffsetMarkerTrait> {
        match &self {
            Self::Format1 { .. } | Self::Format2 { .. } => {
                vec![]
            }
            Self::Format3 {
                coordinate: _,
                device: d,
            } => {
                vec![d]
            }
        }
    }
}

impl Deserialize for CaretValue {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let format: uint16 = c.de()?;
        match format {
            1 => Ok(CaretValue::Format1 {
                coordinate: c.de()?,
            }),
            2 => Ok(CaretValue::Format2 {
                pointIndex: c.de()?,
            }),
            3 => {
                let coordinate: int16 = c.de()?;
                let device: Offset16<Device> = c.de()?;
                Ok(CaretValue::Format3 { coordinate, device })
            }
            _ => Err(DeserializationError(format!(
                "Bad caret value format {:}",
                format
            ))),
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
/// A glyph class definition in the GDEF table
pub enum GlyphClass {
    /// Base glyph (single character, spacing glyph)
    BaseGlyph = 1,
    /// Ligature glyph (multiple character, spacing glyph)
    LigatureGlyph,
    /// Mark glyph (non-spacing combining glyph)
    MarkGlyph,
    /// Component glyph (part of single character, spacing glyph)
    ComponentGlyph,
}
impl From<u16> for GlyphClass {
    fn from(gc: u16) -> Self {
        match gc {
            2 => GlyphClass::LigatureGlyph,
            3 => GlyphClass::MarkGlyph,
            4 => GlyphClass::ComponentGlyph,
            _ => GlyphClass::BaseGlyph,
        }
    }
}
/// A GDEF (Glyph Definition) table
#[derive(Debug, Clone, PartialEq)]
pub struct GDEF {
    /// Glyph class definitions
    pub glyph_class: BTreeMap<GlyphID, GlyphClass>,
    /// Attachment point list
    pub attachment_point_list: BTreeMap<GlyphID, Vec<uint16>>,
    /// Ligature caret list
    pub ligature_caret_list: BTreeMap<GlyphID, Vec<CaretValue>>,
    /// Mark attachment class list
    pub mark_attachment_class: BTreeMap<GlyphID, uint16>,
    /// Mark glyph sets
    pub mark_glyph_sets: Option<Vec<BTreeSet<GlyphID>>>,
    /// Item variation store
    pub item_variation_store: Option<ItemVariationStore>,
}

impl From<MarkGlyphSets> for Vec<BTreeSet<GlyphID>> {
    fn from(mg: MarkGlyphSets) -> Self {
        let mut res = vec![];
        for coverage_offset in mg.coverage.v {
            let bt: BTreeSet<GlyphID> = BTreeSet::from_iter(coverage_offset.link.unwrap().glyphs);
            res.push(bt)
        }
        res
    }
}

impl From<AttachList> for BTreeMap<GlyphID, Vec<uint16>> {
    fn from(al: AttachList) -> Self {
        let mut map = BTreeMap::new();
        for (&g, off) in al
            .coverage
            .link
            .unwrap()
            .glyphs
            .iter()
            .zip(al.attachPoints.v.iter())
        {
            map.insert(g, off.link.as_ref().unwrap().pointIndices.clone());
        }
        map
    }
}

impl From<LigCaretList> for BTreeMap<GlyphID, Vec<CaretValue>> {
    fn from(lcl: LigCaretList) -> Self {
        let mut map = BTreeMap::new();
        for (ligglyph_off, gid) in lcl
            .ligGlyph
            .v
            .iter()
            .zip(lcl.coverage.link.unwrap_or_default().glyphs)
        {
            if let Some(ligglyph) = &ligglyph_off.link {
                let mut v = vec![];
                for caretvalue in &ligglyph.caretValue.v {
                    v.push(caretvalue.link.as_ref().unwrap().clone());
                }
                map.insert(gid, v);
            }
        }
        map
    }
}

impl Deserialize for GDEF {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let core: gdefcore11 = c.de()?;
        let mark_glyph_sets: Option<Vec<BTreeSet<GlyphID>>> = if core.minorVersion > 1 {
            let internal: Offset16<MarkGlyphSets> = c.de()?;
            internal.link.map(|x| x.into())
        } else {
            None
        };
        let ivs = if core.minorVersion > 2 {
            let internal: Offset32<ItemVariationStore> = c.de()?;
            internal.link
        } else {
            None
        };
        Ok(GDEF {
            glyph_class: core.glyphClassDef.link.map_or_else(BTreeMap::new, |gc| {
                gc.classes.iter().map(|(&k, &v)| (k, v.into())).collect()
            }),
            attachment_point_list: core
                .attachList
                .link
                .map_or_else(BTreeMap::new, |m| m.into()),
            ligature_caret_list: core
                .ligCaretList
                .link
                .map_or_else(BTreeMap::new, |m| m.into()),
            mark_attachment_class: core
                .markAttachClassDef
                .link
                .map_or_else(BTreeMap::new, |m| m.classes),
            mark_glyph_sets,
            item_variation_store: ivs,
        })
    }
}

impl Serialize for GDEF {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        if self.item_variation_store.is_some() {
            let gsc: gdefcore13 = self.into();
            gsc.to_bytes(data)
        } else {
            let gsc: gdefcore12 = self.into();
            gsc.to_bytes(data)
        }
    }
}

impl GDEF {
    fn gcd_to_offset(&self) -> Offset16<ClassDef> {
        if self.glyph_class.is_empty() {
            Offset16::to_nothing()
        } else {
            let btree: BTreeMap<GlyphID, u16> = self
                .glyph_class
                .iter()
                .map(|(&k, &v)| (k, v as u16))
                .collect();
            Offset16::to(ClassDef { classes: btree })
        }
    }
    fn apl_to_offset(&self) -> Offset16<AttachList> {
        if self.attachment_point_list.is_empty() {
            Offset16::to_nothing()
        } else {
            let coverage = Coverage {
                glyphs: self.attachment_point_list.keys().copied().collect(),
            };
            let mut points: Vec<Offset16<AttachPoint>> = vec![];
            for glyph in &coverage.glyphs {
                let attachpoint: AttachPoint = AttachPoint {
                    pointIndices: self.attachment_point_list.get(glyph).unwrap().to_vec(),
                };
                points.push(Offset16::to(attachpoint))
            }
            Offset16::to(AttachList {
                coverage: Offset16::to(coverage),
                attachPoints: points.into(),
            })
        }
    }
    fn mac_to_offset(&self) -> Offset16<ClassDef> {
        if self.mark_attachment_class.is_empty() {
            Offset16::to_nothing()
        } else {
            Offset16::to(ClassDef {
                classes: self.mark_attachment_class.clone(),
            })
        }
    }
    fn lcl_to_offset(&self) -> Offset16<LigCaretList> {
        if self.ligature_caret_list.is_empty() {
            Offset16::to_nothing()
        } else {
            let coverage = Coverage {
                glyphs: self.ligature_caret_list.keys().copied().collect(),
            };
            let mut ligglyphs: Vec<Offset16<LigGlyph>> = vec![];
            for glyph in &coverage.glyphs {
                let carets: Vec<Offset16<CaretValue>> = self
                    .ligature_caret_list
                    .get(glyph)
                    .unwrap()
                    .iter()
                    .map(|x| Offset16::to(x.clone()))
                    .collect();
                ligglyphs.push(Offset16::to(LigGlyph {
                    caretValue: carets.into(),
                }));
            }
            Offset16::to(LigCaretList {
                coverage: Offset16::to(coverage),
                ligGlyph: ligglyphs.into(),
            })
        }
    }
    fn mgs_to_offset(&self) -> Offset16<MarkGlyphSets> {
        if let Some(mgs) = &self.mark_glyph_sets {
            let mut coverage_tables: Vec<Offset32<Coverage>> = vec![];
            for gs in mgs {
                let coverage = Coverage {
                    glyphs: gs.iter().copied().collect(),
                };
                coverage_tables.push(Offset32::to(coverage));
            }
            Offset16::to(MarkGlyphSets {
                format: 1,
                coverage: VecOffset32 { v: coverage_tables },
            })
        } else {
            Offset16::to_nothing()
        }
    }
}

impl From<&GDEF> for gdefcore12 {
    fn from(gdef: &GDEF) -> Self {
        gdefcore12 {
            majorVersion: 1,
            minorVersion: 2,
            glyphClassDef: gdef.gcd_to_offset(),
            attachList: gdef.apl_to_offset(),
            ligCaretList: gdef.lcl_to_offset(),
            markAttachClassDef: gdef.mac_to_offset(),
            markGlyphSets: gdef.mgs_to_offset(),
        }
    }
}
impl From<&GDEF> for gdefcore13 {
    fn from(gdef: &GDEF) -> Self {
        gdefcore13 {
            majorVersion: 1,
            minorVersion: 3,
            glyphClassDef: gdef.gcd_to_offset(),
            attachList: gdef.apl_to_offset(),
            ligCaretList: gdef.lcl_to_offset(),
            markAttachClassDef: gdef.mac_to_offset(),
            markGlyphSets: gdef.mgs_to_offset(),
            itemVarStore: gdef
                .item_variation_store
                .clone()
                .map_or_else(Offset32::to_nothing, Offset32::to),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::btreemap;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_gdef_deser_simple() {
        let binary_gdef = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1c, 0x00, 0x02,
            0x00, 0x02, 0x00, 0x05, 0x00, 0x06, 0x00, 0x01, 0x00, 0x1e, 0x00, 0x29, 0x00, 0x03,
            0x00, 0x02, 0x00, 0x04, 0x00, 0x1e, 0x00, 0x1e, 0x00, 0x01, 0x00, 0x1f, 0x00, 0x1f,
            0x00, 0x02, 0x00, 0x23, 0x00, 0x23, 0x00, 0x01, 0x00, 0x29, 0x00, 0x29, 0x00, 0x01,
        ];
        let gdef: GDEF = otspec::de::from_bytes(&binary_gdef).unwrap();
        let expected: GDEF = GDEF {
            glyph_class: btreemap!(
                 5 => GlyphClass::BaseGlyph,
                 6 => GlyphClass::BaseGlyph,
                30 => GlyphClass::MarkGlyph,
                31 => GlyphClass::MarkGlyph,
                32 => GlyphClass::MarkGlyph,
                33 => GlyphClass::MarkGlyph,
                34 => GlyphClass::MarkGlyph,
                35 => GlyphClass::MarkGlyph,
                36 => GlyphClass::MarkGlyph,
                37 => GlyphClass::MarkGlyph,
                38 => GlyphClass::MarkGlyph,
                39 => GlyphClass::MarkGlyph,
                40 => GlyphClass::MarkGlyph,
                41 => GlyphClass::MarkGlyph,
            ),
            attachment_point_list: btreemap!(),
            ligature_caret_list: btreemap!(),
            mark_attachment_class: btreemap!(30 => 1, 31 => 2, 35 => 1, 41 => 1),
            mark_glyph_sets: None,
            item_variation_store: None,
        };
        assert_eq!(gdef, expected);

        let binary = otspec::ser::to_bytes(&expected).unwrap();
        let gdef2: GDEF = otspec::de::from_bytes(&binary).unwrap();
        assert_eq!(gdef2, expected);
    }

    #[test]
    fn test_gdef_deser_ligcaret() {
        let binary_gdef = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x0a,
            0x00, 0x03, 0x00, 0x14, 0x00, 0x22, 0x00, 0x30, 0x00, 0x01, 0x00, 0x03, 0x00, 0xef,
            0x00, 0xf0, 0x02, 0x65, 0x00, 0x02, 0x00, 0x06, 0x00, 0x0a, 0x00, 0x02, 0x00, 0x17,
            0x00, 0x02, 0x00, 0x2e, 0x00, 0x02, 0x00, 0x06, 0x00, 0x0a, 0x00, 0x01, 0x01, 0x90,
            0x00, 0x01, 0x02, 0x58, 0x00, 0x01, 0x00, 0x04, 0x00, 0x01, 0x01, 0xf4,
        ];
        /*
            table GDEF {
                LigatureCaretByPos f_f_l 400 600;
                LigatureCaretByPos c_t 500;
                LigatureCaretByIndex f_f_i 23 46;
            } GDEF;
        */
        let gdef: GDEF = otspec::de::from_bytes(&binary_gdef).unwrap();
        let expected: GDEF = GDEF {
            glyph_class: btreemap!(),
            attachment_point_list: btreemap!(),
            ligature_caret_list: btreemap!(
            239 => vec![CaretValue::Format2 { pointIndex: 23 }, CaretValue::Format2 { pointIndex: 46 }],
            240 => vec![CaretValue::Format1 { coordinate: 400 }, CaretValue::Format1 { coordinate: 600 }],
            613 => vec![CaretValue::Format1 { coordinate: 500 } ],
            ),
            mark_attachment_class: btreemap!(),
            mark_glyph_sets: None,
            item_variation_store: None,
        };
        assert_eq!(gdef, expected);

        let binary = otspec::ser::to_bytes(&expected).unwrap();
        let gdef2: GDEF = otspec::de::from_bytes(&binary).unwrap();
        assert_eq!(gdef2, expected);
    }

    #[test]
    fn test_gdef_deser_attach() {
        let binary_gdef = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x08,
            0x00, 0x02, 0x00, 0x10, 0x00, 0x16, 0x00, 0x01, 0x00, 0x02, 0x00, 0x42, 0x00, 0x43,
            0x00, 0x02, 0x00, 0x04, 0x00, 0x05, 0x00, 0x01, 0x00, 0x08,
        ];
        /*
            table GDEF {
                Attach a 4 5;
                Attach b 8;
            } GDEF;
        */
        let gdef: GDEF = otspec::de::from_bytes(&binary_gdef).unwrap();
        let expected: GDEF = GDEF {
            glyph_class: btreemap!(),
            attachment_point_list: btreemap!(66 => vec![4,5], 67 => vec![8]),
            ligature_caret_list: btreemap!(),
            mark_attachment_class: btreemap!(),
            mark_glyph_sets: None,
            item_variation_store: None,
        };
        assert_eq!(gdef, expected);

        let binary = otspec::ser::to_bytes(&expected).unwrap();
        let gdef2: GDEF = otspec::de::from_bytes(&binary).unwrap();
        assert_eq!(gdef2, expected);
    }
}
