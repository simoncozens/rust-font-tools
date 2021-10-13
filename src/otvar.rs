///! OpenType Variations common tables

/// Item Variation Store (used in `MVAR`, etc.)
mod itemvariationstore;
/// Utilities for Interpolation of Unreferenced Points
pub mod iup;
/// Structs to store locations (user and normalized)
mod locations;
/// Structs for storing packed deltas within a tuple variation store
mod packeddeltas;
/// Structs for storing packed points
mod packedpoints;
/// Headers locating variation data within a tuple variation store
mod tuplevariationheader;
/// Tuple Variation Store
mod tuplevariationstore;

//pub mod instancer;

pub use itemvariationstore::{ItemVariationData, ItemVariationStore, RegionAxisCoordinates};
pub use locations::{support_scalar, Location, NormalizedLocation, VariationModel};
use otspec::types::int16;
pub use packeddeltas::PackedDeltas;
pub use packedpoints::PackedPoints;
pub use tuplevariationheader::{TupleIndexFlags, TupleVariationHeader};
pub use tuplevariationstore::{TupleVariation, TupleVariationStore};

/// Represents either a two-dimensional (`gvar`) or one-dimensional (`cvt`) delta value
#[derive(Debug, PartialEq, Clone)]
pub enum Delta {
    /// A one-dimensional delta (used in the `cvt` table)
    Delta1D(int16),
    /// A two-dimensional delta (used in the `gvar` table)
    Delta2D((int16, int16)),
}
impl Delta {
    /// Assuming that this is a two-dimensional delta, returns the delta as a
    /// X,Y coordinate tuple.
    pub fn get_2d(&self) -> (int16, int16) {
        if let Delta::Delta2D(p) = self {
            *p
        } else {
            panic!("Tried to turn a scalar delta into a coordinate delta");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn otvar_serde_ivd() {
        let binary_ivd = vec![
            0x00, 0x04, 0x00, 0x01, 0x00, 0x01, 0x00, 0x00, 0xFF, 0x38, 0xFF, 0xCE, 0x00, 0x64,
            0x00, 0xC8,
        ];
        let fivd = ItemVariationData {
            region_indexes: vec![0],
            delta_values: vec![vec![-200], vec![-50], vec![100], vec![200]],
        };
        let deserialized: ItemVariationData = otspec::de::from_bytes(&binary_ivd).unwrap();
        assert_eq!(deserialized, fivd);
        let binary_ser = otspec::ser::to_bytes(&fivd).unwrap();
        assert_eq!(binary_ser, binary_ivd);
    }

    #[test]
    fn otvar_serde_ivs() {
        let binary_ivs = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x01, 0x00, 0x00, 0x00, 0x16, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x00, 0x40, 0x00, 0x40, 0x00, 0x00, 0x04, 0x00, 0x01, 0x00, 0x01,
            0x00, 0x00, 0xFF, 0x38, 0xFF, 0xCE, 0x00, 0x64, 0x00, 0xC8,
        ];
        let deserialized: ItemVariationStore = otspec::de::from_bytes(&binary_ivs).unwrap();
        let fivd = ItemVariationData {
            region_indexes: vec![0],
            delta_values: vec![vec![-200], vec![-50], vec![100], vec![200]],
        };
        let fivs = ItemVariationStore {
            format: 1,
            axisCount: 1,
            variationRegions: vec![vec![RegionAxisCoordinates {
                startCoord: 0.0,
                peakCoord: 1.0,
                endCoord: 1.0,
            }]],
            variationData: vec![fivd],
        };
        assert_eq!(deserialized, fivs);

        let binary_ser = otspec::ser::to_bytes(&fivs).unwrap();
        assert_eq!(binary_ser, binary_ivs);
    }
}
