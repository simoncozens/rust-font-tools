#![allow(non_camel_case_types, non_snake_case)]

use otspec::types::*;
use otspec::*;
use otspec_macros::tables;

tables!(
    fvarcore {
        uint16 majorVersion
        uint16 minorVersion
        uint16 axesArrayOffset
        uint16 reserved
        uint16 axisCount
        uint16 axisSize
        uint16 instanceCount
        uint16 instanceSize
    }
    VariationAxisRecord {
        Tag axisTag
        Fixed   minValue
        Fixed   defaultValue
        Fixed   maxValue
        uint16  flags
        uint16  axisNameID
    }
);

/// Struct representing a named instance within the variable font's design space
#[derive(Debug, PartialEq)]
pub struct InstanceRecord {
    /// The name ID for entries in the 'name' table that provide subfamily names for this instance.
    pub subfamilyNameID: uint16,
    /// Location of this instance in the design space.
    pub coordinates: Tuple,
    /// The name ID for entries in the 'name' table that provide PostScript names for this instance.
    pub postscriptNameID: Option<uint16>,
}

stateful_deserializer!(
Vec<InstanceRecord>,
InstanceRecordDeserializer,
{
    axisCount: uint16,
    instanceCount: uint16,
    has_postscript_name_id: bool
},
fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Vec<InstanceRecord>, A::Error>
        where
            A: SeqAccess<'de>
        {
            let mut res = vec![];
            for _ in 0..self.instanceCount {
                let subfamilyNameID =
                    read_field!(seq, uint16, "instance record family name ID");
                let _flags = read_field!(seq, uint16, "instance record flags");
                let coordinates = (read_field_counted!(seq, self.axisCount, "a coordinate")
                    as Vec<i32>)
                    .iter()
                    .map(|x| Fixed::unpack(*x))
                    .collect();
                let postscriptNameID = if self.has_postscript_name_id {
                    Some(read_field!(
                        seq,
                        uint16,
                        "instance record postscript name ID"
                    ))
                } else {
                    None
                };
                res.push(InstanceRecord {
                    subfamilyNameID,
                    coordinates,
                    postscriptNameID,
                });
                println!("Got a record {:?}", res);
            }
            Ok(res)
        }
);

/// Represents a font's fvar (Font Variations) table
#[derive(Debug, PartialEq)]
pub struct fvar {
    /// The font's axes of variation
    pub axes: Vec<VariationAxisRecord>,
    /// Any named instances within the design space
    pub instances: Vec<InstanceRecord>,
}

impl Deserialize for fvar {}

deserialize_visitor!(
    fvar,
    FvarVisitor,
    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
        let core = read_field!(seq, fvarcore, "an fvar table header");
        let remainder = read_remainder!(seq, "an fvar table");
        let offset = core.axesArrayOffset as usize;
        let offset_base: usize = 16;
        let axis_count = core.axisCount as usize;
        let axis_records = &remainder[offset - offset_base..];
        let mut de = OTDeserializer::from_bytes(axis_records);
        let cs: CountedDeserializer<VariationAxisRecord> =
            CountedDeserializer::with_len(axis_count);
        let axes: Vec<VariationAxisRecord> = cs
            .deserialize(&mut de)
            .map_err(|_| serde::de::Error::custom("Expecting a VariationAxisRecord"))?;

        let instance_records =
            &remainder[offset - offset_base + (core.axisCount * core.axisSize) as usize..];
        let mut de2 = otspec::de::Deserializer::from_bytes(instance_records);
        let cs2 = InstanceRecordDeserializer {
            axisCount: core.axisCount,
            instanceCount: core.instanceCount,
            has_postscript_name_id: core.instanceSize == core.axisCount * 4 + 6,
        };
        let instances: Vec<InstanceRecord> = cs2.deserialize(&mut de2).map_err(|e| {
            serde::de::Error::custom(format!("Expecting a InstanceRecord: {:?}", e))
        })?;

        Ok(fvar { axes, instances })
    }
);

impl Serialize for fvar {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let has_postscript_name_id = self.instances.iter().any(|x| x.postscriptNameID.is_some());
        if has_postscript_name_id && !self.instances.iter().all(|x| x.postscriptNameID.is_some()) {
            return Err(SerializationError(
                "Inconsistent use of postscriptNameID in fvar instances".to_string(),
            ));
        }
        let core = fvarcore {
            majorVersion: 1,
            minorVersion: 0,
            axesArrayOffset: 16,
            reserved: 2,
            axisCount: self.axes.len() as uint16,
            axisSize: 20,
            instanceCount: self.instances.len() as uint16,
            instanceSize: (self.axes.len() * 4 + if has_postscript_name_id { 6 } else { 4 })
                as uint16,
        };
        core.to_bytes(&mut data)?;
        for axis in &self.axes {
            axis.to_bytes(&mut data)?;
        }
        for instance in &self.instances {
            // Have to do this by hand
            data.put(instance.subfamilyNameID)?;
            data.put(0_u16)?;
            data.put(instance.coordinates.collect::<Vec<i32>>())?;
            if has_postscript_name_id {
                data.put(instance.postscriptNameID.unwrap())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::fvar;
    use crate::fvar::InstanceRecord;
    use otspec::tag;

    #[test]
    fn fvar_de() {
        let ffvar = fvar::fvar {
            axes: vec![
                fvar::VariationAxisRecord {
                    axisTag: tag!("wght"),
                    flags: 0,
                    minValue: 200.0,
                    defaultValue: 200.0,
                    maxValue: 1000.0,
                    axisNameID: 256,
                },
                fvar::VariationAxisRecord {
                    axisTag: *b"ital",
                    flags: 0,
                    minValue: 0.0,
                    defaultValue: 0.0,
                    maxValue: 9.0,
                    axisNameID: 257,
                },
            ],
            instances: vec![
                InstanceRecord {
                    subfamilyNameID: 17,
                    coordinates: vec![200.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 258,
                    coordinates: vec![300.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 259,
                    coordinates: vec![400.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 260,
                    coordinates: vec![600.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 261,
                    coordinates: vec![700.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 262,
                    coordinates: vec![800.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 263,
                    coordinates: vec![900.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 259,
                    coordinates: vec![1000.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 264,
                    coordinates: vec![200.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 265,
                    coordinates: vec![300.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 257,
                    coordinates: vec![400.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 266,
                    coordinates: vec![600.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 267,
                    coordinates: vec![700.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 268,
                    coordinates: vec![800.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 269,
                    coordinates: vec![900.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 257,
                    coordinates: vec![1000.0, 9.0],
                    postscriptNameID: None,
                },
            ],
        };
        let binary_fvar = vec![
            0x00, 0x01, 0x00, 0x00, 0x00, 0x10, 0x00, 0x02, 0x00, 0x02, 0x00, 0x14, 0x00, 0x10,
            0x00, 0x0c, 0x77, 0x67, 0x68, 0x74, 0x00, 0xc8, 0x00, 0x00, 0x00, 0xc8, 0x00, 0x00,
            0x03, 0xe8, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x69, 0x74, 0x61, 0x6c, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x01, 0x01,
            0x00, 0x11, 0x00, 0x00, 0x00, 0xc8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x02,
            0x00, 0x00, 0x01, 0x2c, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x03, 0x00, 0x00,
            0x01, 0x90, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x04, 0x00, 0x00, 0x02, 0x58,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x05, 0x00, 0x00, 0x02, 0xbc, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x01, 0x06, 0x00, 0x00, 0x03, 0x20, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x07, 0x00, 0x00, 0x03, 0x84, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x01, 0x03, 0x00, 0x00, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x08,
            0x00, 0x00, 0x00, 0xc8, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x01, 0x09, 0x00, 0x00,
            0x01, 0x2c, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x01, 0x90,
            0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x01, 0x0a, 0x00, 0x00, 0x02, 0x58, 0x00, 0x00,
            0x00, 0x09, 0x00, 0x00, 0x01, 0x0b, 0x00, 0x00, 0x02, 0xbc, 0x00, 0x00, 0x00, 0x09,
            0x00, 0x00, 0x01, 0x0c, 0x00, 0x00, 0x03, 0x20, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00,
            0x01, 0x0d, 0x00, 0x00, 0x03, 0x84, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x01, 0x01,
            0x00, 0x00, 0x03, 0xe8, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00,
        ];
        let deserialized: fvar::fvar = otspec::de::from_bytes(&binary_fvar).unwrap();
        assert_eq!(deserialized, ffvar);
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_fvar);
    }
}
