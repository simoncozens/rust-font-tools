/// Packed points within a Tuple Variation Store
use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};

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

impl Deserialize for PackedPoints {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let count1_u8: u8 = c.de()?;
        let mut count: u16 = count1_u8 as u16;
        if count > 127 {
            let count2: u8 = c.de()?;
            count = (count & 0xff) << 8 | count2 as u16;
        }
        if count == 0 {
            // All of them
            return Ok(PackedPoints { points: None });
        }
        let mut res = vec![];
        while res.len() < count as usize {
            let control_byte: u8 = c.de()?;
            let points_are_words = (control_byte & 0x80) > 0;
            // "The low 7 bits specify the number of elements in the run minus 1."
            // MINUS ONE.
            let run_count = (control_byte & 0x7f) + 1;
            let deltas: Vec<u16> = if points_are_words {
                c.de_counted(run_count.into())?
            } else {
                let delta_bytes: Vec<u8> = c.de_counted(run_count.into())?;
                delta_bytes.iter().map(|x| *x as u16).collect()
            };
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
}

impl Serialize for PackedPoints {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        if self.points.is_none() {
            return data.put(0_u8);
        }
        let points = self.points.as_ref().unwrap();
        let num_points = points.len() as uint16;
        if num_points <= 0x80 {
            data.put(num_points as u8)?;
        } else {
            data.put(num_points | 0x8000)?;
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
            data.put(run)?
        }
        Ok(())
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

    #[test]
    fn test_packed_point_zero_ser() {
        let expected = vec![0x01, 0x00, 0x00];
        let object = PackedPoints {
            points: Some(vec![0]),
        };
        let serialized = otspec::ser::to_bytes(&object).unwrap();
        assert_eq!(serialized, expected);
    }
}
