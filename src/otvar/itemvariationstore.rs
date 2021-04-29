use otspec::types::*;
use otspec::{deserialize_visitor, read_field, read_field_counted, read_remainder};
use otspec_macros::tables;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::Deserializer;
use serde::{Deserialize, Serialize};

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
    pub regionIndexes: Vec<uint16>,
    pub deltaValues: Vec<Vec<int16>>,
}

deserialize_visitor!(
    ItemVariationData,
    ItemVariationDataVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let header = read_field!(seq, ItemVariationDataHeader, "a header");
        let regionIndexCount = header.regionIndexes.len();
        let mut deltaValues = vec![];
        for _ in 0..header.itemCount {
            let mut v: Vec<i16> = Vec::new();
            for col in 0..regionIndexCount {
                if col <= header.shortDeltaCount as usize {
                    v.push(read_field!(seq, i16, "a delta"));
                } else {
                    v.push(read_field!(seq, i8, "a delta").into());
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

pub struct VariationRegionList {
    pub axisCount: uint16,
    pub regionCount: uint16,
    pub variationRegions: Vec<Vec<RegionAxisCoordinates>>,
}
deserialize_visitor!(
    VariationRegionList,
    VariationRegionListVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let axisCount = read_field!(seq, uint16, "an axis count");
        let regionCount = read_field!(seq, uint16, "a region count");
        let mut variationRegions = Vec::with_capacity(regionCount.into());
        for _ in 0..regionCount {
            let v: Vec<RegionAxisCoordinates> =
                read_field_counted!(seq, axisCount, "a VariationRegion record");
            variationRegions.push(v)
        }
        Ok(VariationRegionList {
            axisCount,
            regionCount,
            variationRegions,
        })
    }
);

#[derive(Debug, PartialEq)]
pub struct ItemVariationStore {
    pub format: uint16,
    pub axisCount: uint16,
    pub variationRegions: Vec<Vec<RegionAxisCoordinates>>,
    pub variationData: Vec<ItemVariationData>,
}

deserialize_visitor!(
    ItemVariationStore,
    ItemVariationStoreVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let format = read_field!(seq, uint16, "a header");
        let offset = read_field!(seq, uint32, "an offset");
        let vardatacount = read_field!(seq, uint16, "a count") as usize;
        let variationDataOffsets: Vec<uint32> =
            read_field_counted!(seq, vardatacount, "item variation data offsets");
        let remainder = read_remainder!(seq, "an item variation store");
        let binary_variation_region_list =
            &remainder[offset as usize - (8 + 4 * vardatacount as usize)..];
        let variationRegions: VariationRegionList =
            otspec::de::from_bytes(binary_variation_region_list).map_err(|e| {
                serde::de::Error::custom(format!("Expecting a variation region list: {:?}", e))
            })?;
        let mut variationData = Vec::with_capacity(vardatacount);
        for i in 0..vardatacount {
            let vardata_binary =
                &remainder[variationDataOffsets[i] as usize - (8 + 4 * vardatacount as usize)..];
            variationData.push(otspec::de::from_bytes(vardata_binary).map_err(|e| {
                serde::de::Error::custom(format!("Expecting variation data: {:?}", e))
            })?);
        }
        Ok(ItemVariationStore {
            format,
            axisCount: variationRegions.axisCount,
            variationRegions: variationRegions.variationRegions,
            variationData,
        })
    }
);
