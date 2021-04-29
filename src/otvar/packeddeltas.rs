use otspec::de::CountedDeserializer;
use otspec::types::*;
use otspec::{read_field, read_field_counted, stateful_deserializer};
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;

#[derive(Debug, PartialEq)]
pub struct PackedDeltas {
    pub deltas: Option<Vec<int16>>,
}

stateful_deserializer!(
    PackedDeltas,
    PackedDeltasDeserializer,
    { num_points: uint16 },
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut res = vec![];
        while res.len() < self.num_points as usize {
            let control_byte = read_field!(seq, u8, "a packed point control byte");
            let deltas_are_words = (control_byte & 0x40) > 0;
            // "The low 6 bits specify the number of delta values in the run minus 1."
            // MINUS ONE.
            let run_count = (control_byte & 0x3f) + 1;
            let deltas: Vec<i16>;
            if control_byte & 0x80 > 0 {
                deltas = std::iter::repeat(0).take(run_count as usize).collect();
            } else if deltas_are_words {
                deltas = read_field_counted!(seq, run_count, "packed points");
            } else {
                let delta_bytes: Vec<i8> = read_field_counted!(seq, run_count, "packed points");
                deltas = delta_bytes.iter().map(|x| *x as i16).collect();
            }
            res.extend(deltas);
        }
        Ok(PackedDeltas { deltas: Some(res) })
    }
);

pub fn from_bytes(s: &[u8], num_points: u16) -> otspec::error::Result<PackedDeltas> {
    let mut deserializer = otspec::de::Deserializer::from_bytes(s);
    let cs = PackedDeltasDeserializer { num_points };
    cs.deserialize(&mut deserializer)
}

#[cfg(test)]
mod tests {
    use crate::otvar::packeddeltas::from_bytes;
    use crate::otvar::PackedDeltas;

    #[test]
    fn test_packed_delta_de() {
        let packed = vec![
            0x03, 0x0a, 0x97, 0x00, 0xc6, 0x87, 0x41, 0x10, 0x22, 0xfb, 0x34,
        ];
        let expected = PackedDeltas {
            deltas: Some(vec![10, -105, 0, -58, 0, 0, 0, 0, 0, 0, 0, 0, 4130, -1228]),
        };
        let deserialized: PackedDeltas = from_bytes(&packed, 14).unwrap();
        assert_eq!(deserialized, expected);
    }
}
