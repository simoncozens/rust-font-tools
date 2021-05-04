/// Structs for manipulating a Tuple Variation Header
///
/// Tuple Variation Headers are used within a Tuple Variation Store to locate
/// the deltas in the design space. These are low-level data structures,
/// generally used only in serialization/deserialization.
use bitflags::bitflags;
use otspec::types::*;
use otspec::{read_field, read_field_counted, stateful_deserializer};
use serde::de::{DeserializeSeed, SeqAccess, Visitor};
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize, Serializer};

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

stateful_deserializer!(
    TupleVariationHeader,
    TupleVariationHeaderDeserializer,
    { axis_count: uint16 },
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<TupleVariationHeader, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let mut res = TupleVariationHeader {
            size: 0,
            peakTuple: None,
            startTuple: None,
            endTuple: None,
            flags: TupleIndexFlags::empty(),
            sharedTupleIndex: 0,
        };
        res.size = read_field!(seq, uint16, "a table size");
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

// In order to be stateless, this serializer relies on the `.size` and `.flags`
// fields being set correctly upstream in the TVS serializer, so that it is
// handed a struct "ready to go".
impl Serialize for TupleVariationHeader {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        seq.serialize_element::<uint16>(&self.size)?;
        seq.serialize_element::<uint16>(&(self.sharedTupleIndex | self.flags.bits()))?;
        if self.flags.contains(TupleIndexFlags::EMBEDDED_PEAK_TUPLE) {
            if self.peakTuple.is_some() {
                seq.serialize_element(
                    &self
                        .peakTuple
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|x| otspec::ser::to_bytes(&F2DOT14::pack(*x)).unwrap())
                        .flatten()
                        .collect::<Vec<u8>>(),
                )?;
            } else {
                panic!("EMBEDDED_PEAK_TUPLE was set, but there wasn't one.");
            }
        }
        if self.flags.contains(TupleIndexFlags::INTERMEDIATE_REGION) {
            if self.startTuple.is_some() {
                // This is messy. Ideally we should be able to serialize Tuple
                // directly, but it's a type alias. Maybe investigate another way
                // to call a serializer on it?
                seq.serialize_element(
                    &self
                        .startTuple
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|x| otspec::ser::to_bytes(&F2DOT14::pack(*x)).unwrap())
                        .flatten()
                        .collect::<Vec<u8>>(),
                )?
            } else {
                panic!("INTERMEDIATE_REGION was set, but there was no start tuple.");
            }
            if self.endTuple.is_some() {
                seq.serialize_element(
                    &self
                        .endTuple
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|x| otspec::ser::to_bytes(&F2DOT14::pack(*x)).unwrap())
                        .flatten()
                        .collect::<Vec<u8>>(),
                )?
            } else {
                panic!("INTERMEDIATE_REGION was set, but there was no end tuple.");
            }
        }
        seq.end()
    }
}

#[cfg(test)]
mod tests {
    use crate::otvar::tuplevariationheader::TupleVariationHeaderDeserializer;
    use crate::otvar::{TupleIndexFlags, TupleVariationHeader};
    use serde::de::DeserializeSeed;

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
        let mut de = otspec::de::Deserializer::from_bytes(&serialized);
        let cs = TupleVariationHeaderDeserializer { axis_count: 1 };
        let deserialized = cs.deserialize(&mut de).unwrap();

        assert_eq!(deserialized, tvh);
    }

    #[test]
    fn test_tvh_deser() {
        let binary_tvh: Vec<u8> = vec![0, 33, 128, 0, 32, 0];
        let mut de = otspec::de::Deserializer::from_bytes(&binary_tvh);
        let cs = TupleVariationHeaderDeserializer { axis_count: 1 };
        let deserialized = cs.deserialize(&mut de).unwrap();
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_tvh);
    }
}
