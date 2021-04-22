use otspec::read_field;
use otspec::types::*;
use serde::de::DeserializeSeed;
use serde::de::SeqAccess;
use serde::de::Visitor;
use serde::ser::SerializeSeq;
use serde::Serialize;
use serde::Serializer;
use std::fmt;
extern crate otspec;

#[derive(Debug, PartialEq, Serialize)]
pub struct Metric {
    pub advanceWidth: u16,
    pub lsb: int16,
}

#[derive(Debug, PartialEq)]
pub struct hmtx {
    pub metrics: Vec<Metric>,
}

pub struct HmtxDeserializer {
    numberOfHMetrics: uint16,
}

impl hmtx {
    pub fn to_bytes(&self) -> (Vec<u8>, uint16) {
        let mut num_h_metrics = self.metrics.len() - 1;
        while num_h_metrics > 0
            && self.metrics[num_h_metrics - 1].advanceWidth
                == self.metrics[num_h_metrics].advanceWidth
        {
            num_h_metrics -= 1;
        }
        let mut bytes: Vec<u8> = vec![];

        for (i, metric) in self.metrics.iter().enumerate() {
            if i <= num_h_metrics {
                bytes.extend(otspec::ser::to_bytes(&metric).unwrap());
            } else {
                bytes.extend(otspec::ser::to_bytes(&metric.lsb).unwrap());
            }
        }

        (bytes, num_h_metrics as u16)
    }
}
impl<'de> DeserializeSeed<'de> for HmtxDeserializer {
    type Value = hmtx;

    fn deserialize<D>(self, deserializer: D) -> std::result::Result<Self::Value, D::Error>
    where
        D: serde::de::Deserializer<'de>,
    {
        struct HmtxDeserializerVisitor {
            numberOfHMetrics: uint16,
        }

        impl<'de> Visitor<'de> for HmtxDeserializerVisitor {
            type Value = hmtx;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                write!(formatter, "a hmtx table")
            }

            fn visit_seq<A>(self, mut seq: A) -> std::result::Result<hmtx, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut res = hmtx {
                    metrics: Vec::new(),
                };
                for _ in 0..self.numberOfHMetrics {
                    let advanceWidth = read_field!(seq, uint16, "an advance width");
                    let lsb = read_field!(seq, int16, "a LSB");
                    res.metrics.push(Metric { advanceWidth, lsb })
                }
                if let Some(otherMetrics) = seq.next_element::<Vec<int16>>()? {
                    let last = res
                        .metrics
                        .last()
                        .expect("Must be one advance width in hmtx!")
                        .advanceWidth;
                    res.metrics.extend(otherMetrics.iter().map(|x| Metric {
                        lsb: *x,
                        advanceWidth: last,
                    }))
                }
                Ok(res)
            }
        }

        deserializer.deserialize_seq(HmtxDeserializerVisitor {
            numberOfHMetrics: self.numberOfHMetrics,
        })
    }
}

impl Serialize for hmtx {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // We'll do this elsewhere
        panic!(
            "loca cannot be serialized directly. Call compile_glyf_loca_maxp on the font instead"
        )
    }
}

pub fn from_bytes(s: &[u8], numberOfHMetrics: uint16) -> otspec::error::Result<hmtx> {
    let mut deserializer = otspec::de::Deserializer::from_bytes(s);
    let cs: HmtxDeserializer = HmtxDeserializer { numberOfHMetrics };
    cs.deserialize(&mut deserializer)
}

#[cfg(test)]
mod tests {
    use crate::hmtx::{self, Metric};

    #[test]
    fn hmtx_de_16bit() {
        let binary_hmtx = vec![
            0x02, 0xf4, 0x00, 0x05, 0x02, 0xf4, 0x00, 0x05, 0x02, 0x98, 0x00, 0x1e, 0x02, 0xf4,
            0x00, 0x05, 0x00, 0xc8, 0x00, 0x00, 0x02, 0x58, 0x00, 0x1d, 0x02, 0x58, 0x00, 0x1d,
            0x00, 0x0a, 0xff, 0x73,
        ];
        let fhmtx = hmtx::from_bytes(&binary_hmtx, 8).unwrap();
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
