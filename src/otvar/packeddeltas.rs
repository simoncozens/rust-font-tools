/// Packed deltas within a Tuple Variation Store
use otspec::types::*;
use otspec::{read_field, read_field_counted, stateful_deserializer};
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::{Serialize, Serializer};

/// An array of packed deltas
///
/// This is the underlying storage for delta values in the cvt and gvar tables
#[derive(Debug, PartialEq)]
pub struct PackedDeltas(pub Vec<int16>);

/// In a run control byte, signifies that the deltas are two-byte values
const DELTAS_ARE_WORDS: u8 = 0x40;
/// In a run control byte, signifies that the deltas are zero and omitted
const DELTAS_ARE_ZERO: u8 = 0x80;
/// Mask off a run control byte to find the number of deltas in the run
const DELTA_RUN_COUNT_MASK: u8 = 0x3f;

stateful_deserializer!(
    PackedDeltas,
    PackedDeltasDeserializer,
    { num_points: usize },
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let mut res = vec![];
        while res.len() < self.num_points {
            let control_byte = read_field!(seq, u8, "a packed point control byte");
            let deltas_are_words = (control_byte & DELTAS_ARE_WORDS) > 0;
            // "The low 6 bits specify the number of delta values in the run minus 1."
            // MINUS ONE.
            let run_count = (control_byte & DELTA_RUN_COUNT_MASK) + 1;
            let deltas: Vec<i16>;
            if control_byte & DELTAS_ARE_ZERO > 0 {
                deltas = std::iter::repeat(0).take(run_count as usize).collect();
            } else if deltas_are_words {
                deltas = read_field_counted!(seq, run_count, "packed points");
            } else {
                let delta_bytes: Vec<i8> = read_field_counted!(seq, run_count, "packed points");
                deltas = delta_bytes.iter().map(|x| *x as i16).collect();
            }
            res.extend(deltas);
        }
        Ok(PackedDeltas(res))
    }
);

impl Serialize for PackedDeltas {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        let mut pos = 0;
        let deltas = &self.0;
        while pos < deltas.len() {
            let mut value = deltas[pos];
            if value == 0 {
                let mut run_length = 0;
                while pos < deltas.len() && deltas[pos] == 0 {
                    run_length += 1;
                    pos += 1;
                }
                while run_length >= 64 {
                    seq.serialize_element(&(DELTAS_ARE_ZERO | 63_u8))?;
                    run_length -= 64;
                }
                if run_length > 0 {
                    seq.serialize_element(&((DELTAS_ARE_ZERO | (run_length - 1)) as u8))?;
                }
            } else if (-128..=127).contains(&value) {
                // Runs of byte values
                let mut start_of_run = pos;
                while pos < deltas.len() {
                    value = deltas[pos];
                    if !(-128..=127).contains(&value) {
                        break;
                    }
                    // Avoid a sequence of more than one zero in a run.
                    if value == 0 && pos + 1 < deltas.len() && deltas[pos + 1] == 0 {
                        break;
                    }
                    pos += 1;
                }
                let mut run_length = pos - start_of_run;
                while run_length >= 64 {
                    seq.serialize_element(&63_u8)?;
                    seq.serialize_element(
                        &(deltas[start_of_run..start_of_run + 64]
                            .iter()
                            .map(|x| *x as i8)
                            .collect::<Vec<i8>>()),
                    )?;
                    start_of_run += 64;
                    run_length -= 64;
                }
                if run_length > 0 {
                    seq.serialize_element(&((run_length - 1) as u8))?;
                    seq.serialize_element(
                        &(deltas[start_of_run..pos]
                            .iter()
                            .map(|x| *x as i8)
                            .collect::<Vec<i8>>()),
                    )?;
                }
            } else {
                // Runs of word values
                let mut start_of_run = pos;
                while pos < deltas.len() {
                    value = deltas[pos];
                    // Avoid a single zero
                    if value == 0 {
                        break;
                    }
                    // Avoid a sequence of more than one byte-value in a run.
                    if (-128..=127).contains(&value)
                        && pos + 1 < deltas.len()
                        && (-128..=127).contains(&deltas[pos + 1])
                    {
                        break;
                    }
                    pos += 1;
                }
                let mut run_length = pos - start_of_run;
                while run_length >= 64 {
                    seq.serialize_element(&(DELTAS_ARE_WORDS | 63))?;
                    seq.serialize_element(&deltas[start_of_run..(start_of_run + 64)])?;
                    start_of_run += 64;
                    run_length -= 64;
                }
                if run_length > 0 {
                    seq.serialize_element(&(DELTAS_ARE_WORDS | (run_length - 1) as u8))?;
                    seq.serialize_element(&deltas[start_of_run..pos])?;
                }
            }
        }
        seq.end()
    }
}

/// Deserialize the packed deltas array from a binary buffer.
/// The number of points must be provided.
#[allow(dead_code)] // We *do* use it, I promise. Like, just a few lines below.
pub fn from_bytes(s: &[u8], num_points: usize) -> otspec::error::Result<PackedDeltas> {
    let mut deserializer = otspec::de::Deserializer::from_bytes(s);
    let cs = PackedDeltasDeserializer { num_points };
    cs.deserialize(&mut deserializer)
}

#[cfg(test)]
mod tests {
    use crate::otvar::packeddeltas::from_bytes;
    use crate::otvar::packeddeltas::PackedDeltas;

    #[test]
    fn test_packed_delta_de() {
        let packed = vec![
            0x03, 0x0a, 0x97, 0x00, 0xc6, 0x87, 0x41, 0x10, 0x22, 0xfb, 0x34,
        ];
        let expected = PackedDeltas(vec![10, -105, 0, -58, 0, 0, 0, 0, 0, 0, 0, 0, 4130, -1228]);
        let deserialized: PackedDeltas = from_bytes(&packed, 14).unwrap();
        assert_eq!(deserialized, expected);
    }

    #[test]
    fn test_packed_delta_ser() {
        let expected = vec![
            0x03, 0x0a, 0x97, 0x00, 0xc6, 0x87, 0x41, 0x10, 0x22, 0xfb, 0x34,
        ];
        let object = PackedDeltas(vec![10, -105, 0, -58, 0, 0, 0, 0, 0, 0, 0, 0, 4130, -1228]);
        let serialized = otspec::ser::to_bytes(&object).unwrap();
        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_packed_delta_66_zeros_ser() {
        let expected = vec![0xbf, 0x81];
        let object = PackedDeltas(std::iter::repeat(0).take(66).collect());
        let serialized = otspec::ser::to_bytes(&object).unwrap();
        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_packed_delta_66_words_ser() {
        let expected = vec![
            0x7f, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01,
            0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01,
            0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01,
            0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01,
            0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01,
            0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01,
            0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01,
            0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01,
            0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01, 0x90, 0x01,
            0x90, 0x01, 0x90, 0x41, 0x01, 0x90, 0x01, 0x90,
        ];
        let object = PackedDeltas(std::iter::repeat(400).take(66).collect());
        let serialized = otspec::ser::to_bytes(&object).unwrap();
        assert_eq!(serialized, expected);
    }

    #[test]
    fn test_long_bytes_ser() {
        let expected: Vec<u8> = vec![
            0x3f, 0xf7, 0xef, 0x07, 0x20, 0x0d, 0x0c, 0x0d, 0x20, 0x0f, 0xf6, 0xef, 0xfd, 0x22,
            0x0f, 0x0e, 0x0f, 0x22, 0x05, 0xf5, 0xf0, 0xf5, 0x08, 0x20, 0x1c, 0x22, 0xe4, 0xdf,
            0xe3, 0xf6, 0x0e, 0x0a, 0x0f, 0x03, 0xe0, 0xf4, 0xf4, 0xf4, 0xe0, 0xfb, 0x09, 0x10,
            0x03, 0xde, 0xf2, 0xf2, 0xf2, 0xde, 0xfb, 0x0b, 0x10, 0x0c, 0xf9, 0xe2, 0xe5, 0xdf,
            0x1c, 0x22, 0x1e, 0x0b, 0xf2, 0xf7, 0xe5, 0xdd, 0x1c, 0x01, 0x24, 0xe5, 0x83,
        ];
        let object = PackedDeltas(vec![
            -9, -17, 7, 32, 13, 12, 13, 32, 15, -10, -17, -3, 34, 15, 14, 15, 34, 5, -11, -16, -11,
            8, 32, 28, 34, -28, -33, -29, -10, 14, 10, 15, 3, -32, -12, -12, -12, -32, -5, 9, 16,
            3, -34, -14, -14, -14, -34, -5, 11, 16, 12, -7, -30, -27, -33, 28, 34, 30, 11, -14, -9,
            -27, -35, 28, 36, -27, 0, 0, 0, 0,
        ]);
        let serialized = otspec::ser::to_bytes(&object).unwrap();
        assert_eq!(serialized, expected);
    }
}
