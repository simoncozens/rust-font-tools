use crate::otvar::{TupleIndexFlags, TupleVariationHeader, TupleVariationHeaderDeserializer};
use otspec::de::CountedDeserializer;
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::Deserializer;

use std::fmt;
extern crate otspec;

use otspec::types::*;
use otspec::{read_field, stateful_deserializer};

#[derive(Debug, PartialEq)]
pub struct GlyphVariationData {
    shared_points: Vec<uint16>,
    headers: Vec<TupleVariationHeader>,
}

stateful_deserializer!(
    GlyphVariationData,
    GlyphVariationDataDeserializer,
    {
        axisCount: uint16,
        is_gvar: bool,
        point_count: uint16
    },
    fn visit_seq<A>(self, mut seq: A) -> std::result::Result<GlyphVariationData, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let packed_count = read_field!(seq, uint16, "a packed count");
        let count = packed_count & 0x0FFF;
        let points_are_shared = (packed_count & 0x8000) != 0;
        let mut shared_points = vec![];
        let data_offset = read_field!(seq, uint16, "a data offset");
        let mut headers: Vec<TupleVariationHeader> = vec![];
        let all_points: Vec<u16> = (0..self.point_count).collect();
        for _ in 0..count {
            headers.push(
                seq.next_element_seed(TupleVariationHeaderDeserializer {
                    axisCount: self.axisCount,
                })?
                .unwrap(),
            );
        }
        if points_are_shared {
            // first thing in data offset is packed point number data
            let first_packed = read_field!(seq, u8, "a packed point number");
            let shared_point_count: uint16 = if first_packed > 127 {
                let second_packed = read_field!(seq, u8, "a packed point number");
                ((first_packed as uint16 & 0x7f) << 8) + (second_packed as uint16)
            } else {
                first_packed as uint16
            };
            if shared_point_count == 0 {
                // They're all shared
            } else {
                while shared_points.len() < shared_point_count as usize {
                    let run_control_byte = read_field!(seq, u8, "a control byte");
                    unimplemented!()
                }
            }
        }

        for header in &headers {
            println!("Processing header {:?}", header);
            let points_for_this_header: Vec<u16>;
            /* Private points? */
            if header
                .flags
                .contains(TupleIndexFlags::PRIVATE_POINT_NUMBERS)
            {
                let private_points = read_field!(seq, PackedPoints, "packed points");
                points_for_this_header = private_points.points.unwrap_or(all_points);
            } else {
                points_for_this_header = shared_points;
            }
        }

        Ok(GlyphVariationData {
            shared_points,
            headers,
        })
    }
);
