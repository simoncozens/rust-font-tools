use bitflags::bitflags;
use otspec::de::CountedDeserializer;
use otspec::types::*;
use otspec::{read_field, read_field_counted, stateful_deserializer};
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};

bitflags! {
    #[derive(Serialize, Deserialize)]
    pub struct TupleIndexFlags: u16 {
        const EMBEDDED_PEAK_TUPLE = 0x8000;
        const INTERMEDIATE_REGION = 0x4000;
        const PRIVATE_POINT_NUMBERS = 0x2000;
        const TUPLE_INDEX_MASK = 0x0FFF;
    }
}

#[derive(Debug, PartialEq)]
pub struct TupleVariationHeader {
    pub size: uint16,
    pub flags: TupleIndexFlags,
    pub sharedTupleIndex: uint16,
    pub peakTuple: Option<Vec<f32>>,
    pub startTuple: Option<Vec<f32>>,
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
            size: 0,
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
