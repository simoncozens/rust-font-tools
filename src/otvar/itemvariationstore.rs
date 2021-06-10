use otspec::{types::*, Deserialize};
use otspec::{DeserializationError, Deserializer, ReaderContext};
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
/// Represents variation data inside an item variation store
pub struct ItemVariationData {
    /// Indices into the IVS's region array.
    pub region_indexes: Vec<uint16>,
    /// A two-dimensional array of delta values.
    ///
    /// "Rows in the table provide sets of deltas for particular target items, and columns correspond to regions of the variation space."
    pub delta_values: Vec<Vec<int16>>,
}

impl Deserialize for ItemVariationData {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let header: ItemVariationDataHeader = c.de()?;
        let region_index_count = header.regionIndexes.len();
        let mut delta_values = vec![];
        for _ in 0..header.itemCount {
            let mut v: Vec<i16> = Vec::new();
            for col in 0..region_index_count {
                if col <= header.shortDeltaCount as usize {
                    let delta: i16 = c.de()?;
                    v.push(delta);
                } else {
                    let delta: i8 = c.de()?;
                    v.push(delta.into());
                }
            }
            delta_values.push(v);
        }
        Ok(ItemVariationData {
            region_indexes: header.regionIndexes,
            delta_values,
        })
    }
}

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

impl Deserialize for VariationRegionList {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let axis_count: u16 = c.de()?;
        let region_count: u16 = c.de()?;
        let mut variation_regions = Vec::with_capacity(region_count.into());
        for _ in 0..region_count {
            let v: Vec<RegionAxisCoordinates> = c.de_counted(axis_count.into())?;
            variation_regions.push(v)
        }
        Ok(VariationRegionList {
            axisCount: axis_count,
            regionCount: region_count,
            variationRegions: variation_regions,
        })
    }
}

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

impl Deserialize for ItemVariationStore {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        c.push();
        let format: uint16 = c.de()?;
        let offset: uint32 = c.de()?;
        let vardatacount: uint16 = c.de()?;
        let variation_data_offsets: Vec<uint32> = c.de_counted(vardatacount.into())?;
        c.ptr = c.top_of_table() + offset as usize;
        let variation_regions: VariationRegionList = c.de()?;
        let mut variation_data = Vec::with_capacity(vardatacount.into());
        for off in variation_data_offsets {
            c.ptr = c.top_of_table() + off as usize;
            variation_data.push(c.de()?);
        }
        Ok(ItemVariationStore {
            format,
            axisCount: variation_regions.axisCount,
            variationRegions: variation_regions.variationRegions,
            variationData: variation_data,
        })
    }
}
