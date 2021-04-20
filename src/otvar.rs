use otspec::de::CountedDeserializer;
use otspec::ser;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::Deserializer;
use serde::Serializer;
use serde::{Deserialize, Serialize};
use std::convert::TryInto;
extern crate otspec;
use otspec::deserialize_visitor;
use otspec::types::*;
use otspec_macros::tables;

tables!(
    RegionAxisCoordinates {
        F2DOT14	startCoord
        F2DOT14	peakCoord
        F2DOT14	endCoord
    }
    ItemVariationDataHeader {
        uint16	itemCount
        uint16	shortDeltaCount
        Counted(uint16) regionIndexes
    }

);

#[derive(Debug, PartialEq)]
pub struct ItemVariationData {
    regionIndexes: Vec<uint16>,
    deltaValues: Vec<Vec<int16>>,
}

deserialize_visitor!(
    ItemVariationData,
    ItemVariationDataVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let header = seq
            .next_element::<ItemVariationDataHeader>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a header"))?;
        let regionIndexCount = header.regionIndexes.len();
        let mut deltaValues = vec![];
        for _ in 0..header.itemCount {
            let mut v: Vec<i16> = Vec::new();
            for col in 0..regionIndexCount {
                if col <= header.shortDeltaCount as usize {
                    v.push(
                        seq.next_element::<i16>()?
                            .ok_or_else(|| serde::de::Error::custom("Expecting a delta"))?
                            as i16,
                    );
                } else {
                    v.push(
                        seq.next_element::<i8>()?
                            .ok_or_else(|| serde::de::Error::custom("Expecting a delta"))?
                            as i16,
                    );
                }
            }
            deltaValues.push(v);
        }
        Ok(ItemVariationData {
            regionIndexes: header.regionIndexes,
            deltaValues,
        })
    }
);

// #[derive(Debug, PartialEq, Serialize)]
// pub struct ItemVariationStore {
//     format: uint16,
//     axisCount: uint16,
//     regionCount: uint16,
//     variationRegions: Vec<Vec<RegionAxisCoordinates>>,
//     variationData: Vec<ItemVariationData>,
// }

#[cfg(test)]
mod tests {
    use crate::otvar;

    #[test]
    fn otvar_de() {
        let binary_ivd = vec![
            0x00, 0x04, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0x38, 0xFF, 0xCE, 0x00, 0x64,
            0x00, 0xC8,
        ];
        let fivd = otvar::ItemVariationData {
            regionIndexes: vec![0],
            deltaValues: vec![vec![-200], vec![-50], vec![100], vec![200]],
        };
        let deserialized: otvar::ItemVariationData = otspec::de::from_bytes(&binary_ivd).unwrap();
        assert_eq!(deserialized, fivd);
    }
}
