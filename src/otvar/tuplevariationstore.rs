use crate::otvar::{
    Delta, PackedDeltas, PackedDeltasDeserializer, PackedPoints, TupleIndexFlags,
    TupleVariationHeader, TupleVariationHeaderDeserializer,
};
use otspec::types::*;
use otspec::{read_field, stateful_deserializer};
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};
use std::collections::VecDeque;

/// A record within a tuple variation store
///
/// This is a low-level representation of variation data, consisting of a
/// TupleVariationHeader (which serves to locate the deltas in the design space)
/// and an optimized set of deltas, some of which may be omitted due to IUP.
#[derive(Debug, PartialEq)]
pub struct TupleVariation(pub TupleVariationHeader, pub Vec<Option<Delta>>);

fn iup_segment(
    newdeltas: &mut Vec<(i16, i16)>,
    coords: &[(i16, i16)],
    rc1: (i16, i16),
    rd1: &Option<Delta>,
    rc2: (i16, i16),
    rd2: &Option<Delta>,
) {
    let rd1 = rd1.as_ref().unwrap().get_2d();
    let rd2 = rd2.as_ref().unwrap().get_2d();
    let mut out_arrays: Vec<Vec<i16>> = vec![vec![], vec![]];
    for j in 0..2 {
        let (mut x1, mut x2, mut d1, mut d2) = if j == 0 {
            (rc1.0, rc2.0, rd1.0, rd2.0)
        } else {
            (rc1.1, rc2.1, rd1.1, rd2.1)
        };
        if x1 == x2 {
            let n = coords.len();
            out_arrays[j].extend(std::iter::repeat(if d1 == d2 { d1 } else { 0 }).take(n));
            continue;
        }
        if x1 > x2 {
            std::mem::swap(&mut x2, &mut x1);
            std::mem::swap(&mut d2, &mut d1);
        }

        let scale = (d2 - d1) as f32 / (x2 - x1) as f32;

        for pair in coords {
            let x = if j == 0 { pair.0 } else { pair.1 };
            let d = if x <= x1 {
                d1
            } else if x >= x2 {
                d2
            } else {
                d1 + ((x - x1) as f32 * scale) as i16
            };
            out_arrays[j].push(d);
        }
    }
    newdeltas.extend(
        out_arrays[0]
            .iter()
            .zip(out_arrays[1].iter())
            .map(|(x, y)| (*x, *y)),
    );
}

fn iup_contour(newdeltas: &mut Vec<(i16, i16)>, deltas: &[Option<Delta>], coords: &[(i16, i16)]) {
    if deltas.iter().all(|x| x.is_some()) {
        newdeltas.extend::<Vec<(i16, i16)>>(
            deltas
                .iter()
                .map(|x| x.as_ref().unwrap().get_2d())
                .collect(),
        );
        return;
    }
    let n = deltas.len();
    let indices: Vec<usize> = deltas
        .iter()
        .enumerate()
        .filter(|(_, d)| d.is_some())
        .map(|(i, _)| i)
        .collect();
    if indices.is_empty() {
        newdeltas.extend(std::iter::repeat((0, 0)).take(n));
        return;
    }
    let mut start = indices[0];
    let verystart = start;
    if start != 0 {
        let (i1, i2, ri1, ri2) = (0, start, start, *indices.last().unwrap());
        iup_segment(
            newdeltas,
            &coords[i1..i2],
            coords[ri1],
            &deltas[ri1],
            coords[ri2],
            &deltas[ri2],
        );
    }
    newdeltas.push(deltas[start].as_ref().unwrap().get_2d());
    for end in indices.iter().skip(1) {
        if *end - start > 1 {
            let (i1, i2, ri1, ri2) = (start + 1, *end, start, *end);
            iup_segment(
                newdeltas,
                &coords[i1..i2],
                coords[ri1],
                &deltas[ri1],
                coords[ri2],
                &deltas[ri2],
            );
        }
        newdeltas.push(deltas[*end].as_ref().unwrap().get_2d());
        start = *end;
    }
    if start != n - 1 {
        let (i1, i2, ri1, ri2) = (start + 1, n, start, verystart);
        iup_segment(
            newdeltas,
            &coords[i1..i2],
            coords[ri1],
            &deltas[ri1],
            coords[ri2],
            &deltas[ri2],
        );
    }
}

impl TupleVariation {
    /// Unpacks the delta array using Interpolation of Unreferenced Points
    ///
    /// The tuple variation record is stored in an optimized format with deltas
    /// omitted if they can be computed from other surrounding deltas. This takes
    /// a tuple variation record along with the original points list (from the glyf
    /// table) and the indices of the end points of the contours (as the optimization
    /// is done contour-wise), and returns a full list of (x,y) deltas, with the
    /// implied deltas expanded.
    pub fn iup_delta(&self, coords: &[(i16, i16)], ends: &[usize]) -> Vec<(i16, i16)> {
        // Unlike Python the ends array has all the ends in.
        let deltas = &self.1;
        if deltas.iter().all(|x| x.is_some()) {
            // No IUP needed
            return self
                .1
                .iter()
                .map(|x| x.as_ref().unwrap().get_2d())
                .collect();
        }
        let mut newdeltas = vec![];
        let mut start = 0;
        for end in ends {
            let contour_delta = &deltas[start..end + 1];
            let contour_orig = &coords[start..end + 1];
            start = end + 1;
            iup_contour(&mut newdeltas, contour_delta, contour_orig);
        }
        newdeltas
    }
}

/// A Tuple Variation Store
///
/// A tuple variation store is the way that OpenType internally represents
/// variation records in the `gvar` and `cvt` tables.
#[derive(Debug, PartialEq)]
pub struct TupleVariationStore(pub Vec<TupleVariation>);

stateful_deserializer!(
    TupleVariationStore,
    TupleVariationStoreDeserializer,
    {
        axis_count: uint16,
        is_gvar: bool,
        point_count: uint16
    },
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<TupleVariationStore, A::Error>
    where
        A: SeqAccess<'de>,
    {
        // Begin with the "GlyphVariationData header"
        let packed_count = read_field!(seq, uint16, "a packed count");
        let count = packed_count & 0x0FFF;
        let points_are_shared = (packed_count & 0x8000) != 0;
        let mut shared_points = vec![];
        let _data_offset = read_field!(seq, uint16, "a data offset");

        // Read the headers
        let mut headers: Vec<TupleVariationHeader> = vec![];
        let mut variations: Vec<TupleVariation> = vec![];
        for _ in 0..count {
            headers.push(
                seq.next_element_seed(TupleVariationHeaderDeserializer {
                    axis_count: self.axis_count,
                })?
                .unwrap(),
            );
        }

        // Now we are into the "serialized data block"
        // ...which begins with Shared "point" numbers (optional per flag in the header)
        if points_are_shared {
            shared_points = match read_field!(seq, PackedPoints, "packed points").points {
                Some(pts) => pts,
                None =>  (0..self.point_count).collect()
            };
        }

        // And finally per-tuple variation data
        for header in headers {
            let mut points_for_this_header: VecDeque<u16>;
            /* Private points? */
            if header
                .flags
                .contains(TupleIndexFlags::PRIVATE_POINT_NUMBERS)
            {
                let private_points = read_field!(seq, PackedPoints, "packed points");
                if private_points.points.is_some() {
                    points_for_this_header = private_points.points.unwrap().clone().into();
                } else {
                    points_for_this_header =  (0..self.point_count).collect();
                }
            } else {
                points_for_this_header = shared_points.clone().into();
            }
            let mut deltas:VecDeque<Delta> = if self.is_gvar {
                let packed_x = seq.next_element_seed(PackedDeltasDeserializer { num_points: points_for_this_header.len() })?.unwrap().0;
                let packed_y = seq.next_element_seed(PackedDeltasDeserializer { num_points: points_for_this_header.len() })?.unwrap().0;
                packed_x.iter().zip(packed_y.iter()).map(|(x,y)| Delta::Delta2D((*x,*y)) ).collect()
            } else {
                let packed = seq.next_element_seed(PackedDeltasDeserializer { num_points: points_for_this_header.len() })?.unwrap().0;
                packed.iter().map(|x| Delta::Delta1D(*x) ).collect()
            };
            let mut all_deltas:Vec<Option<Delta>> = vec![];
            for i in 0..self.point_count {
                if !points_for_this_header.is_empty() && i == points_for_this_header[0] {
                    all_deltas.push(Some(deltas.pop_front().unwrap()));
                    points_for_this_header.pop_front();
                } else {
                    all_deltas.push(None);  // IUP needed later
                }
            }
            variations.push( TupleVariation(header, all_deltas))
        }

        Ok(TupleVariationStore(variations))
    }
);

impl Serialize for TupleVariationStore {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut ser = serializer.serialize_seq(None)?;
        let packed_count: uint16 = self.0.len() as uint16 | 0x8000; // Shared points only!
        ser.serialize_element(&packed_count)?;
        let mut serialized_headers = vec![];
        let mut serialized_data_block: Vec<u8> = vec![];

        // Shared points go here

        #[allow(clippy::vec_init_then_push)]
        let _ = serialized_data_block.push(0); // This is dummy code

        let mut last_delta_len = serialized_data_block.len();
        for var in &self.0 {
            // For each glyph
            let header = &var.0;
            // We need to set .flags here
            let deltas = &var.1;

            // Private point numbers go here
            // println!("Last length was {:?}", last_delta_len);

            let mut dx = vec![];
            let mut dy = vec![];
            for d in deltas.iter().flatten() {
                match d {
                    Delta::Delta1D(d) => {
                        dx.push(*d);
                    }
                    Delta::Delta2D((x, y)) => {
                        dx.push(*x);
                        dy.push(*y);
                    }
                }
            }
            // Remove the .clones here when things are fixed, they're only needed for a later println
            serialized_data_block.extend(otspec::ser::to_bytes(&PackedDeltas(dx.clone())).unwrap());
            if !dy.is_empty() {
                serialized_data_block
                    .extend(otspec::ser::to_bytes(&PackedDeltas(dy.clone())).unwrap());
            }
            // println!("Serializing a TVH (will fix size later): {:?}", header);
            let mut serialized_header = otspec::ser::to_bytes(&header).unwrap();
            // println!("Current data block {:?}", serialized_data_block);
            // println!("Current length is {:?}", serialized_data_block.len());
            let data_size = (serialized_data_block.len() - last_delta_len) as u16;
            // println!("Data size is {:?}", data_size);
            let size: Vec<u8> = otspec::ser::to_bytes(&data_size).unwrap();
            // Set header size
            serialized_header[0] = size[0];
            serialized_header[1] = size[1];
            // println!("    header as bytes {:?}", serialized_header);
            // println!("    X deltas {:?}", dx);
            // println!("    Y deltas {:?}", dy);
            // println!(
            // "    data for this header: {:?}",
            // serialized_data_block[last_delta_len..serialized_data_block.len()].to_vec()
            // );
            last_delta_len = serialized_data_block.len();
            serialized_headers.extend(serialized_header);
        }
        let data_offset: uint16 = 4 + (serialized_headers.len() as uint16);
        ser.serialize_element(&data_offset)?;
        ser.serialize_element(&serialized_headers)?;
        ser.serialize_element(&serialized_data_block)?;
        ser.end()
    }
}

#[cfg(test)]
mod tests {
    use crate::otvar::Delta::Delta2D;
    use crate::otvar::TupleVariation;
    use crate::otvar::TupleVariationHeader;
    use crate::otvar::{TupleIndexFlags, TupleVariationStore, TupleVariationStoreDeserializer};
    use serde::de::DeserializeSeed;

    #[test]
    fn test_tvs_de() {
        let binary_tvs: Vec<u8> = vec![
            0x80, 0x01, 0x00, 0x0a, 0x00, 0x21, 0x80, 0x00, 0x20, 0x00, 0x00, 0x06, 0xcb, 0xd0,
            0xb7, 0xbb, 0x00, 0xf0, 0x8c, 0x40, 0xff, 0x7c, 0x03, 0xb5, 0xd2, 0xc3, 0x00, 0x40,
            0xfe, 0xe0, 0x81, 0x0a, 0x08, 0xfd, 0xfd, 0x08, 0x08, 0xe4, 0xe4, 0x08, 0xc5, 0xc5,
            0xeb, 0x83,
        ];
        let mut de = otspec::de::Deserializer::from_bytes(&binary_tvs);
        let cs = TupleVariationStoreDeserializer {
            axis_count: 1,
            point_count: 15,
            is_gvar: true,
        };
        let tvs = cs.deserialize(&mut de).unwrap();
        let expected = TupleVariationStore(vec![TupleVariation(
            TupleVariationHeader {
                size: 33,
                flags: TupleIndexFlags::EMBEDDED_PEAK_TUPLE,
                sharedTupleIndex: 0,
                peakTuple: Some(vec![0.5]),
                startTuple: None,
                endTuple: None,
            },
            vec![
                Some(Delta2D((-53, 8))),
                Some(Delta2D((-48, -3))),
                Some(Delta2D((-73, -3))),
                Some(Delta2D((-69, 8))),
                Some(Delta2D((0, 8))),
                Some(Delta2D((-16, -28))),
                Some(Delta2D((-116, -28))),
                Some(Delta2D((-132, 8))),
                Some(Delta2D((-75, -59))),
                Some(Delta2D((-46, -59))),
                Some(Delta2D((-61, -21))),
                Some(Delta2D((0, 0))),
                Some(Delta2D((-288, 0))),
                Some(Delta2D((0, 0))),
                Some(Delta2D((0, 0))),
            ],
        )]);
        assert_eq!(tvs, expected);
    }

    #[test]
    fn test_tvs_ser() {
        let expected: Vec<u8> = vec![
            0x80, 0x01, /* tupleVariationCount. SHARED_POINT_NUMBERS */
            0x00, 0x0a, /* dataOffset */
            /* TVH */
            0x00, 0x21, /* variationDataSize: 33 bytes */
            0x80, 0x00, /* tuple index. EMBEDDED_PEAK_TUPLE */
            0x20, 0x00, /* Peak tuple record */
            0x00, /* Shared point numbers */
            /* per-tuple variation data */
            0x06, 0xcb, 0xd0, 0xb7, 0xbb, 0x00, 0xf0, 0x8c, 0x40, 0xff, 0x7c, 0x03, 0xb5, 0xd2,
            0xc3, 0x00, 0x40, 0xfe, 0xe0, 0x81, 0x0a, 0x08, 0xfd, 0xfd, 0x08, 0x08, 0xe4, 0xe4,
            0x08, 0xc5, 0xc5, 0xeb, 0x83,
        ];
        let tvs = TupleVariationStore(vec![TupleVariation(
            TupleVariationHeader {
                size: 33,
                flags: TupleIndexFlags::EMBEDDED_PEAK_TUPLE,
                sharedTupleIndex: 0,
                peakTuple: Some(vec![0.5]),
                startTuple: None,
                endTuple: None,
            },
            vec![
                Some(Delta2D((-53, 8))),
                Some(Delta2D((-48, -3))),
                Some(Delta2D((-73, -3))),
                Some(Delta2D((-69, 8))),
                Some(Delta2D((0, 8))),
                Some(Delta2D((-16, -28))),
                Some(Delta2D((-116, -28))),
                Some(Delta2D((-132, 8))),
                Some(Delta2D((-75, -59))),
                Some(Delta2D((-46, -59))),
                Some(Delta2D((-61, -21))),
                Some(Delta2D((0, 0))),
                Some(Delta2D((-288, 0))),
                Some(Delta2D((0, 0))),
                Some(Delta2D((0, 0))),
            ],
        )]);
        let binary_tvs = otspec::ser::to_bytes(&tvs).unwrap();
        assert_eq!(binary_tvs, expected);
    }
}
