use super::glyf::{glyf, Glyph};
use crate::otvar::{
    iup::optimize_deltas, Delta, TupleIndexFlags, TupleVariation, TupleVariationHeader,
    TupleVariationStore,
};
use counter::Counter;
use otspec::types::*;
use otspec::{DeserializationError, Deserializer, ReaderContext, SerializationError, Serialize};
use otspec_macros::tables;
use std::convert::TryInto;

#[cfg(feature = "rayon")]
use rayon::prelude::*;

pub(crate) type Coords = Vec<(int16, int16)>;
pub(crate) type CoordsAndEndsVec = Vec<(Coords, Vec<usize>)>;

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
#[derive(Debug, PartialEq, Clone)]
pub struct DeltaSet {
    /// The peak location at which this region is active.
    pub peak: Tuple,
    /// The location at which this region begins to be active.
    pub start: Tuple,
    /// The location at which this region is no longer active.
    pub end: Tuple,
    /// A list of deltas to be applied to the glyph's coordinates at the peak of this region.
    pub deltas: Vec<(i16, i16)>,
}

impl DeltaSet {
    fn to_tuple_variation(
        &self,
        shared_tuples: &[Vec<u8>],
        original_glyph: Option<&Glyph>,
    ) -> TupleVariation {
        let mut serialized_peak: Vec<u8> = vec![];
        for p in &self.peak {
            F2DOT14::from(*p).to_bytes(&mut serialized_peak).unwrap();
        }
        let index = shared_tuples.iter().position(|t| t == &serialized_peak);
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

        let deltas: Vec<Option<Delta>> = self
            .deltas
            .iter()
            .map(|(x, y)| Some(Delta::Delta2D((*x, *y))))
            .collect();
        if let Some(glyph) = original_glyph {
            let optimized_deltas = optimize_deltas(deltas.clone(), glyph);
            if optimized_deltas.iter().flatten().count() == 0 {
                // Zero private points goes bad
                return TupleVariation(tvh, deltas);
            }
            /* Disgusting amounts of cloning here to check length. :-/ */
            let deltas_copy = deltas.clone();
            let tv_unoptimized = TupleVariation(tvh.clone(), deltas_copy);
            let original_length = otspec::ser::to_bytes(&TupleVariationStore(vec![tv_unoptimized]))
                .unwrap()
                .len();

            let optimized_length =
                otspec::ser::to_bytes(&TupleVariationStore(vec![TupleVariation(
                    tvh.clone(),
                    optimized_deltas.clone(),
                )]))
                .unwrap()
                .len();
            if optimized_length < original_length {
                return TupleVariation(tvh, optimized_deltas);
            } else {
                return TupleVariation(tvh, deltas);
            }
        }
        TupleVariation(tvh, deltas)
    }

    pub(crate) fn combine(&self, other: &Self) -> Self {
        let mut new = self.clone();
        if new.deltas.len() != other.deltas.len() {
            panic!("Tried to add deltas with different lengths")
        }
        for (ix, (x, y)) in new.deltas.iter_mut().enumerate() {
            *x += other.deltas[ix].0;
            *y += other.deltas[ix].1;
        }
        new
    }

    pub(crate) fn scale_deltas(&mut self, factor: f32) {
        for (x, y) in self.deltas.iter_mut() {
            *x = (*x as f32 * factor) as i16;
            *y = (*y as f32 * factor) as i16;
        }
    }
}

#[derive(Debug, PartialEq, Clone)]
/// A description of how an individual glyph's outline varies across the designspace.
pub struct GlyphVariationData {
    /// A list of designsets, containing deltas at particular designspace regions.
    pub deltasets: Vec<DeltaSet>,
}

#[derive(Debug, PartialEq, Clone)]
#[allow(non_camel_case_types)]
/// A Glyph Variations table, describing how glyph outlines vary across the
/// designspace.
pub struct gvar {
    /// An array of variation data, one for each glyph in the `glyf` table.
    pub variations: Vec<Option<GlyphVariationData>>,
}

/// Constructs a `gvar` object from a binary table, given a set of coordinates
/// and end-of-contour indices. These can be extracted from the `glyf` table by
/// calling the `gvar_coords_and_ends` method on each glyph.
pub fn from_bytes(
    s: &[u8],
    coords_and_ends: CoordsAndEndsVec,
) -> Result<gvar, DeserializationError> {
    let mut c = ReaderContext::new(s.to_vec());
    c.push();
    let core: gvarcore = c.de()?;
    let offset_count = (core.glyphCount + 1) as usize;
    let data_offsets: Vec<u32> = if core.flags & 0x1 == 0 {
        // u16 offsets, need doubling
        let u16_and_halved: Vec<u16> = c.de_counted(offset_count)?;
        u16_and_halved.iter().map(|x| (x * 2).into()).collect()
    } else {
        c.de_counted(offset_count)?
    };
    // println!("Offsets {:?}", dataOffsets);
    let axis_count = core.axisCount as usize;

    /* Shared tuples */
    let mut shared_tuples: Vec<Tuple> = Vec::with_capacity(core.sharedTupleCount as usize);
    c.ptr = c.top_of_table() + (core.sharedTuplesOffset as usize);
    for _ in 0..core.sharedTupleCount + 1 {
        // println!("Trying to deserialize shared tuple array {:?}", bytes);
        let tuple: Vec<F2DOT14> = c.de_counted(axis_count)?;
        let tuple_f32: Vec<f32> = tuple.iter().map(|t| (*t).into()).collect();
        shared_tuples.push(tuple_f32);
    }

    /* Glyph variation data */
    let mut glyph_variations = vec![];
    for i in 0..(core.glyphCount as usize) {
        // println!("Reading data for glyph {:?}", i);
        let offset: usize = (data_offsets[i] + (core.glyphVariationDataArrayOffset))
            .try_into()
            .unwrap();
        let next_offset: usize = (data_offsets[(i + 1) as usize]
            + (core.glyphVariationDataArrayOffset))
            .try_into()
            .unwrap();
        let length = next_offset - offset;
        if length == 0 {
            glyph_variations.push(None);
        } else {
            let mut deltasets: Vec<DeltaSet> = vec![];
            c.ptr = c.top_of_table() + offset;
            let tvs = TupleVariationStore::from_bytes(
                &mut c,
                axis_count.try_into().unwrap(),
                true,
                coords_and_ends[i].0.len() as u16,
            )?;
            // println!("TVS {:?}", tvs);
            for tvh in tvs.0 {
                let deltas = tvh.iup_delta(&coords_and_ends[i].0, &coords_and_ends[i].1);
                let index = tvh.0.sharedTupleIndex as usize;
                if index > shared_tuples.len() {
                    return Err(DeserializationError(format!(
                        "Invalid shared tuple index {:}",
                        index
                    )));
                }
                let peak_tuple = tvh
                    .0
                    .peakTuple
                    .unwrap_or_else(|| shared_tuples[index].clone());
                let start_tuple = tvh.0.startTuple.unwrap_or_else(|| {
                    peak_tuple
                        .iter()
                        .map(|&x| if x > 0.0 { 0.0 } else { -1.0 })
                        .collect()
                });
                let end_tuple = tvh.0.endTuple.unwrap_or_else(|| {
                    peak_tuple
                        .iter()
                        .map(|&x| if x > 0.0 { 1.0 } else { 0.0 })
                        .collect()
                });
                deltasets.push(DeltaSet {
                    deltas,
                    peak: peak_tuple,
                    end: end_tuple,
                    start: start_tuple,
                })
            }
            glyph_variations.push(Some(GlyphVariationData { deltasets }));
        }
    }

    Ok(gvar {
        variations: glyph_variations,
    })
}

impl gvar {
    /// Serializes this table to binary, given a reference to the `glyf` table.
    pub fn to_bytes(&self, glyf: Option<&glyf>) -> Vec<u8> {
        let mut out: Vec<u8> = vec![];
        // Determine all the shared tuples.
        let mut shared_tuple_counter: Counter<Vec<u8>> = Counter::new();
        let mut axis_count: uint16 = 0;
        for var in self.variations.iter().flatten() {
            for ds in &var.deltasets {
                axis_count = ds.peak.len() as uint16;
                // println!("Peak: {:?}", ds.peak);
                let mut tuple: Vec<u8> = vec![];
                for t in &ds.peak {
                    F2DOT14::from(*t).to_bytes(&mut tuple).unwrap();
                }
                shared_tuple_counter[&tuple] += 1;
            }
        }
        // shared_tuple_counter.retain(|_, &mut v| v > 1);
        let most_common_tuples: Vec<(Vec<u8>, usize)> = shared_tuple_counter.most_common();
        if most_common_tuples.is_empty() {
            panic!("Some more sensible error checking here for null case");
        }
        let shared_tuple_count = most_common_tuples.len() as u16;
        let flags = 1; // XXX

        let mut glyph_variation_data_offsets: Vec<u8> = vec![];

        // println!("Most common tuples: {:?}", most_common_tuples);
        let mut shared_tuples = vec![];
        let mut serialized_tuples = vec![];
        let mut serialized_tvs = vec![];
        for (a, _) in most_common_tuples {
            serialized_tuples.extend(otspec::ser::to_bytes(&a).unwrap());
            shared_tuples.push(a);
        }
        // Now we need a bunch of TVSes
        for (ix, var) in self.variations.iter().enumerate() {
            // Data offset
            if flags != 0 {
                glyph_variation_data_offsets
                    .extend(&otspec::ser::to_bytes(&(serialized_tvs.len() as u32)).unwrap());
            } else {
                glyph_variation_data_offsets
                    .extend(&otspec::ser::to_bytes(&(serialized_tvs.len() as u16 / 2)).unwrap());
            }

            if let Some(var) = var {
                let maybe_glyph = glyf.map(|g| &g.glyphs[ix]);
                #[cfg(feature = "rayon")]
                let tuple_variations = var
                    .deltasets
                    .par_iter()
                    .map(|ds| ds.to_tuple_variation(&shared_tuples, maybe_glyph))
                    .filter(|tv| tv.has_effect())
                    .collect();

                #[cfg(not(feature = "rayon"))]
                let tuple_variations = var
                    .deltasets
                    .iter()
                    .map(|ds| ds.to_tuple_variation(&shared_tuples, maybe_glyph))
                    .filter(|tv| tv.has_effect())
                    .collect();

                let tvs = TupleVariationStore(tuple_variations);
                serialized_tvs.extend(otspec::ser::to_bytes(&tvs).unwrap());
                // Add a byte of padding
                if (serialized_tvs.len() % 2) != 0 {
                    serialized_tvs.push(0);
                }
            }
        }
        // Final data offset
        if flags != 0 {
            glyph_variation_data_offsets
                .extend(&otspec::ser::to_bytes(&(serialized_tvs.len() as u32)).unwrap());
        } else {
            glyph_variation_data_offsets
                .extend(&otspec::ser::to_bytes(&(serialized_tvs.len() as u16 / 2)).unwrap());
        }
        out.extend(
            otspec::ser::to_bytes(&gvarcore {
                majorVersion: 1,
                minorVersion: 0,
                axisCount: axis_count,
                sharedTupleCount: shared_tuple_count,
                sharedTuplesOffset: 20 + glyph_variation_data_offsets.len() as u32,
                glyphCount: self.variations.len() as u16,
                flags,
                glyphVariationDataArrayOffset: 20
                    + glyph_variation_data_offsets.len() as u32
                    + serialized_tuples.len() as u32,
            })
            .unwrap(),
        );

        out.extend(glyph_variation_data_offsets);
        out.extend(serialized_tuples);
        out.extend(serialized_tvs);

        out
    }
}

// Serialization plan:
//  For each glyph, we have: Vec<DeltaSet>. We want TupleVariationStore (Vec<TupleVariation>).
//      A DeltaSet consists of peak/start/end and (i16,i16) deltas.
//      Each TupleVariation consists of the TupleVariationHeader and a Vec<Option<Delta>>

impl Serialize for gvar {
    fn to_bytes(&self, _data: &mut Vec<u8>) -> Result<(), SerializationError> {
        panic!("Don't call this serializer, call the one in Font instead")
    }
}

#[cfg(test)]
mod tests {
    use super::GlyphVariationData;

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
        let deserialized: super::gvar = super::from_bytes(
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
            Some(super::GlyphVariationData {
                deltasets: vec![
                    super::DeltaSet {
                        peak: vec![1.0, 0.0],
                        start: vec![0.0, -1.0],
                        end: vec![1.0, 0.0],
                        deltas: vec![(0, -46), (0, -46), (0, 46), (0, 0), (0, 0), (0, 0), (0, 0)]
                    },
                    super::DeltaSet {
                        peak: vec![0.0, 1.0],
                        start: vec![-1.0, 0.0],
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
                    super::DeltaSet {
                        peak: vec![1.0, 0.0],
                        start: vec![0.0, -1.0],
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
                    super::DeltaSet {
                        peak: vec![0.0, 1.0],
                        start: vec![-1.0, 0.0],
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
                    super::DeltaSet {
                        peak: vec![1.0, 1.0],
                        start: vec![0.0, 0.0],
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
        let deserialized: super::gvar = super::from_bytes(&binary_gvar, points.clone()).unwrap();
        let serialized = deserialized.to_bytes(None);
        let re_de: super::gvar = super::from_bytes(&serialized, points).unwrap();
        assert_eq!(re_de, deserialized); // Are they semantically the same?

        // They won't literally be the same quite yet because we are currently finessing
        // away a few hard problems - handling shared/private points, optimizing IUP
        // deltas, etc.

        // assert_eq!(serialized, binary_gvar); // Are they the same binary?
    }
}
