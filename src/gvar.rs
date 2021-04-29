use otspec::de::CountedDeserializer;
use otspec::de::Deserializer as OTDeserializer;
use otspec::{read_field, read_field_counted, read_remainder, stateful_deserializer};
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
use std::convert::TryInto;
extern crate otspec;
use crate::otvar::*;
use otspec::types::*;
use otspec_macros::tables;

tables!( gvarcore {
    uint16  majorVersion
    uint16  minorVersion
    uint16  axisCount
    uint16  sharedTupleCount
    u32  sharedTuplesOffset
    uint16  glyphCount
    uint16  flags
    u32  glyphVariationDataArrayOffset
}
);

#[derive(Debug, PartialEq)]
struct GlyphVariationData {}

#[derive(Debug, PartialEq)]
pub struct gvar {
    variations: Vec<Option<GlyphVariationData>>,
}

stateful_deserializer!(
    gvar,
    GvarDeserializer,
    { point_counts: Vec<u16> },
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let core = read_field!(seq, gvarcore, "a gvar table header");
        let dataOffsets: Vec<u32> = if core.flags & 0x1 == 0 {
            // u16 offsets, need doubling
            let u16_and_halved: Vec<u16> =
                read_field_counted!(seq, core.glyphCount + 1, "a glyphVariationDataOffset");
            u16_and_halved.iter().map(|x| (x * 2).into()).collect()
        } else {
            read_field_counted!(seq, core.glyphCount + 1, "a glyphVariationDataOffset")
        };
        println!("Offsets {:?}", dataOffsets);
        let remainder = read_remainder!(seq, "a gvar table");
        let mut offset_base: usize =
            20 + (core.glyphCount as usize + 1) * (if core.flags & 0x1 == 0 { 2 } else { 4 });
        println!("Offset base: {:?}", offset_base);
        println!("Remainder: {:?}", remainder);
        let axis_count = core.axisCount as usize;

        /* Shared tuples */
        let mut shared_tuples: Vec<Tuple> = Vec::with_capacity(core.sharedTupleCount as usize);
        let mut shared_tuple_start = (core.sharedTuplesOffset as usize) - offset_base;
        let shared_tuple_end =
            shared_tuple_start + (core.sharedTupleCount * core.axisCount * 2) as usize;
        while shared_tuple_start < shared_tuple_end {
            println!("Start {:?}", shared_tuple_start);
            let bytes = &remainder[shared_tuple_start..shared_tuple_start + 2 * axis_count];
            let mut de = OTDeserializer::from_bytes(bytes);
            println!("Trying to deserialize shared tuple array {:?}", bytes);
            let cs: CountedDeserializer<i16> = CountedDeserializer::with_len(axis_count);
            let tuple: Vec<f32> = cs
                .deserialize(&mut de)
                .map_err(|_| serde::de::Error::custom("Expecting a tuple"))?
                .iter()
                .map(|i| *i as f32 / 16384.0)
                .collect();
            println!("Tuple {:?}", tuple);
            shared_tuple_start += 2 * axis_count;
            shared_tuples.push(tuple);
        }

        /* Glyph variation data */
        let mut glyphVariations = vec![];
        for i in 0..(core.glyphCount) {
            println!("Reading data for glyph {:?}", i);
            let offset: usize = (dataOffsets[i as usize] + (core.glyphVariationDataArrayOffset)
                - offset_base as u32)
                .try_into()
                .unwrap();
            let next_offset: usize = (dataOffsets[(i + 1) as usize]
                + (core.glyphVariationDataArrayOffset)
                - offset_base as u32)
                .try_into()
                .unwrap();
            let length = next_offset - offset;
            let bytes = &remainder[offset..];
            if length == 0 {
                glyphVariations.push(None);
            } else {
                let mut de = otspec::de::Deserializer::from_bytes(bytes);
                let cs = TupleVariationStoreDeserializer {
                    axis_count: core.axisCount,
                    point_count: self.point_counts[i as usize],
                    is_gvar: true,
                };
                let tvh = cs.deserialize(&mut de).unwrap();
                println!("TVH {:?}", tvh);
            }
        }

        Ok(gvar {
            variations: glyphVariations,
        })
    }
);

pub fn from_bytes(s: &[u8], point_counts: Vec<u16>) -> otspec::error::Result<gvar> {
    let mut deserializer = otspec::de::Deserializer::from_bytes(s);
    let cs = GvarDeserializer { point_counts };
    cs.deserialize(&mut deserializer)
}

#[cfg(test)]
mod tests {
    use crate::gvar;

    #[test]
    fn gvar_de() {
        let binary_gvar = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x02, 0x00, 0x02, 0x00, 0x00, 0x00, 0x1e, 0x00, 0x04,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x26, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x0d,
            0x00, 0x24, 0x40, 0x00, 0x00, 0x00, 0x00, 0x00, 0x40, 0x00, 0x80, 0x02, 0x00, 0x0c,
            0x00, 0x06, 0x00, 0x00, 0x00, 0x06, 0x00, 0x01, 0x00, 0x86, 0x02, 0xd2, 0xd2, 0x2e,
            0x83, 0x02, 0x52, 0xae, 0xf7, 0x83, 0x86, 0x00, 0x80, 0x03, 0x00, 0x14, 0x00, 0x0a,
            0x20, 0x00, 0x00, 0x07, 0x00, 0x01, 0x00, 0x07, 0x80, 0x00, 0x40, 0x00, 0x40, 0x00,
            0x00, 0x02, 0x01, 0x01, 0x02, 0x01, 0x26, 0xda, 0x01, 0x83, 0x7d, 0x03, 0x26, 0x26,
            0xda, 0xda, 0x83, 0x87, 0x03, 0x13, 0x13, 0xed, 0xed, 0x83, 0x87, 0x00,
        ];
        let deserialized: gvar::gvar = gvar::from_bytes(&binary_gvar, vec![10, 0, 7, 8]).unwrap();
        // assert_eq!(deserialized.majorVersion, 1);
        // assert_eq!(deserialized.minorVersion, 0);
        // assert_eq!(deserialized.axisCount, 2);
        // assert_eq!(deserialized.sharedTuples.len(), 0);
        // let serialized = ser::to_bytes(&deserialized).unwrap();
        // assert_eq!(serialized, binary_post);
        assert!(false);
    }
}
