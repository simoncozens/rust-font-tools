use bitflags::bitflags;
use otspec::types::*;
use otspec::ReaderContext;
use otspec::{
    DeserializationError, Deserialize, Deserializer, SerializationError, Serialize, Serializer,
};
use otspec_macros::{tables, Deserialize, Serialize};
use std::collections::BTreeMap;

tables!(STATcore {
    uint16 majorVersion
    uint16 minorVersion
    uint16 designAxisSize
    uint16 designAxisCount
    uint32 designAxesOffset
    uint16 axisValueCount
    uint32 offsetToAxisValueOffsets
}

AxisRecord {
    Tag axisTag
    uint16 axisNameID
    uint16 axisOrdering
}

AxisValueFormat1 {
    uint16  format
    uint16  axisIndex
    uint16  flags
    uint16  valueNameID
    Fixed   value
}

AxisValueFormat2 {
    uint16  format
    uint16  axisIndex
    uint16  flags
    uint16  valueNameID
    Fixed   nominalValue
    Fixed   rangeMinValue
    Fixed   rangeMaxValue
}

AxisValueFormat3 {
    uint16  format
    uint16  axisIndex
    uint16  flags
    uint16  valueNameID
    Fixed   value
    Fixed   linkedValue
}

AxisValueFormat4Core {
    uint16  format
    uint16  axisCount
    uint16  flags
    uint16  valueNameID
}

AxisValueFormat4AxisValue {
    uint16  axisIndex
    Fixed   value
}
);

bitflags! {
    #[derive(Serialize, Deserialize)]
    /// The following axis value table flags are defined:
    pub struct AxisValueFlags: u16 {
        /// If set, this axis value table provides axis value information that is applicable to other fonts within the same font family.
        const OLDER_SIBLING_FONT_ATTRIBUTE = 0x0001;
        /// If set, it indicates that the axis value represents the “normal” value for the axis and may be omitted when composing name strings.
        const ELIDABLE_AXIS_VALUE_NAME = 0x0002;
    }
}

// It's probably more rust-like to have an enum here, but the downside of
// that is that it forces users to care about the specific OT format they're
// representing. So I'm using a maximalist structure which gets resolved automatically
// to the right underlying format. There are times when people *do* know and care,
// hence the new_format_... functions below, but this allows for maximum flexibility.

#[derive(Debug, PartialEq, Clone)]
/// An axis value table (underlying format resolved on write)
pub struct AxisValue {
    /// Zero-base index into the axis record array identifying the axis of design variation to which the axis value table applies.
    pub axis_index: Option<uint16>,
    /// Flags
    pub flags: AxisValueFlags,
    /// The name ID for entries in the 'name' table that provide a display string for this attribute value.
    pub name_id: uint16,
    /// A numeric value for this attribute value.
    pub nominal_value: Option<f32>,
    /// The minimum and maximum values for a range associated with the specified name ID.
    pub range_min_max: Option<(f32, f32)>,
    /// The numeric value for a style-linked mapping from this value.
    pub linked_value: Option<f32>,
    /// A location at which this value applies.
    pub locations: Option<BTreeMap<uint16, f32>>,
}

#[derive(Debug, PartialEq)]
#[allow(clippy::upper_case_acronyms)]
/// The Style Attributes table
pub struct STAT {
    /// ID of fallback name when all aspects are elided
    pub elided_fallback_name_id: Option<uint16>,
    /// The design axes array
    pub design_axes: Vec<AxisRecord>,
    /// The axis value table array
    pub axis_values: Vec<AxisValue>,
}

impl Deserialize for STAT {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        c.push();
        let core: STATcore = c.de()?;
        let elided_fallback_name_id = if core.minorVersion >= 1 {
            let val: uint16 = c.de()?;
            Some(val)
        } else {
            None
        };

        // It's probably contiguous, but use the offsets anyway
        c.ptr = c.top_of_table() + core.designAxesOffset as usize;
        let design_axes: Vec<AxisRecord> = c.de_counted(core.designAxisCount.into())?;

        let start_of_offsets = c.top_of_table() + core.offsetToAxisValueOffsets as usize;
        c.ptr = start_of_offsets;
        let axis_value_offsets: Vec<uint16> = c.de_counted(core.axisValueCount.into())?;

        let mut axis_values: Vec<AxisValue> = vec![];
        for off in axis_value_offsets {
            c.ptr = start_of_offsets + off as usize;
            axis_values.push(c.de()?);
        }

        Ok(STAT {
            elided_fallback_name_id,
            design_axes,
            axis_values,
        })
    }
}

impl From<AxisValueFormat1> for AxisValue {
    fn from(af: AxisValueFormat1) -> Self {
        AxisValue {
            axis_index: Some(af.axisIndex),
            flags: AxisValueFlags::from_bits_truncate(af.flags),
            name_id: af.valueNameID,
            nominal_value: Some(af.value),
            range_min_max: None,
            linked_value: None,
            locations: None,
        }
    }
}

impl From<AxisValueFormat2> for AxisValue {
    fn from(af: AxisValueFormat2) -> Self {
        AxisValue {
            axis_index: Some(af.axisIndex),
            flags: AxisValueFlags::from_bits_truncate(af.flags),
            name_id: af.valueNameID,
            nominal_value: Some(af.nominalValue),
            range_min_max: Some((af.rangeMinValue, af.rangeMaxValue)),
            linked_value: None,
            locations: None,
        }
    }
}
impl From<AxisValueFormat3> for AxisValue {
    fn from(af: AxisValueFormat3) -> Self {
        AxisValue {
            axis_index: Some(af.axisIndex),
            flags: AxisValueFlags::from_bits_truncate(af.flags),
            name_id: af.valueNameID,
            nominal_value: Some(af.value),
            range_min_max: None,
            linked_value: Some(af.linkedValue),
            locations: None,
        }
    }
}
impl Deserialize for AxisValue {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        match c.peek(2)? {
            [0x0, 0x1] => {
                let v: AxisValueFormat1 = c.de()?;
                Ok(v.into())
            }
            [0x0, 0x2] => {
                let v: AxisValueFormat2 = c.de()?;
                Ok(v.into())
            }
            [0x0, 0x3] => {
                let v: AxisValueFormat3 = c.de()?;
                Ok(v.into())
            }
            [0x0, 0x4] => {
                let core: AxisValueFormat4Core = c.de()?;
                let values: Vec<AxisValueFormat4AxisValue> = c.de_counted(core.axisCount.into())?;
                let mut locations = BTreeMap::new();
                for v in values {
                    locations.insert(v.axisIndex, v.value);
                }
                Ok(AxisValue {
                    axis_index: None,
                    flags: AxisValueFlags::from_bits_truncate(core.flags),
                    name_id: core.valueNameID,
                    nominal_value: None,
                    range_min_max: None,
                    linked_value: None,
                    locations: Some(locations),
                })
            }
            _ => Err(DeserializationError(format!(
                "Bad STAT table axis value format {:?}",
                c.peek(2)?
            ))),
        }
    }
}

impl Serialize for STAT {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        data.put(STATcore {
            majorVersion: 1,
            minorVersion: if self.axis_values.iter().any(|x| x.locations.is_some()) {
                2
            } else {
                1
            },
            designAxisSize: 8,
            designAxisCount: self.design_axes.len() as u16,
            designAxesOffset: if self.design_axes.is_empty() { 0 } else { 20 },
            axisValueCount: self.axis_values.len() as u16,
            offsetToAxisValueOffsets: if self.axis_values.is_empty() {
                0
            } else {
                (20 + 8 * self.design_axes.len()) as u32
            },
        })?;

        data.put(self.elided_fallback_name_id.unwrap_or(17))?; // XXX

        // Design axes
        for d in &self.design_axes {
            data.put(d)?;
        }

        // Axis values
        let mut binary_axis_values: Vec<Vec<u8>> = vec![];
        let mut offset = 2 * self.axis_values.len() as u16;
        for av in &self.axis_values {
            data.put(offset)?;
            let binary_axis_value = otspec::ser::to_bytes(av)?;
            offset += binary_axis_value.len() as u16;
            binary_axis_values.push(binary_axis_value);
        }
        for bav in binary_axis_values {
            data.put(bav)?;
        }

        Ok(())
    }
}

impl Serialize for AxisValue {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        if let Some(mapping) = &self.locations {
            data.put(AxisValueFormat4Core {
                format: 4,
                axisCount: mapping.len() as u16,
                flags: self.flags.bits,
                valueNameID: self.name_id,
            })?;
            for (k, v) in mapping.iter() {
                data.put(AxisValueFormat4AxisValue {
                    axisIndex: *k,
                    value: *v,
                })?;
            }
        } else if let Some(linked) = self.linked_value {
            data.put(AxisValueFormat3 {
                format: 3,
                axisIndex: self.axis_index.unwrap(),
                flags: self.flags.bits(),
                valueNameID: self.name_id,
                value: self.nominal_value.unwrap_or(0.0),
                linkedValue: linked,
            })?;
        } else if let Some((rmin, rmax)) = self.range_min_max {
            data.put(AxisValueFormat2 {
                format: 2,
                axisIndex: self.axis_index.unwrap(),
                flags: self.flags.bits(),
                valueNameID: self.name_id,
                nominalValue: self.nominal_value.unwrap_or(0.0),
                rangeMinValue: rmin,
                rangeMaxValue: rmax,
            })?
        } else {
            data.put(AxisValueFormat1 {
                format: 1,
                axisIndex: self.axis_index.unwrap(),
                flags: self.flags.bits(),
                valueNameID: self.name_id,
                value: self.nominal_value.unwrap_or(0.0),
            })?
        }
        Ok(())
    }
}

impl AxisValue {
    /// Create a new format 1 axis value record
    pub fn new_format1(axis_index: u16, flags: AxisValueFlags, name_id: u16, value: f32) -> Self {
        AxisValue {
            axis_index: Some(axis_index),
            flags,
            name_id,
            nominal_value: Some(value),
            range_min_max: None,
            linked_value: None,
            locations: None,
        }
    }

    /// Create a new format 2 axis value record
    pub fn new_format2(
        axis_index: u16,
        flags: AxisValueFlags,
        name_id: u16,
        value: f32,
        rmin: f32,
        rmax: f32,
    ) -> Self {
        AxisValue {
            axis_index: Some(axis_index),
            flags,
            name_id,
            nominal_value: Some(value),
            range_min_max: Some((rmin, rmax)),
            linked_value: None,
            locations: None,
        }
    }

    /// Create a new format 3 axis value record
    pub fn new_format3(
        axis_index: u16,
        flags: AxisValueFlags,
        name_id: u16,
        value: f32,
        linked_value: f32,
    ) -> Self {
        AxisValue {
            axis_index: Some(axis_index),
            flags,
            name_id,
            nominal_value: Some(value),
            range_min_max: None,
            linked_value: Some(linked_value),
            locations: None,
        }
    }

    /// Create a new format 4 axis value record
    pub fn new_format4(flags: AxisValueFlags, name_id: u16, mapping: BTreeMap<u16, f32>) -> Self {
        AxisValue {
            axis_index: None,
            flags,
            name_id,
            nominal_value: None,
            range_min_max: None,
            linked_value: None,
            locations: Some(mapping),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{btreemap, tag};
    use pretty_assertions::assert_eq;
    use std::iter::FromIterator;
    #[test]
    fn test_stat_de() {
        let binary_stat = vec![
            0x00, 0x01, 0x00, 0x01, 0x00, 0x08, 0x00, 0x03, 0x00, 0x00, 0x00, 0x14, 0x00, 0x05,
            0x00, 0x00, 0x00, 0x2c, 0x00, 0x02, 0x77, 0x64, 0x74, 0x68, 0x01, 0x01, 0x00, 0x00,
            0x77, 0x67, 0x68, 0x74, 0x01, 0x00, 0x00, 0x01, 0x69, 0x74, 0x61, 0x6c, 0x01, 0x0f,
            0x00, 0x02, 0x00, 0x0a, 0x00, 0x16, 0x00, 0x22, 0x00, 0x36, 0x00, 0x4a, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x00, 0x01, 0x0d, 0x00, 0x32, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
            0x00, 0x00, 0x01, 0x0e, 0x00, 0x4b, 0x00, 0x00, 0x00, 0x02, 0x00, 0x01, 0x00, 0x00,
            0x01, 0x02, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x32, 0x00, 0x00,
            0x00, 0x02, 0x00, 0x01, 0x00, 0x00, 0x01, 0x03, 0x00, 0x64, 0x00, 0x00, 0x00, 0x32,
            0x00, 0x00, 0x00, 0x96, 0x00, 0x00, 0x00, 0x03, 0x00, 0x02, 0x00, 0x02, 0x01, 0x10,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00,
        ];
        let stat: STAT = otspec::de::from_bytes(&binary_stat).unwrap();
        assert_eq!(
            stat,
            STAT {
                elided_fallback_name_id: Some(2),
                design_axes: vec![
                    AxisRecord {
                        axisTag: tag!("wdth"),
                        axisNameID: 257,
                        axisOrdering: 0
                    },
                    AxisRecord {
                        axisTag: tag!("wght"),
                        axisNameID: 256,
                        axisOrdering: 1
                    },
                    AxisRecord {
                        axisTag: tag!("ital"),
                        axisNameID: 271,
                        axisOrdering: 2
                    },
                ],
                axis_values: vec![
                    AxisValue::new_format1(0, AxisValueFlags::empty(), 269, 50.0),
                    AxisValue::new_format1(0, AxisValueFlags::empty(), 270, 75.0),
                    AxisValue::new_format2(1, AxisValueFlags::empty(), 258, 1.0, 1.0, 50.0),
                    AxisValue::new_format2(1, AxisValueFlags::empty(), 259, 100.0, 50.0, 150.0),
                    AxisValue::new_format3(
                        2,
                        AxisValueFlags::ELIDABLE_AXIS_VALUE_NAME,
                        272,
                        0.0,
                        1.0
                    ),
                ]
            }
        );

        let serialized = &otspec::ser::to_bytes(&stat).unwrap();
        let stat2: STAT = otspec::de::from_bytes(serialized).unwrap();
        assert_eq!(stat2, stat);
    }

    #[test]
    fn test_stat_recursive() {
        let binary_stat = vec![
            0x00, 0x01, 0x00, 0x02, 0x00, 0x08, 0x00, 0x05, 0x00, 0x00, 0x00, 0x14, 0x00, 0x0e,
            0x00, 0x00, 0x00, 0x3c, 0x00, 0x02, 0x4d, 0x4f, 0x4e, 0x4f, 0x01, 0x0d, 0x00, 0x00,
            0x43, 0x41, 0x53, 0x4c, 0x01, 0x0e, 0x00, 0x01, 0x77, 0x67, 0x68, 0x74, 0x01, 0x0f,
            0x00, 0x02, 0x73, 0x6c, 0x6e, 0x74, 0x01, 0x10, 0x00, 0x03, 0x43, 0x52, 0x53, 0x56,
            0x01, 0x11, 0x00, 0x04, 0x00, 0x1c, 0x00, 0x30, 0x00, 0x44, 0x00, 0x50, 0x00, 0x5c,
            0x00, 0x68, 0x00, 0x74, 0x00, 0x80, 0x00, 0x90, 0x00, 0x9c, 0x00, 0xa8, 0x00, 0xb4,
            0x00, 0xc0, 0x00, 0xcc, 0x00, 0x04, 0x00, 0x02, 0x00, 0x02, 0x01, 0x9d, 0x00, 0x03,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x80, 0x00, 0x00, 0x04, 0x00, 0x02,
            0x00, 0x00, 0x01, 0x9e, 0x00, 0x03, 0xff, 0xf1, 0x00, 0x00, 0x00, 0x04, 0x00, 0x01,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x92, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x93, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x01, 0x00, 0x00, 0x01, 0x94, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x01,
            0x00, 0x00, 0x01, 0x0e, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00,
            0x01, 0x95, 0x01, 0x2c, 0x00, 0x00, 0x00, 0x03, 0x00, 0x02, 0x00, 0x02, 0x01, 0x96,
            0x01, 0x90, 0x00, 0x00, 0x02, 0xbc, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00,
            0x01, 0x97, 0x01, 0xf4, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x01, 0x98,
            0x02, 0x58, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x01, 0x99, 0x02, 0xbc,
            0x00, 0x00, 0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x01, 0x9a, 0x03, 0x20, 0x00, 0x00,
            0x00, 0x01, 0x00, 0x02, 0x00, 0x00, 0x01, 0x9b, 0x03, 0x84, 0x00, 0x00, 0x00, 0x01,
            0x00, 0x02, 0x00, 0x00, 0x01, 0x9c, 0x03, 0xe8, 0x00, 0x00,
        ];
        let stat: STAT = otspec::de::from_bytes(&binary_stat).unwrap();
        assert_eq!(
            stat,
            STAT {
                elided_fallback_name_id: Some(2),
                design_axes: vec![
                    AxisRecord {
                        axisTag: tag!("MONO"),
                        axisNameID: 269,
                        axisOrdering: 0
                    },
                    AxisRecord {
                        axisTag: tag!("CASL"),
                        axisNameID: 270,
                        axisOrdering: 1
                    },
                    AxisRecord {
                        axisTag: tag!("wght"),
                        axisNameID: 271,
                        axisOrdering: 2
                    },
                    AxisRecord {
                        axisTag: tag!("slnt"),
                        axisNameID: 272,
                        axisOrdering: 3
                    },
                    AxisRecord {
                        axisTag: tag!("CRSV"),
                        axisNameID: 273,
                        axisOrdering: 4
                    },
                ],
                axis_values: vec![
                    AxisValue::new_format4(
                        AxisValueFlags::ELIDABLE_AXIS_VALUE_NAME,
                        413,
                        btreemap!(3 => 0.0, 4 => 0.5)
                    ),
                    AxisValue::new_format4(
                        AxisValueFlags::empty(),
                        414,
                        btreemap!(3 => -15.0, 4 => 1.0)
                    ),
                    AxisValue::new_format1(0, AxisValueFlags::empty(), 402, 0.0),
                    AxisValue::new_format1(0, AxisValueFlags::empty(), 403, 1.0),
                    AxisValue::new_format1(1, AxisValueFlags::empty(), 404, 0.0),
                    AxisValue::new_format1(1, AxisValueFlags::empty(), 270, 1.0),
                    AxisValue::new_format1(2, AxisValueFlags::empty(), 405, 300.0),
                    AxisValue::new_format3(
                        2,
                        AxisValueFlags::ELIDABLE_AXIS_VALUE_NAME,
                        406,
                        400.0,
                        700.0
                    ),
                    AxisValue::new_format1(2, AxisValueFlags::empty(), 407, 500.0),
                    AxisValue::new_format1(2, AxisValueFlags::empty(), 408, 600.0),
                    AxisValue::new_format1(2, AxisValueFlags::empty(), 409, 700.0),
                    AxisValue::new_format1(2, AxisValueFlags::empty(), 410, 800.0),
                    AxisValue::new_format1(2, AxisValueFlags::empty(), 411, 900.0),
                    AxisValue::new_format1(2, AxisValueFlags::empty(), 412, 1000.0),
                ]
            }
        );

        let serialized = &otspec::ser::to_bytes(&stat).unwrap();
        let stat2: STAT = otspec::de::from_bytes(serialized).unwrap();
        assert_eq!(stat2, stat);
    }
}
