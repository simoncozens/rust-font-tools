use crate::layout::anchor::Anchor;
use crate::layout::coverage::Coverage;

use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
};
use otspec_macros::tables;

use std::collections::BTreeMap;

tables!(
  CursivePosFormat1 {
    [offset_base]
    uint16 posFormat
    Offset16(Coverage) coverage
    [embed]
    Counted(EntryExitRecord)  entryExitRecord
  }
  EntryExitRecord [embedded] {
    Offset16(Anchor) entryAnchor
    Offset16(Anchor) exitAnchor
  }
);

#[derive(Debug, PartialEq, Clone, Default)]
/// A cursive positioning subtable.
pub struct CursivePos {
    /// The mapping of glyph IDs to entry and exit anchor records.
    pub mapping: BTreeMap<GlyphID, (Option<Anchor>, Option<Anchor>)>,
}

impl Deserialize for CursivePos {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let cursivepos1: CursivePosFormat1 = c.de()?;
        let mut mapping = BTreeMap::new();
        for (input, anchors) in cursivepos1
            .coverage
            .link
            .unwrap()
            .glyphs
            .iter()
            .zip(cursivepos1.entryExitRecord.iter())
        {
            let entry = anchors.entryAnchor.link;
            let exit = anchors.exitAnchor.link;
            mapping.insert(*input, (entry, exit));
        }
        Ok(CursivePos { mapping })
    }
}

impl From<&CursivePos> for CursivePosFormat1 {
    fn from(lookup: &CursivePos) -> Self {
        let coverage = Offset16::to(Coverage {
            glyphs: lookup.mapping.keys().copied().collect(),
        });
        let mut anchors = vec![];
        for right in lookup.mapping.values() {
            let entry_exit = EntryExitRecord {
                entryAnchor: right.0.map_or_else(Offset16::to_nothing, Offset16::to),
                exitAnchor: right.1.map_or_else(Offset16::to_nothing, Offset16::to),
            };
            anchors.push(entry_exit);
        }
        CursivePosFormat1 {
            posFormat: 1,
            coverage,
            entryExitRecord: anchors,
        }
    }
}

impl Serialize for CursivePos {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let i: CursivePosFormat1 = self.into();
        i.to_bytes(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::btreemap;
    use otspec::offsetmanager::OffsetManager;
    use std::iter::FromIterator;

    #[test]
    fn some_curs_de() {
        let binary_curs = vec![
            0x00, 0x01, 0x00, 0x16, 0x00, 0x04, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x26, 0x00, 0x2C, 0x00, 0x32, 0x00, 0x02, 0x00, 0x01, 0x00, 0x42,
            0x00, 0x45, 0x00, 0x00, 0x00, 0x01, 0x00, 0x64, 0x00, 0xC8, 0x00, 0x01, 0xFE, 0xD4,
            0xFE, 0x70, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x03, 0x00, 0x04,
        ];
        let deserialized: CursivePos = otspec::de::from_bytes(&binary_curs).unwrap();
        let expected = CursivePos {
            mapping: btreemap!(
                66 => (Some(Anchor { xCoordinate: 100, yCoordinate: 200, anchorPoint: None }), None),
                67 => (None, None),
                68 => (None, Some(Anchor { xCoordinate: -300, yCoordinate: -400, anchorPoint: None })),
                69 => (Some(Anchor { xCoordinate: 1, yCoordinate: 2, anchorPoint: None }),
                       Some(Anchor { xCoordinate: 3, yCoordinate: 4, anchorPoint: None }))
            ),
        };
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn some_curs_ser() {
        let binary_curs = vec![
            0x00, 0x01, 0x00, 0x16, 0x00, 0x04, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x26, 0x00, 0x2C, 0x00, 0x32, 0x00, 0x02, 0x00, 0x01, 0x00, 0x42,
            0x00, 0x45, 0x00, 0x00, 0x00, 0x01, 0x00, 0x64, 0x00, 0xC8, 0x00, 0x01, 0xFE, 0xD4,
            0xFE, 0x70, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x03, 0x00, 0x04,
        ];
        let curs = CursivePos {
            mapping: btreemap!(
                66 => (Some(Anchor { xCoordinate: 100, yCoordinate: 200, anchorPoint: None }), None),
                67 => (None, None),
                68 => (None, Some(Anchor { xCoordinate: -300, yCoordinate: -400, anchorPoint: None })),
                69 => (Some(Anchor { xCoordinate: 1, yCoordinate: 2, anchorPoint: None }),
                       Some(Anchor { xCoordinate: 3, yCoordinate: 4, anchorPoint: None }))
            ),
        };
        let i: CursivePosFormat1 = (&curs).into();
        println!("{:?}", i);
        let root = Offset16::to(i);
        let mut mgr = OffsetManager::new(&root);
        mgr.resolve();
        mgr.dump_graph();

        assert_eq!(otspec::ser::to_bytes(&curs).unwrap(), binary_curs);
    }
}
