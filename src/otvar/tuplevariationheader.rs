/// Structs for manipulating a Tuple Variation Header
///
/// Tuple Variation Headers are used within a Tuple Variation Store to locate
/// the deltas in the design space. These are low-level data structures,
/// generally used only in serialization/deserialization.
use bitflags::bitflags;
use otspec::types::*;
use otspec::{read_field, read_field_counted, stateful_deserializer};
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};

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
#[derive(Debug, PartialEq)]
pub struct TupleVariationHeader {
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

stateful_deserializer!(
    TupleVariationHeader,
    TupleVariationHeaderDeserializer,
    { axis_count: uint16 },
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<TupleVariationHeader, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut res = TupleVariationHeader {
            peakTuple: None,
            startTuple: None,
            endTuple: None,
            flags: TupleIndexFlags::empty(),
            sharedTupleIndex: 0,
        };
        let _size = read_field!(seq, uint16, "a table size");
        res.flags = read_field!(seq, TupleIndexFlags, "a tuple index");
        res.sharedTupleIndex = res.flags.bits() & TupleIndexFlags::TUPLE_INDEX_MASK.bits();
        if res.flags.contains(TupleIndexFlags::EMBEDDED_PEAK_TUPLE) {
            res.peakTuple = Some(
                (read_field_counted!(seq, self.axis_count, "a peak tuple") as Vec<i16>)
                    .iter()
                    .map(|x| F2DOT14::unpack(*x))
                    .collect(),
            );
        }
        if res.flags.contains(TupleIndexFlags::INTERMEDIATE_REGION) {
            res.startTuple = Some(
                (read_field_counted!(seq, self.axis_count, "a start tuple") as Vec<i16>)
                    .iter()
                    .map(|x| F2DOT14::unpack(*x))
                    .collect(),
            );
            res.endTuple = Some(
                (read_field_counted!(seq, self.axis_count, "an end tuple") as Vec<i16>)
                    .iter()
                    .map(|x| F2DOT14::unpack(*x))
                    .collect(),
            );
        }
        Ok(res)
    }
);
