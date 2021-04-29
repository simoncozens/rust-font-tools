use otspec::types::*;
use otspec::{deserialize_visitor, read_field, read_field_counted};
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::Deserialize;
use serde::Deserializer;

#[derive(Debug, PartialEq)]
pub struct PackedPoints {
    pub points: Option<Vec<uint16>>,
}

deserialize_visitor!(
    PackedPoints,
    PackedPointsVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut count: u16 = read_field!(seq, u8, "a packed point count (first byte)") as u16;
        if count > 127 {
            let count2: u16 = read_field!(seq, u8, "a packed point count (second byte)") as u16;
            count = (count & 0xff) << 8 | count2;
        }
        if count == 0 {
            // All of them
            return Ok(PackedPoints { points: None });
        }
        let mut res = vec![];
        while res.len() < count as usize {
            let control_byte = read_field!(seq, u8, "a packed point control byte");
            let points_are_words = (control_byte & 0x80) > 0;
            // "The low 7 bits specify the number of elements in the run minus 1."
            // MINUS ONE.
            let run_count = (control_byte & 0x7f) + 1;
            let deltas: Vec<u16>;
            if points_are_words {
                deltas = read_field_counted!(seq, run_count, "packed points");
            } else {
                let delta_bytes: Vec<u8> = read_field_counted!(seq, run_count, "packed points");
                deltas = delta_bytes.iter().map(|x| *x as u16).collect();
            }
            res.extend(deltas);
        }
        let cumsum: Vec<u16> = res
            .iter()
            .scan(0, |acc, &x| {
                *acc += x;
                Some(*acc)
            })
            .collect();
        Ok(PackedPoints {
            points: Some(cumsum),
        })
    }
);

#[cfg(test)]
mod tests {
    use crate::otvar::PackedPoints;

    #[test]
    fn test_packed_point_de() {
        let packed = vec![
            0x0b, 0x0a, 0x00, 0x03, 0x01, 0x03, 0x01, 0x03, 0x01, 0x03, 0x02, 0x02, 0x02,
        ];
        let expected = PackedPoints {
            points: Some(vec![0, 3, 4, 7, 8, 11, 12, 15, 17, 19, 21]),
        };
        let deserialized: PackedPoints = otspec::de::from_bytes(&packed).unwrap();
        assert_eq!(deserialized, expected);
    }
}
