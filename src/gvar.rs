use otspec::de::CountedDeserializer;
use otspec::de::Deserializer as OTDeserializer;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Serialize};
use serde::{Deserializer, Serializer};
extern crate otspec;
use otspec::types::*;
use otspec_macros::tables;

tables!( GvarHeader {
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
    majorVersion: uint16,
    minorVersion: uint16,
    axisCount: uint16,
    sharedTuples: Vec<Tuple>,
    glyphVariations: Vec<GlyphVariationData>,
}

struct GvarVisitor {
    _phantom: std::marker::PhantomData<gvar>,
}

impl GvarVisitor {
    fn new() -> Self {
        GvarVisitor {
            _phantom: std::marker::PhantomData,
        }
    }
}
impl<'de> Visitor<'de> for GvarVisitor {
    type Value = gvar;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "A sequence of values")
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let core = seq
            .next_element::<GvarHeader>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a gvar table"))?;
        let mut dataOffsets: Vec<u32> = Vec::new();
        for _i in 0..(core.glyphCount + 1) {
            if core.flags & 0x1 == 0 {
                let offset = seq.next_element::<u16>()?.ok_or_else(|| {
                    serde::de::Error::custom("Expecting a glyphVariationDataOffset")
                })?;
                dataOffsets.push((offset * 2).into());
            } else {
                let offset = seq.next_element::<u32>()?.ok_or_else(|| {
                    serde::de::Error::custom("Expecting a glyphVariationDataOffset")
                })?;
                dataOffsets.push(offset);
            }
        }
        let remainder = seq
            .next_element::<Vec<u8>>()?
            .ok_or_else(|| serde::de::Error::custom("Expecting a gvar table"))?;
        let offset_base: usize = 20;
        let axis_count = core.axisCount as usize;

        /* Shared tuples */
        let mut shared_tuples: Vec<Tuple> = Vec::with_capacity(core.sharedTupleCount as usize);
        let mut shared_tuple_start = (core.sharedTuplesOffset as usize) - offset_base;
        let shared_tuple_end =
            shared_tuple_start + (core.sharedTupleCount * core.axisCount * 2) as usize;
        // while shared_tuple_start < shared_tuple_end {
        //     let bytes = &remainder[shared_tuple_start..shared_tuple_start + 2 * axis_count];
        //     let mut de = OTDeserializer::from_bytes(bytes);
        //     println!("Trying to deserialize shared tuple array {:?}", bytes);
        //     let cs: CountedDeserializer<i16> = CountedDeserializer::with_len(axis_count);
        //     let tuple: Vec<f32> = cs
        //         .deserialize(de)
        //         .map_err(|_| serde::de::Error::custom("Expecting a tuple"))?
        //         .iter()
        //         .map(|i| *i as f32 / 16384.0)
        //         .collect();
        //     println!("Tuple {:?}", tuple);
        //     shared_tuple_start += 2 * axis_count;
        //     shared_tuples.push(tuple);
        // }

        /* Glyph offsets */
        for i in 0..(core.glyphCount + 1) {
            println!("Glyph {:?} offset {:?}", i, dataOffsets[i as usize]);
            let offset = dataOffsets[i as usize] + (core.glyphVariationDataArrayOffset) - 20;
            // let bytes =
        }

        Ok(gvar {
            majorVersion: core.majorVersion,
            minorVersion: core.minorVersion,
            axisCount: core.axisCount,
            sharedTuples: shared_tuples,
            glyphVariations: vec![],
        })
    }
}

impl<'de> Deserialize<'de> for gvar {
    fn deserialize<D>(d: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        d.deserialize_seq(GvarVisitor::new())
    }
}

#[cfg(test)]
mod tests {
    use crate::gvar;
    use otspec::de;
    use otspec::ser;

    #[test]
    fn gvar_de() {
        let binary_gvar = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x02,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x1a, 0x00, 0x00, 0x00, 0x00, 0x00, 0x16, 0x80, 0x01,
            0x00, 0x0a, 0x00, 0x21, 0x80, 0x00, 0x20, 0x00, 0x00, 0x06, 0x35, 0x30, 0x49, 0x45,
            0x00, 0x10, 0x74, 0x40, 0x00, 0x84, 0x03, 0x4b, 0x2e, 0x3d, 0x00, 0x40, 0x01, 0x20,
            0x81, 0x0a, 0xf8, 0x03, 0x03, 0xf8, 0xf8, 0x1c, 0x1c, 0xf8, 0x3b, 0x3b, 0x15, 0x83,
        ];
        let deserialized: gvar::gvar = otspec::de::from_bytes(&binary_gvar).unwrap();
        assert_eq!(deserialized.majorVersion, 1);
        assert_eq!(deserialized.minorVersion, 0);
        assert_eq!(deserialized.axisCount, 1);
        assert_eq!(deserialized.sharedTuples.len(), 0);
        // let serialized = ser::to_bytes(&deserialized).unwrap();
        // assert_eq!(serialized, binary_post);
    }
}
