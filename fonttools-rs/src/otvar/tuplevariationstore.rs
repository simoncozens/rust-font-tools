use super::iup::iup_contour;
use super::packeddeltas::from_bytes as packed_deltas_from_bytes;
use super::{Delta, PackedDeltas, PackedPoints, TupleIndexFlags, TupleVariationHeader};
use otspec::types::*;
use otspec::{
    DeserializationError, Deserializer, ReaderContext, SerializationError, Serialize, Serializer,
};
use std::collections::VecDeque;

/// A record within a tuple variation store
///
/// This is a low-level representation of variation data, consisting of a
/// TupleVariationHeader (which serves to locate the deltas in the design space)
/// and an optimized set of deltas, some of which may be omitted due to IUP.
#[derive(Debug, PartialEq, Clone)]
pub struct TupleVariation(pub TupleVariationHeader, pub Vec<Option<Delta>>);

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

    /// Determines if any deltas in this variation are non-zero
    pub fn has_effect(&self) -> bool {
        self.1.iter().any(|maybe_delta| {
            maybe_delta.is_some() && {
                let d2d = maybe_delta.as_ref().unwrap().get_2d();
                d2d.0 != 0 || d2d.1 != 0
            }
        })
    }
}

/// A Tuple Variation Store
///
/// A tuple variation store is the way that OpenType internally represents
/// variation records in the `gvar` and `cvt` tables.
#[derive(Debug, PartialEq, Clone)]
pub struct TupleVariationStore(pub Vec<TupleVariation>);

impl TupleVariationStore {
    /// Construct a new TupleVariationStore from a serialized binary
    pub fn from_bytes(
        c: &mut ReaderContext,
        axis_count: uint16,
        is_gvar: bool,
        point_count: uint16,
    ) -> Result<Self, DeserializationError> {
        // Begin with the "GlyphVariationData header"
        let packed_count: uint16 = c.de()?;
        let count = packed_count & 0x0FFF;
        let points_are_shared = (packed_count & 0x8000) != 0;
        let mut shared_points = vec![];
        let _data_offset: uint16 = c.de()?;

        // Read the headers
        let mut headers: Vec<TupleVariationHeader> = vec![];
        let mut variations: Vec<TupleVariation> = vec![];
        for _ in 0..count {
            let header = TupleVariationHeader::from_bytes(c, axis_count)?;
            headers.push(header);
        }

        // Now we are into the "serialized data block"
        // ...which begins with Shared "point" numbers (optional per flag in the header)
        if points_are_shared {
            let pp: PackedPoints = c.de()?;
            shared_points = match pp.points {
                Some(pts) => pts,
                None => (0..point_count).collect(),
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
                let private_points: PackedPoints = c.de()?;
                if private_points.points.is_some() {
                    points_for_this_header = private_points.points.unwrap().clone().into();
                } else {
                    points_for_this_header = (0..point_count).collect();
                }
            } else {
                points_for_this_header = shared_points.clone().into();
            }
            #[allow(clippy::branches_sharing_code)] // Just easier to understand this way
            let mut deltas: VecDeque<Delta> = if is_gvar {
                let packed_x = packed_deltas_from_bytes(c, points_for_this_header.len())?.0;
                let packed_y = packed_deltas_from_bytes(c, points_for_this_header.len())?.0;
                packed_x
                    .iter()
                    .zip(packed_y.iter())
                    .map(|(x, y)| Delta::Delta2D((*x, *y)))
                    .collect()
            } else {
                let packed = packed_deltas_from_bytes(c, points_for_this_header.len())?.0;
                packed.iter().map(|x| Delta::Delta1D(*x)).collect()
            };
            let mut all_deltas: Vec<Option<Delta>> = vec![];
            for i in 0..point_count {
                if !points_for_this_header.is_empty() && i == points_for_this_header[0] {
                    all_deltas.push(Some(deltas.pop_front().unwrap()));
                    points_for_this_header.pop_front();
                } else {
                    all_deltas.push(None); // IUP needed later
                }
            }
            variations.push(TupleVariation(header, all_deltas))
        }

        Ok(TupleVariationStore(variations))
    }
}

impl Serialize for TupleVariationStore {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let packed_count: uint16 = self.0.len() as uint16 | 0x8000; // Shared points only!
        data.put(packed_count)?;
        let mut serialized_headers = vec![];
        let mut serialized_data_block: Vec<u8> = vec![];

        // Shared points go here

        #[allow(clippy::vec_init_then_push)]
        let _ = serialized_data_block.push(0); // This is dummy code

        let mut last_delta_len = serialized_data_block.len();
        for var in &self.0 {
            // For each variation
            let mut header = var.0.clone();
            // We need to set .flags here
            let deltas = &var.1;

            // Private point numbers go here
            if deltas.iter().any(|x| x.is_none()) {
                let mut private_points: Vec<u16> = vec![];
                header.flags |= TupleIndexFlags::PRIVATE_POINT_NUMBERS;
                for (ix, d) in deltas.iter().enumerate() {
                    if d.is_some() {
                        private_points.push(ix as u16);
                    }
                }
                serialized_data_block.extend(
                    otspec::ser::to_bytes(&PackedPoints {
                        points: Some(private_points),
                    })
                    .unwrap(),
                );
            }
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
        data.put(data_offset)?;
        data.put(serialized_headers)?;
        data.put(serialized_data_block)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::otvar::Delta::Delta2D;
    use crate::otvar::{
        TupleIndexFlags, TupleVariation, TupleVariationHeader, TupleVariationStore,
    };
    use otspec::ReaderContext;

    #[test]
    fn test_tvs_de() {
        let binary_tvs: Vec<u8> = vec![
            0x80, 0x01, 0x00, 0x0a, 0x00, 0x21, 0x80, 0x00, 0x20, 0x00, 0x00, 0x06, 0xcb, 0xd0,
            0xb7, 0xbb, 0x00, 0xf0, 0x8c, 0x40, 0xff, 0x7c, 0x03, 0xb5, 0xd2, 0xc3, 0x00, 0x40,
            0xfe, 0xe0, 0x81, 0x0a, 0x08, 0xfd, 0xfd, 0x08, 0x08, 0xe4, 0xe4, 0x08, 0xc5, 0xc5,
            0xeb, 0x83,
        ];
        let tvs = TupleVariationStore::from_bytes(&mut ReaderContext::new(binary_tvs), 1, true, 15)
            .unwrap();
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
