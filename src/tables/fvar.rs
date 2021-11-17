use otspec::types::*;
use otspec::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};
use otspec_macros::tables;

/// The 'fvar' OpenType tag.
pub const TAG: Tag = crate::tag!("fvar");

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
#[derive(Debug, PartialEq, Clone)]
#[allow(non_snake_case)]
pub struct InstanceRecord {
    /// The name ID for entries in the 'name' table that provide subfamily names for this instance.
    pub subfamilyNameID: uint16,
    /// Flags (unused)
    pub flags: uint16,
    /// Location of this instance in the design space.
    pub coordinates: Tuple,
    /// The name ID for entries in the 'name' table that provide PostScript names for this instance.
    pub postscriptNameID: Option<uint16>,
}

impl InstanceRecord {
    #[allow(non_snake_case)]
    fn from_bytes(
        c: &mut ReaderContext,
        axis_count: uint16,
        has_postscript_name_id: bool,
    ) -> Result<Self, DeserializationError> {
        let subfamilyNameID = c.de()?;
        let flags: uint16 = c.de()?;
        let coordinates: Vec<f32> = c
            .de_counted(axis_count.into())?
            .iter()
            .map(|x: &Fixed| (*x).into())
            .collect();
        let postscriptNameID: Option<uint16> = if has_postscript_name_id {
            Some(c.de()?)
        } else {
            None
        };
        Ok(InstanceRecord {
            subfamilyNameID,
            flags,
            coordinates,
            postscriptNameID,
        })
    }
}

/// Represents a font's fvar (Font Variations) table
#[derive(Clone, Debug, PartialEq)]
#[allow(non_camel_case_types)]
pub struct fvar {
    /// The font's axes of variation
    pub axes: Vec<VariationAxisRecord>,
    /// Any named instances within the design space
    pub instances: Vec<InstanceRecord>,
}

impl Deserialize for fvar {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        c.push();
        let core: fvarcore = c.de()?;
        let offset = core.axesArrayOffset as usize;
        let axis_count = core.axisCount as usize;

        c.ptr = c.top_of_table() + offset;
        let axes: Vec<VariationAxisRecord> = c.de_counted(axis_count)?;
        let instances: Result<Vec<InstanceRecord>, DeserializationError> = (0..core.instanceCount)
            .map(|_| {
                let c: Result<InstanceRecord, DeserializationError> = InstanceRecord::from_bytes(
                    c,
                    core.axisCount,
                    core.instanceSize == core.axisCount * 4 + 6,
                );
                c
            })
            .collect();
        Ok(fvar {
            axes,
            instances: instances?,
        })
    }
}

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
        core.to_bytes(data)?;
        for axis in &self.axes {
            axis.to_bytes(data)?;
        }
        for instance in &self.instances {
            // Have to do this by hand
            data.put(instance.subfamilyNameID)?;
            data.put(0_u16)?;
            let coords: Vec<Fixed> = instance
                .coordinates
                .iter()
                .map(|x: &f32| (*x).into())
                .collect();
            data.put(coords)?;
            if has_postscript_name_id {
                data.put(instance.postscriptNameID.unwrap())?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::tables::fvar::InstanceRecord;
    use crate::tag;

    #[test]
    fn fvar_de() {
        let ffvar = super::fvar {
            axes: vec![
                super::VariationAxisRecord {
                    axisTag: tag!("wght"),
                    flags: 0,
                    minValue: 200.0,
                    defaultValue: 200.0,
                    maxValue: 1000.0,
                    axisNameID: 256,
                },
                super::VariationAxisRecord {
                    axisTag: tag!("ital"),
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
                    flags: 0,
                    coordinates: vec![200.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 258,
                    flags: 0,
                    coordinates: vec![300.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 259,
                    flags: 0,
                    coordinates: vec![400.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 260,
                    flags: 0,
                    coordinates: vec![600.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 261,
                    flags: 0,
                    coordinates: vec![700.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 262,
                    flags: 0,
                    coordinates: vec![800.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 263,
                    flags: 0,
                    coordinates: vec![900.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 259,
                    flags: 0,
                    coordinates: vec![1000.0, 0.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 264,
                    flags: 0,
                    coordinates: vec![200.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 265,
                    flags: 0,
                    coordinates: vec![300.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 257,
                    flags: 0,
                    coordinates: vec![400.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 266,
                    flags: 0,
                    coordinates: vec![600.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 267,
                    flags: 0,
                    coordinates: vec![700.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 268,
                    flags: 0,
                    coordinates: vec![800.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 269,
                    flags: 0,
                    coordinates: vec![900.0, 9.0],
                    postscriptNameID: None,
                },
                InstanceRecord {
                    subfamilyNameID: 257,
                    flags: 0,
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
        let deserialized: super::fvar = otspec::de::from_bytes(&binary_fvar).unwrap();
        assert_eq!(deserialized, ffvar);
        let serialized = otspec::ser::to_bytes(&deserialized).unwrap();
        assert_eq!(serialized, binary_fvar);
    }
}
