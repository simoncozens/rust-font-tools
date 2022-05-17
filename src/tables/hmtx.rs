use std::convert::TryInto;

use otspec::types::*;
use otspec::{DeserializationError, Deserializer, ReaderContext, Serialize};
use otspec_macros::{Deserialize, Serialize};

/// The 'hmtx' OpenType tag.
pub const TAG: Tag = crate::tag!("hmtx");

/// A single horizontal metric
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[allow(non_snake_case)]
pub struct Metric {
    /// The full horizontal advance width of the glyph
    pub advanceWidth: u16,
    /// The left side bearing of the glyph
    pub lsb: int16,
}

/// The horizontal metrics table
#[derive(Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub struct hmtx {
    /// The list of metrics, corresponding to the glyph order
    pub metrics: Vec<Metric>,
}

impl hmtx {
    /// Serialize the horizontal metrics table to a binary vector and a corresponding
    /// number of horizontal metrics (to be stored in the `hhea` table)
    pub fn to_bytes(&self) -> (Vec<u8>, uint16) {
        if self.metrics.is_empty() {
            return (vec![], 0);
        }
        let mut end_index_h_metrics = self.metrics.len() - 1;
        while end_index_h_metrics > 0
            && self.metrics[end_index_h_metrics - 1].advanceWidth
                == self.metrics[end_index_h_metrics].advanceWidth
        {
            end_index_h_metrics -= 1;
        }
        let mut bytes: Vec<u8> = vec![];

        for (i, metric) in self.metrics.iter().enumerate() {
            if i <= end_index_h_metrics {
                bytes.extend(otspec::ser::to_bytes(&metric).unwrap());
            } else {
                bytes.extend(otspec::ser::to_bytes(&metric.lsb).unwrap());
            }
        }

        (bytes, (end_index_h_metrics + 1) as u16)
    }

    /// The number of horizontal metrics (to be stored in the `hhea` table)
    pub fn number_of_hmetrics(&self) -> uint16 {
        let last = match self.metrics.last() {
            Some(metric) => metric.advanceWidth,
            None => return 0,
        };

        let dupe_widths = self
            .metrics
            .iter()
            .rev()
            .skip(1)
            .take_while(|m| m.advanceWidth == last)
            .count();
        (self.metrics.len() - dupe_widths).try_into().unwrap()
    }
}

impl Serialize for hmtx {
    fn to_bytes(
        &self,
        _: &mut std::vec::Vec<u8>,
    ) -> std::result::Result<(), otspec::SerializationError> {
        Err(otspec::SerializationError(
            "Can't serialize hmtx directly".to_string(),
        ))
    }
}

/// Deserializes a Horizontal Metrics Table given a binary vector and the
/// `numberOfHMetrics` field of the `hhea` table.
pub fn from_bytes(
    c: &mut ReaderContext,
    number_of_h_metrics: uint16,
) -> Result<hmtx, DeserializationError> {
    let mut res = hmtx {
        metrics: Vec::new(),
    };
    for _ in 0..number_of_h_metrics {
        let metric: Metric = c.de()?;
        res.metrics.push(metric)
    }
    let maybe_other_metrics: Result<Vec<int16>, DeserializationError> = c.de();
    if let Ok(other_metrics) = maybe_other_metrics {
        let last = res
            .metrics
            .last()
            .expect("Must be one advance width in hmtx!")
            .advanceWidth;
        res.metrics.extend(other_metrics.iter().map(|x| Metric {
            lsb: *x,
            advanceWidth: last,
        }))
    }
    Ok(res)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hmtx_de_16bit() {
        let mut binary_hmtx = otspec::ReaderContext::new(vec![
            0x02, 0xf4, 0x00, 0x05, 0x02, 0xf4, 0x00, 0x05, 0x02, 0x98, 0x00, 0x1e, 0x02, 0xf4,
            0x00, 0x05, 0x00, 0xc8, 0x00, 0x00, 0x02, 0x58, 0x00, 0x1d, 0x02, 0x58, 0x00, 0x1d,
            0x00, 0x0a, 0xff, 0x73,
        ]);
        let fhmtx = super::from_bytes(&mut binary_hmtx, 8).unwrap();
        let metrics = [
            Metric {
                advanceWidth: 756,
                lsb: 5,
            },
            Metric {
                advanceWidth: 756,
                lsb: 5,
            },
            Metric {
                advanceWidth: 664,
                lsb: 30,
            },
            Metric {
                advanceWidth: 756,
                lsb: 5,
            },
            Metric {
                advanceWidth: 200,
                lsb: 0,
            },
            Metric {
                advanceWidth: 600,
                lsb: 29,
            },
            Metric {
                advanceWidth: 600,
                lsb: 29,
            },
            Metric {
                advanceWidth: 10,
                lsb: -141,
            },
        ];
        // println!("{:?}", fhmtx);
        assert_eq!(fhmtx.metrics, metrics);
    }
}
