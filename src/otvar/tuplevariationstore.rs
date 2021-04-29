use crate::otvar::{
    Delta, PackedDeltasDeserializer, PackedPoints, TupleIndexFlags, TupleVariationHeader,
    TupleVariationHeaderDeserializer,
};
use otspec::types::*;
use otspec::{read_field, stateful_deserializer};
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use std::collections::VecDeque;

#[derive(Debug, PartialEq)]
pub struct TupleVariation(pub TupleVariationHeader, pub Vec<Option<Delta>>);

fn iup_contour(
    newdeltas: &mut Vec<(i16, i16)>,
    deltas: &[Option<Delta>],
    coords: &[(i16, i16)],
) -> Vec<(i16, i16)> {
    unimplemented!()
}

impl TupleVariation {
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
        let packed_count = read_field!(seq, uint16, "a packed count");
        let count = packed_count & 0x0FFF;
        let points_are_shared = (packed_count & 0x8000) != 0;
        let mut shared_points = vec![];
        let _data_offset = read_field!(seq, uint16, "a data offset");
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
        if points_are_shared {
            shared_points = match read_field!(seq, PackedPoints, "packed points").points {
                Some(pts) => pts,
                None =>  (0..self.point_count).collect()
            };
        }

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
