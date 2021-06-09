/// Structs for manipulating a Tuple Variation Header
///
/// Tuple Variation Headers are used within a Tuple Variation Store to locate
/// the deltas in the design space. These are low-level data structures,
/// generally used only in serialization/deserialization.
use bitflags::bitflags;
use otspec::{types::*, Deserialize};
use otspec::{
    DeserializationError, Deserializer, ReaderContext, SerializationError, Serialize, Serializer,
};
use otspec_macros::{Deserialize, Serialize};

use super::TupleVariation;

bitflags! {
    /// Flags used internally to a tuple variation header
    #[derive(Serialize, Deserialize)]
    pub struct TupleIndexFlags: u16 {
        /// This header contains its own peak tuple (rather than a shared tuple)
        const EMBEDDED_PEAK_TUPLE = 0x8000;
        /// This header contains a start tuple and end tuple
        const INTERMEDIATE_REGION = 0x4000;
        /// This header has its own set of point numbers (rather than shared points)
        const PRIVATE_POINT_NUMBERS = 0x2000;
        /// Masks off flags to reveal the shared tuple index
        const TUPLE_INDEX_MASK = 0x0FFF;
    }
}

/// A tuple variation header
///
/// Used to locate a set of deltas within the design space.
#[allow(non_snake_case, non_camel_case_types)]
#[derive(Debug, PartialEq, Clone)]
pub struct TupleVariationHeader {
    /// Size in bytes of the serialized data (the data *after* the header/tuples
    // including the private points but *not* including the shared points)
    pub size: uint16,
    /// Flags (including the shared tuple index)
    pub flags: TupleIndexFlags,
    /// The index into the Tuple Variation Store's shared tuple array to be used
    /// if this header does not define its own peak tuple.
    pub sharedTupleIndex: uint16,
    /// The location at which this set of deltas has maximum effect.
    pub peakTuple: Option<Vec<f32>>,
    /// The start location for this delta region.
    pub startTuple: Option<Vec<f32>>,
    /// The end location for this delta region.
    pub endTuple: Option<Vec<f32>>,
}

impl TupleVariationHeader {
    pub fn from_bytes(
        c: &mut ReaderContext,
        axis_count: uint16,
    ) -> Result<Self, DeserializationError> {
        let mut res = TupleVariationHeader {
            size: 0,
            peakTuple: None,
            startTuple: None,
            endTuple: None,
            flags: TupleIndexFlags::empty(),
            sharedTupleIndex: 0,
        };
        res.size = c.de()?;
        res.flags = c.de()?;
        res.sharedTupleIndex = res.flags.bits() & TupleIndexFlags::TUPLE_INDEX_MASK.bits();
        if res.flags.contains(TupleIndexFlags::EMBEDDED_PEAK_TUPLE) {
            res.peakTuple = Some(
                (c.de_counted(axis_count.into())? as Vec<i16>)
                    .iter()
                    .map(|x| F2DOT14::from_packed(*x).into())
                    .collect(),
            );
        }
        if res.flags.contains(TupleIndexFlags::INTERMEDIATE_REGION) {
            res.startTuple = Some(
                (c.de_counted(axis_count.into())? as Vec<i16>)
                    .iter()
                    .map(|x| F2DOT14::from_packed(*x).into())
                    .collect(),
            );
            res.endTuple = Some(
                (c.de_counted(axis_count.into())? as Vec<i16>)
                    .iter()
                    .map(|x| F2DOT14::from_packed(*x).into())
                    .collect(),
            );
        }
        Ok(res)
    }
}

// In order to be stateless, this serializer relies on the `.size` and `.flags`
// fields being set correctly upstream in the TVS serializer, so that it is
// handed a struct "ready to go".
impl Serialize for TupleVariationHeader {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        data.put(self.size)?;
        data.put(self.sharedTupleIndex | self.flags.bits())?;
        if self.flags.contains(TupleIndexFlags::EMBEDDED_PEAK_TUPLE) {
            if self.peakTuple.is_some() {
                for coord in self.peakTuple.as_ref().unwrap() {
                    F2DOT14::from(*coord).to_bytes(data)?;
                }
            } else {
                panic!("EMBEDDED_PEAK_TUPLE was set, but there wasn't one.");
            }
        }
        if self.flags.contains(TupleIndexFlags::INTERMEDIATE_REGION) {
            if self.startTuple.is_some() {
                for coord in self.startTuple.as_ref().unwrap() {
                    F2DOT14::from(*coord).to_bytes(data)?;
                }
            } else {
                panic!("INTERMEDIATE_REGION was set, but there was no start tuple.");
            }
            if self.endTuple.is_some() {
                for coord in self.endTuple.as_ref().unwrap() {
                    F2DOT14::from(*coord).to_bytes(data)?;
                }
            } else {
                panic!("INTERMEDIATE_REGION was set, but there was no end tuple.");
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::otvar::{TupleIndexFlags, TupleVariationHeader};
    use otspec::ReaderContext;

    #[test]
    fn test_tvh_serde() {
        let tvh = TupleVariationHeader {
            size: 33,
            flags: TupleIndexFlags::EMBEDDED_PEAK_TUPLE,
            sharedTupleIndex: 0,
            peakTuple: Some(vec![0.5]),
            startTuple: None,
            endTuple: None,
        };
        let serialized = otspec::ser::to_bytes(&tvh).unwrap();

        let deserialized =
            TupleVariationHeader::from_bytes(&mut ReaderContext::new(serialized), 1).unwrap();

        assert_eq!(deserialized, tvh);
    }

    #[test]
    fn test_tvh_deser() {
        let binary_tvh: Vec<u8> = vec![0, 33, 128, 0, 32, 0];
        let deserialized =
            TupleVariationHeader::from_bytes(&mut ReaderContext::new(binary_tvh.clone()), 1)
                .unwrap();
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_tvh);
    }
}
