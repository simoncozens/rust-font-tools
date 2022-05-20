use otspec::layout::anchor::Anchor;
use otspec::layout::coverage::Coverage;
use otspec::types::*;
use otspec::Deserializer;
use otspec_macros::tables;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::offsetmanager::OffsetManager;

    #[test]
    fn some_curs_deser() {
        let binary_curs = vec![
            0x00, 0x01, 0x00, 0x16, 0x00, 0x04, 0x00, 0x20, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x26, 0x00, 0x2C, 0x00, 0x32, 0x00, 0x02, 0x00, 0x01, 0x00, 0x42,
            0x00, 0x45, 0x00, 0x00, 0x00, 0x01, 0x00, 0x64, 0x00, 0xC8, 0x00, 0x01, 0xFE, 0xD4,
            0xFE, 0x70, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 0x01, 0x00, 0x03, 0x00, 0x04,
        ];
        let deserialized: CursivePosFormat1 = otspec::de::from_bytes(&binary_curs).unwrap();
        let expected = CursivePosFormat1 {
            posFormat: 1,
            coverage: Offset16::to(Coverage {
                glyphs: vec![66, 67, 68, 69],
            }),
            entryExitRecord: vec![
                EntryExitRecord {
                    entryAnchor: Offset16::to(Anchor {
                        xCoordinate: 100,
                        yCoordinate: 200,
                        anchorPoint: None,
                    }),
                    exitAnchor: Offset16::to_nothing(),
                },
                EntryExitRecord {
                    entryAnchor: Offset16::to_nothing(),
                    exitAnchor: Offset16::to_nothing(),
                },
                EntryExitRecord {
                    entryAnchor: Offset16::to_nothing(),
                    exitAnchor: Offset16::to(Anchor {
                        xCoordinate: -300,
                        yCoordinate: -400,
                        anchorPoint: None,
                    }),
                },
                EntryExitRecord {
                    entryAnchor: Offset16::to(Anchor {
                        xCoordinate: 1,
                        yCoordinate: 2,
                        anchorPoint: None,
                    }),
                    exitAnchor: Offset16::to(Anchor {
                        xCoordinate: 3,
                        yCoordinate: 4,
                        anchorPoint: None,
                    }),
                },
            ],
        };

        println!("{:?}", deserialized.entryExitRecord[0].entryAnchor);
        println!("{:?}", deserialized.entryExitRecord[0].exitAnchor);
        assert!(deserialized.entryExitRecord[0]
            .exitAnchor
            .is_explicitly_zero());
        assert_eq!(deserialized, expected);
        let curs_ser = otspec::ser::to_bytes(&deserialized).unwrap();
        let root = Offset16::to(deserialized);
        let mut mgr = OffsetManager::new(&root);
        mgr.resolve();
        mgr.dump_graph();

        assert_eq!(curs_ser, binary_curs);
    }
}
