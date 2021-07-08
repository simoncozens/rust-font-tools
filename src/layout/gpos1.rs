use crate::layout::coverage::Coverage;
use crate::layout::valuerecord::{ValueRecord, ValueRecords};
use otspec::types::*;
use otspec::DeserializationError;
use otspec::Deserialize;
use otspec::Deserializer;
use otspec::ReaderContext;
use otspec::SerializationError;
use otspec::Serialize;
use otspec::Serializer;

use otspec_macros::tables;
use std::collections::BTreeMap;

tables!(
  SinglePosFormat1 {
    [offset_base]
    uint16 posFormat
    Offset16(Coverage) coverage // Offset to Coverage table, from beginning of positioning subtable
    ValueRecord valueRecord  // Positioning value (includes type)
  }
  SinglePosFormat2 {
    [offset_base]
    uint16 posFormat
    Offset16(Coverage)  coverage  // Offset to Coverage table, from beginning of positioning subtable
    ValueRecords valueRecords // Positioning values (includes type)
  }
);

#[derive(Debug, Clone, PartialEq)]
pub enum SinglePosInternal {
    Format1(SinglePosFormat1),
    Format2(SinglePosFormat2),
}

impl Serialize for SinglePosInternal {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        match self {
            SinglePosInternal::Format1(s) => s.to_bytes(data),
            SinglePosInternal::Format2(s) => s.to_bytes(data),
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// A single positioning subtable.
pub struct SinglePos {
    /// The mapping of input glyph IDs to value records.
    pub mapping: BTreeMap<uint16, ValueRecord>,
}

impl SinglePos {
    fn best_format(&self) -> uint16 {
        let vals: Vec<ValueRecord> = self.mapping.values().map(|x| *x).collect();
        if vals.windows(2).all(|w| w[0] == w[1]) {
            1
        } else {
            2
        }
    }
}

impl Deserialize for SinglePos {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let mut mapping = BTreeMap::new();
        let fmt = c.peek(2)?;
        match fmt {
            [0x00, 0x01] => {
                let sub: SinglePosFormat1 = c.de()?;
                unimplemented!()
            }
            [0x00, 0x02] => {
                let sub: SinglePosFormat2 = c.de()?;
                unimplemented!()
            }
            _ => panic!("Bad single subst format {:?}", fmt),
        }
        Ok(SinglePos { mapping })
    }
}

impl From<&SinglePos> for SinglePosInternal {
    fn from(val: &SinglePos) -> Self {
        let coverage = Coverage {
            glyphs: val.mapping.keys().copied().collect(),
        };
        let format = val.best_format();
        if format == 1 {
            let vr = val.mapping.values().next().unwrap();
            SinglePosInternal::Format1(SinglePosFormat1 {
                posFormat: 1,
                coverage: Offset16::to(coverage),
                valueRecord: *vr,
            })
        } else {
            unimplemented!()
        }
    }
}

impl Serialize for SinglePos {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let ssi: SinglePosInternal = self.into();
        ssi.to_bytes(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use otspec_macros::Serialize;
    use std::iter::FromIterator;

    macro_rules! btreemap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }

    // #[test]
    // fn test_single_subst_1_serde() {
    //     let subst = SinglePos {
    //         mapping: btreemap!(66 => 67, 68 => 69),
    //     };
    //     let binary_subst = vec![
    //         0x00, 0x01, 0x00, 0x06, 0x00, 0x01, 0x00, 0x01, 0x00, 0x02, 0x00, 66, 0x00, 68,
    //     ];
    //     let serialized = otspec::ser::to_bytes(&subst).unwrap();
    //     assert_eq!(serialized, binary_subst);
    //     let de: SinglePos = otspec::de::from_bytes(&binary_subst).unwrap();
    //     assert_eq!(de, subst);
    // }

    // #[test]
    // fn test_single_subst_2_ser() {
    //     let subst = SinglePos {
    //         mapping: btreemap!(34 => 66, 35 => 66, 36  => 66),
    //     };
    //     let binary_subst = vec![
    //         0x00, 0x02, 0x00, 0x0C, 0x00, 0x03, 0x00, 0x42, 0x00, 0x42, 0x00, 0x42, 0x00, 0x01,
    //         0x00, 0x03, 0x00, 0x22, 0x00, 0x23, 0x00, 0x24,
    //     ];
    //     let serialized = otspec::ser::to_bytes(&subst).unwrap();
    //     assert_eq!(serialized, binary_subst);
    //     assert_eq!(
    //         otspec::de::from_bytes::<SinglePos>(&binary_subst).unwrap(),
    //         subst
    //     );
    // }

    // #[test]
    // fn test_single_subst_internal_ser() {
    //     let subst = SinglePos {
    //         mapping: btreemap!(34 => 66, 35 => 66, 36  => 66),
    //     };
    //     let subst: SinglePosInternal = (&subst).into();
    //     let binary_subst = vec![
    //         0x00, 0x02, 0x00, 0x0C, 0x00, 0x03, 0x00, 0x42, 0x00, 0x42, 0x00, 0x42, 0x00, 0x01,
    //         0x00, 0x03, 0x00, 0x22, 0x00, 0x23, 0x00, 0x24,
    //     ];
    //     let serialized = otspec::ser::to_bytes(&subst).unwrap();
    //     assert_eq!(serialized, binary_subst);
    // }

    // #[derive(Serialize, Debug)]
    // pub struct Test {
    //     pub t1: Offset16<SinglePosInternal>,
    // }

    // #[test]
    // fn test_single_subst_internal_ser2() {
    //     let subst = SinglePos {
    //         mapping: btreemap!(34 => 66, 35 => 66, 36  => 66),
    //     };
    //     let subst: SinglePosInternal = (&subst).into();
    //     let test = Test {
    //         t1: Offset16::to(subst),
    //     };

    //     let binary_subst = vec![
    //         0x00, 0x02, 0x00, 0x02, 0x00, 0x0C, 0x00, 0x03, 0x00, 0x42, 0x00, 0x42, 0x00, 0x42,
    //         0x00, 0x01, 0x00, 0x03, 0x00, 0x22, 0x00, 0x23, 0x00, 0x24,
    //     ];
    //     let serialized = otspec::ser::to_bytes(&test).unwrap();
    //     assert_eq!(serialized, binary_subst);
    // }

    // #[derive(Serialize, Debug)]
    // pub struct Test2 {
    //     pub t1: Offset16<SinglePosFormat2>,
    // }

    // #[test]
    // fn test_single_subst_internal_ser3() {
    //     let subst = SinglePos {
    //         mapping: btreemap!(34 => 66, 35 => 66, 36  => 66),
    //     };
    //     let subst: SinglePosInternal = (&subst).into();
    //     if let SinglePosInternal::Format2(s) = subst {
    //         let test = Test2 {
    //             t1: Offset16::to(s),
    //         };

    //         let binary_subst = vec![
    //             0x00, 0x02, 0x00, 0x02, 0x00, 0x0C, 0x00, 0x03, 0x00, 0x42, 0x00, 0x42, 0x00, 0x42,
    //             0x00, 0x01, 0x00, 0x03, 0x00, 0x22, 0x00, 0x23, 0x00, 0x24,
    //         ];
    //         let serialized = otspec::ser::to_bytes(&test).unwrap();
    //         assert_eq!(serialized, binary_subst);
    //     } else {
    //         panic!("Wrong format!");
    //     }
    // }
}
