use crate::otvar::*;
use counter::Counter;
use otspec::de::CountedDeserializer;
use otspec::de::Deserializer as OTDeserializer;
use otspec::types::*;
use otspec::{read_field, read_field_counted, read_remainder, stateful_deserializer};
use otspec_macros::tables;
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::HashMap;
use std::convert::TryInto;

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

/// How a glyph's points vary at one region of the design space.
///
/// (This is the user-friendly version of what is serialized as a TupleVariation)
#[derive(Debug, PartialEq)]
pub struct DeltaSet {
    pub peak: Tuple,
    pub start: Tuple,
    pub end: Tuple,
    pub deltas: Vec<(i16, i16)>,
}

impl DeltaSet {
    fn to_tuple_variation(
        &self,
        shared_tuples: &[Vec<u8>],
        has_private_points: bool,
    ) -> TupleVariation {
        let serialized_peak = &self
            .peak
            .iter()
            .map(|x| otspec::ser::to_bytes(&F2DOT14::pack(*x)).unwrap())
            .flatten()
            .collect::<Vec<u8>>();
        let index = shared_tuples.iter().position(|t| t == serialized_peak);
        let mut flags = TupleIndexFlags::empty();
        let shared_tuple_index: uint16;
        if let Some(sti) = index {
            shared_tuple_index = sti as u16;
        } else {
            shared_tuple_index = 0;
            flags |= TupleIndexFlags::EMBEDDED_PEAK_TUPLE;
        }
        // This check is wrong. See Python compileIntermediateCoord
        if self.peak != self.start || self.peak != self.end {
            flags |= TupleIndexFlags::INTERMEDIATE_REGION;
        }

        if has_private_points {
            flags |= TupleIndexFlags::PRIVATE_POINT_NUMBERS;
        }

        let tvh = TupleVariationHeader {
            size: 0, // This will be filled in when serializing the TVS
            flags,
            sharedTupleIndex: shared_tuple_index,
            peakTuple: if flags.contains(TupleIndexFlags::EMBEDDED_PEAK_TUPLE) {
                Some(self.peak.clone())
            } else {
                None
            },
            startTuple: if flags.contains(TupleIndexFlags::INTERMEDIATE_REGION) {
                Some(self.start.clone())
            } else {
                None
            },
            endTuple: if flags.contains(TupleIndexFlags::INTERMEDIATE_REGION) {
                Some(self.end.clone())
            } else {
                None
            },
        };

        let deltas = self
            .deltas
            .iter()
            .map(|(x, y)| Some(Delta::Delta2D((*x, *y))))
            .collect();
        // Do IUP optimization here.
        TupleVariation(tvh, deltas)
    }
}

#[derive(Debug, PartialEq)]
pub struct GlyphVariationData {
    pub deltasets: Vec<DeltaSet>,
}

#[derive(Debug, PartialEq)]
pub struct gvar {
    variations: Vec<Option<GlyphVariationData>>,
}

stateful_deserializer!(
    gvar,
    GvarDeserializer,
    { coords_and_ends: Vec<(Vec<(int16,int16)>,Vec<usize>)> },
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
        // println!("Offsets {:?}", dataOffsets);
        let remainder = read_remainder!(seq, "a gvar table");
        let offset_base: usize =
            20 + (core.glyphCount as usize + 1) * (if core.flags & 0x1 == 0 { 2 } else { 4 });
        // println!("Remainder: {:?}", remainder);
        let axis_count = core.axisCount as usize;

        /* Shared tuples */
        let mut shared_tuples: Vec<Tuple> = Vec::with_capacity(core.sharedTupleCount as usize);
        let mut shared_tuple_start = (core.sharedTuplesOffset as usize) - offset_base;
        let shared_tuple_end =
            shared_tuple_start + (core.sharedTupleCount * core.axisCount * 2) as usize;
        while shared_tuple_start < shared_tuple_end {
            // println!("Start {:?}", shared_tuple_start);
            let bytes = &remainder[shared_tuple_start..shared_tuple_start + 2 * axis_count];
            let mut de = OTDeserializer::from_bytes(bytes);
            // println!("Trying to deserialize shared tuple array {:?}", bytes);
            let cs: CountedDeserializer<i16> = CountedDeserializer::with_len(axis_count);
            let tuple: Vec<f32> = cs
                .deserialize(&mut de)
                .map_err(|_| serde::de::Error::custom("Expecting a tuple"))?
                .iter()
                .map(|i| *i as f32 / 16384.0)
                .collect();
            // println!("Tuple {:?}", tuple);
            shared_tuple_start += 2 * axis_count;
            shared_tuples.push(tuple);
        }

        /* Glyph variation data */
        let mut glyphVariations = vec![];
        for i in 0..(core.glyphCount as usize) {
            // println!("Reading data for glyph {:?}", i);
            let offset: usize = (dataOffsets[i] + (core.glyphVariationDataArrayOffset)
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
                let mut deltasets: Vec<DeltaSet> = vec![];
                let mut de = otspec::de::Deserializer::from_bytes(bytes);
                let cs = TupleVariationStoreDeserializer {
                    axis_count: core.axisCount,
                    point_count: self.coords_and_ends[i].0.len() as u16,
                    is_gvar: true,
                };
                let tvs = cs.deserialize(&mut de).unwrap();
                // println!("TVS {:?}", tvs);
                for tvh in tvs.0 {
                    let deltas = tvh.iup_delta(&self.coords_and_ends[i].0, &self.coords_and_ends[i].1);
                    let index = tvh.0.sharedTupleIndex as usize;
                    if index > shared_tuples.len() {
                        return Err(serde::de::Error::custom(format!("Invalid shared tuple index {:}", index)))
                    }
                    let peak_tuple = tvh
                        .0
                        .peakTuple
                        .unwrap_or_else(|| shared_tuples[index].clone());
                    let start_tuple = tvh.0.startTuple.unwrap_or_else(|| peak_tuple.clone());
                    let end_tuple = tvh.0.endTuple.unwrap_or_else(|| peak_tuple.clone());
                    deltasets.push(DeltaSet {
                        deltas,
                        peak: peak_tuple,
                        end: end_tuple,
                        start: start_tuple,
                    })
                }
                glyphVariations.push(Some(GlyphVariationData { deltasets }));
            }
        }

        Ok(gvar {
            variations: glyphVariations,
        })
    }
);

impl gvar {
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out: Vec<u8> = vec![];
        // Determine all the shared tuples.
        let mut shared_tuple_counter: Counter<Vec<u8>> = Counter::new();
        let mut axisCount: uint16 = 0;
        for var in self.variations.iter().flatten() {
            for ds in &var.deltasets {
                axisCount = ds.peak.len() as uint16;
                // println!("Peak: {:?}", ds.peak);
                shared_tuple_counter[&ds
                    .peak
                    .iter()
                    .map(|x| otspec::ser::to_bytes(&F2DOT14::pack(*x)).unwrap())
                    .flatten()
                    .collect::<Vec<u8>>()] += 1;
            }
        }
        shared_tuple_counter.retain(|_, &mut v| v > 1);
        let most_common_tuples: Vec<(Vec<u8>, usize)> = shared_tuple_counter.most_common();
        if most_common_tuples.is_empty() {
            panic!("Some more sensible error checking here for null case");
        }
        let sharedTupleCount = most_common_tuples.len() as u16;
        let flags = 0; // XXX

        let mut glyphVariationDataOffsets: Vec<u8> = vec![];

        // println!("Most common tuples: {:?}", most_common_tuples);
        let mut shared_tuples = vec![];
        let mut serialized_tuples = vec![];
        let mut serialized_tvs = vec![];
        for (a, _) in most_common_tuples {
            serialized_tuples.extend(otspec::ser::to_bytes(&a).unwrap());
            shared_tuples.push(a);
        }
        // Now we need a bunch of TVSes
        for var in self.variations.iter() {
            // Data offset
            if flags != 0 {
                glyphVariationDataOffsets
                    .extend(&otspec::ser::to_bytes(&(serialized_tvs.len() as u32)).unwrap());
            } else {
                glyphVariationDataOffsets
                    .extend(&otspec::ser::to_bytes(&(serialized_tvs.len() as u16 / 2)).unwrap());
            }

            if let Some(var) = var {
                let tvs = TupleVariationStore(
                    var.deltasets
                        .iter()
                        .map(|ds| ds.to_tuple_variation(&shared_tuples, false))
                        .collect(),
                );
                serialized_tvs.extend(otspec::ser::to_bytes(&tvs).unwrap());
                // Add a byte of padding
                if (serialized_tvs.len() % 2) != 0 {
                    serialized_tvs.push(0);
                }
            }
        }
        // Final data offset
        if flags != 0 {
            glyphVariationDataOffsets
                .extend(&otspec::ser::to_bytes(&(serialized_tvs.len() as u32)).unwrap());
        } else {
            glyphVariationDataOffsets
                .extend(&otspec::ser::to_bytes(&(serialized_tvs.len() as u16 / 2)).unwrap());
        }
        out.extend(
            otspec::ser::to_bytes(&gvarcore {
                majorVersion: 1,
                minorVersion: 0,
                axisCount,
                sharedTupleCount,
                sharedTuplesOffset: 20 + glyphVariationDataOffsets.len() as u32,
                glyphCount: self.variations.len() as u16,
                flags,
                glyphVariationDataArrayOffset: 20
                    + glyphVariationDataOffsets.len() as u32
                    + serialized_tuples.len() as u32,
            })
            .unwrap(),
        );

        out.extend(glyphVariationDataOffsets);
        out.extend(serialized_tuples);
        out.extend(serialized_tvs);

        out
    }
}

pub fn from_bytes(
    s: &[u8],
    coords_and_ends: Vec<(Vec<(int16, int16)>, Vec<usize>)>,
) -> otspec::error::Result<gvar> {
    let mut deserializer = otspec::de::Deserializer::from_bytes(s);
    let cs = GvarDeserializer { coords_and_ends };
    cs.deserialize(&mut deserializer)
}

// Serialization plan:
//  For each glyph, we have: Vec<DeltaSet>. We want TupleVariationStore (Vec<TupleVariation>).
//      A DeltaSet consists of peak/start/end and (i16,i16) deltas.
//      Each TupleVariation consists of the TupleVariationHeader and a Vec<Option<Delta>>

impl Serialize for gvar {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        panic!("Don't call this serializer, call the one in Font instead")
    }
}

#[cfg(test)]
mod tests {
    use crate::gvar;
    use crate::gvar::GlyphVariationData;

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
        let deserialized: gvar::gvar = gvar::from_bytes(
            &binary_gvar,
            vec![
                (vec![], vec![]), // .notdef
                (vec![], vec![]), // space
                (
                    vec![
                        (437, 125),
                        (109, 125),
                        (254, 308),
                        (0, 0),
                        (0, 0),
                        (0, 0),
                        (0, 0),
                    ],
                    vec![2, 3, 4, 5, 6],
                ),
                (
                    vec![
                        (261, 611),
                        (261, 113),
                        (108, 113),
                        (108, 611),
                        (0, 0),
                        (0, 0),
                        (0, 0),
                        (0, 0),
                    ],
                    vec![3, 4, 5, 6, 7],
                ),
            ],
        )
        .unwrap();
        let variations = &deserialized.variations;
        assert_eq!(variations[0], None);
        assert_eq!(variations[1], None);
        /*
            <glyphVariations glyph="A">
              <tuple>
                <coord axis="wght" value="1.0"/>
                <delta pt="0" x="0" y="-46"/>
                <delta pt="1" x="0" y="-46"/>
                <delta pt="2" x="0" y="46"/>
                <delta pt="3" x="0" y="0"/>
                <delta pt="4" x="0" y="0"/>
                <delta pt="5" x="0" y="0"/>
                <delta pt="6" x="0" y="0"/>
              </tuple>
              <tuple>
                <coord axis="wdth" value="1.0"/>
                <delta pt="0" x="82" y="0"/>
                <delta pt="1" x="-82" y="0"/>
                <delta pt="2" x="-9" y="0"/>
                <delta pt="3" x="0" y="0"/>
                <delta pt="4" x="0" y="0"/>
                <delta pt="5" x="0" y="0"/>
                <delta pt="6" x="0" y="0"/>
              </tuple>
            </glyphVariations>
        */
        assert_eq!(
            variations[2],
            Some(GlyphVariationData {
                deltasets: vec![
                    gvar::DeltaSet {
                        peak: vec![1.0, 0.0],
                        start: vec![1.0, 0.0],
                        end: vec![1.0, 0.0],
                        deltas: vec![(0, -46), (0, -46), (0, 46), (0, 0), (0, 0), (0, 0), (0, 0)]
                    },
                    gvar::DeltaSet {
                        peak: vec![0.0, 1.0],
                        start: vec![0.0, 1.0],
                        end: vec![0.0, 1.0],
                        deltas: vec![(82, 0), (-82, 0), (-9, 0), (0, 0), (0, 0), (0, 0), (0, 0)]
                    }
                ]
            })
        );
        assert_eq!(
            variations[3], // IUP here
            Some(GlyphVariationData {
                deltasets: vec![
                    gvar::DeltaSet {
                        peak: vec![1.0, 0.0],
                        start: vec![1.0, 0.0],
                        end: vec![1.0, 0.0],
                        deltas: vec![
                            (38, 125),   // IUP
                            (38, -125),  // given
                            (-38, -125), // IUP
                            (-38, 125),  // given
                            (0, 0),
                            (0, 0),
                            (0, 0),
                            (0, 0)
                        ]
                    },
                    gvar::DeltaSet {
                        peak: vec![0.0, 1.0],
                        start: vec![0.0, 1.0],
                        end: vec![0.0, 1.0],
                        deltas: vec![
                            (38, 0),
                            (38, 0),
                            (-38, 0),
                            (-38, 0),
                            (0, 0),
                            (0, 0),
                            (0, 0),
                            (0, 0)
                        ]
                    },
                    gvar::DeltaSet {
                        peak: vec![1.0, 1.0],
                        start: vec![1.0, 1.0],
                        end: vec![1.0, 1.0],
                        deltas: vec![
                            (19, 0),
                            (19, 0),
                            (-19, 0),
                            (-19, 0),
                            (0, 0),
                            (0, 0),
                            (0, 0),
                            (0, 0)
                        ]
                    }
                ]
            })
        );
    }

    #[test]
    fn gvar_ser() {
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
        let points = vec![
            (vec![], vec![]), // .notdef
            (vec![], vec![]), // space
            (
                vec![
                    (437, 125),
                    (109, 125),
                    (254, 308),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                ],
                vec![2, 3, 4, 5, 6],
            ),
            (
                vec![
                    (261, 611),
                    (261, 113),
                    (108, 113),
                    (108, 611),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                    (0, 0),
                ],
                vec![3, 4, 5, 6, 7],
            ),
        ];
        let deserialized: gvar::gvar = gvar::from_bytes(&binary_gvar, points.clone()).unwrap();
        let serialized = deserialized.to_bytes();
        let re_de: gvar::gvar = gvar::from_bytes(&serialized, points).unwrap();
        assert_eq!(re_de, deserialized); // Are they semantically the same?

        // They won't literally be the same quite yet because we are currently finessing
        // away a few hard problems - handling shared/private points, optimizing IUP
        // deltas, etc.

        // assert_eq!(serialized, binary_gvar); // Are they the same binary?
    }
}
