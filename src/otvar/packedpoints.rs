/// Packed points within a Tuple Variation Store
use otspec::types::*;
use otspec::{deserialize_visitor, read_field, read_field_counted};
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// An array of packed points
///
/// If the option is None, then this represents all points within the glyph.
/// (Including phantom points.) This must be decoded with reference to the
/// glyph's contour and component information. If the option is Some, a vector
/// of the point numbers for which delta information is provided.
#[derive(Debug, PartialEq)]
pub struct PackedPoints {
    /// the array of points
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

impl Serialize for PackedPoints {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut seq = serializer.serialize_seq(None)?;
        if self.points.is_none() {
            seq.serialize_element::<u8>(&0)?;
            return seq.end();
        }
        let points = self.points.as_ref().unwrap();
        let num_points = points.len() as uint16;
        if num_points <= 0x80 {
            seq.serialize_element::<u8>(&(num_points as u8))?;
        } else {
            seq.serialize_element::<u16>(&(num_points | 0x8000))?;
        }

        let mut pos = 0;
        let mut last_value = 0;
        while pos < points.len() {
            let mut run: Vec<u8> = vec![0];
            let mut use_bytes: Option<bool> = None;
            while pos < points.len() && run.len() < 127 {
                let current = points[pos];
                let delta = current - last_value;
                if use_bytes.is_none() {
                    use_bytes = Some((0..=0xff).contains(&delta));
                }
                if use_bytes.unwrap() && !(0..=0xff).contains(&delta) {
                    break;
                }
                if use_bytes.unwrap() {
                    run.push(delta as u8);
                } else {
                    run.push((delta >> 8) as u8);
                    run.push((delta & 0xff) as u8);
                }
                last_value = current;
                pos += 1;
            }
            if use_bytes.unwrap() {
                run[0] = (run.len() as u8) - 2; // Don't count control byte
            } else {
                run[0] = (run.len() as u8 - 2) | 0x80;
            }
            seq.serialize_element(&run)?;
        }
        seq.end()
    }
}

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

    #[test]
    fn test_packed_point_ser() {
        let expected = vec![
            0x0b, 0x0a, 0x00, 0x03, 0x01, 0x03, 0x01, 0x03, 0x01, 0x03, 0x02, 0x02, 0x02,
        ];
        let object = PackedPoints {
            points: Some(vec![0, 3, 4, 7, 8, 11, 12, 15, 17, 19, 21]),
        };
        let serialized = otspec::ser::to_bytes(&object).unwrap();
        assert_eq!(serialized, expected);
    }
}
