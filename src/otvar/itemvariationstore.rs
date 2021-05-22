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
/// Represents variation data inside an item variation store
pub struct ItemVariationData {
    /// Indices into the IVS's region array.
    pub region_indexes: Vec<uint16>,
    /// A two-dimensional array of delta values.
    ///
    /// "Rows in the table provide sets of deltas for particular target items, and columns correspond to regions of the variation space."
    pub delta_values: Vec<Vec<int16>>,
}

deserialize_visitor!(
    ItemVariationData,
    ItemVariationDataVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let header = read_field!(seq, ItemVariationDataHeader, "a header");
        let region_index_count = header.regionIndexes.len();
        let mut delta_values = vec![];
        for _ in 0..header.itemCount {
            let mut v: Vec<i16> = Vec::new();
            for col in 0..region_index_count {
                if col <= header.shortDeltaCount as usize {
                    v.push(read_field!(seq, i16, "a delta"));
                } else {
                    v.push(read_field!(seq, i8, "a delta").into());
                }
            }
            delta_values.push(v);
        }
        Ok(ItemVariationData {
            region_indexes: header.regionIndexes,
            delta_values,
        })
    }
);

#[allow(non_snake_case, non_camel_case_types)]
/// A set of regions used in a variation
pub struct VariationRegionList {
    /// The number of variation axes for this font. This must be the same number as axisCount in the 'fvar' table.
    pub axisCount: uint16,
    /// The number of variation region tables in the variation region list. Must be less than 32,768.
    pub regionCount: uint16,
    /// Array of variation regions.
    pub variationRegions: Vec<Vec<RegionAxisCoordinates>>,
}
deserialize_visitor!(
    VariationRegionList,
    VariationRegionListVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let axis_count = read_field!(seq, uint16, "an axis count");
        let region_count = read_field!(seq, uint16, "a region count");
        let mut variation_regions = Vec::with_capacity(region_count.into());
        for _ in 0..region_count {
            let v: Vec<RegionAxisCoordinates> =
                read_field_counted!(seq, axis_count, "a VariationRegion record");
            variation_regions.push(v)
        }
        Ok(VariationRegionList {
            axisCount: axis_count,
            regionCount: region_count,
            variationRegions: variation_regions,
        })
    }
);

#[allow(non_snake_case, non_camel_case_types)]
#[derive(Debug, PartialEq)]
/// An item variation store, collecting a set of variation data for scalar values.
pub struct ItemVariationStore {
    /// Format - set to 1
    pub format: uint16,
    /// The number of variation axes in this font.
    pub axisCount: uint16,
    /// The variation regions used in this store.
    pub variationRegions: Vec<Vec<RegionAxisCoordinates>>,
    /// A list of item variation subtables.
    pub variationData: Vec<ItemVariationData>,
}

deserialize_visitor!(
    ItemVariationStore,
    ItemVariationStoreVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let format = read_field!(seq, uint16, "a header");
        let offset = read_field!(seq, uint32, "an offset");
        let vardatacount = read_field!(seq, uint16, "a count") as usize;
        let variation_data_offsets: Vec<uint32> =
            read_field_counted!(seq, vardatacount, "item variation data offsets");
        let remainder = read_remainder!(seq, "an item variation store");
        let binary_variation_region_list =
            &remainder[offset as usize - (8 + 4 * vardatacount as usize)..];
        let variation_regions: VariationRegionList =
            otspec::de::from_bytes(binary_variation_region_list).map_err(|e| {
                serde::de::Error::custom(format!("Expecting a variation region list: {:?}", e))
            })?;
        let mut variation_data = Vec::with_capacity(vardatacount);
        for i in 0..vardatacount {
            let vardata_binary =
                &remainder[variation_data_offsets[i] as usize - (8 + 4 * vardatacount as usize)..];
            variation_data.push(otspec::de::from_bytes(vardata_binary).map_err(|e| {
                serde::de::Error::custom(format!("Expecting variation data: {:?}", e))
            })?);
        }
        Ok(ItemVariationStore {
            format,
            axisCount: variation_regions.axisCount,
            variationRegions: variation_regions.variationRegions,
            variationData: variation_data,
        })
    }
);
