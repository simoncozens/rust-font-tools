use bitflags::bitflags;
use otspec::{types::*, Deserialize, Serialize};
use otspec::{Deserializer, ReaderContext};
use otspec_macros::{Deserialize, Serialize};

// These things are serialized/deserialized weird, so we do it by hand

#[derive(Debug, Clone, PartialEq, Copy)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct ValueRecord {
    pub xPlacement: Option<int16>,
    pub yPlacement: Option<int16>,
    pub xAdvance: Option<int16>,
    pub yAdvance: Option<int16>,
    // xPlaDeviceOffset: Offset16<Device>,
    // yPlaDeviceOffset: Offset16<Device>,
    // xAdvDeviceOffset: Offset16<Device>,
    // yAdvDeviceOffset: Offset16<Device>,
}

bitflags! {
    /// Flags used when serializing/deserializing the value record.
    ///
    /// These are computed automatically, so you don't need to worry about them.
    #[derive(Serialize, Deserialize)]
    pub struct ValueRecordFlags: u16 {
            ///	Includes horizontal adjustment for placement
            const X_PLACEMENT = 0x0001;
            ///	Includes vertical adjustment for placement
            const Y_PLACEMENT = 0x0002;
            ///	Includes horizontal adjustment for advance
            const X_ADVANCE = 0x0004;
            ///	Includes vertical adjustment for advance
            const Y_ADVANCE = 0x0008;
            ///	Includes Device table (non-variable font) / VariationIndex table (variable font) for horizontal placement
            const X_PLACEMENT_DEVICE = 0x0010;
            ///	Includes Device table (non-variable font) / VariationIndex table (variable font) for vertical placement
            const Y_PLACEMENT_DEVICE = 0x0020;
            ///	Includes Device table (non-variable font) / VariationIndex table (variable font) for horizontal advance
            const X_ADVANCE_DEVICE = 0x0040;
            ///	Includes Device table (non-variable font) / VariationIndex table (variable font) for vertical advance
            const Y_ADVANCE_DEVICE = 0x0080;
    }
}

impl ValueRecord {
    pub fn new() -> ValueRecord {
        ValueRecord {
            xPlacement: None,
            yPlacement: None,
            xAdvance: None,
            yAdvance: None,
        }
    }
    fn flags(&self) -> ValueRecordFlags {
        let mut f = ValueRecordFlags::empty();
        if self.xPlacement.is_some() {
            f = f | ValueRecordFlags::X_PLACEMENT
        }
        if self.yPlacement.is_some() {
            f = f | ValueRecordFlags::Y_PLACEMENT
        }
        if self.xAdvance.is_some() {
            f = f | ValueRecordFlags::X_ADVANCE
        }
        if self.yAdvance.is_some() {
            f = f | ValueRecordFlags::Y_ADVANCE
        }
        f
    }
}

impl Serialize for ValueRecord {
    fn to_bytes(&self, output: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        self.flags().to_bytes(output)?;
        self.xPlacement.to_bytes(output)?;
        self.yPlacement.to_bytes(output)?;
        self.xAdvance.to_bytes(output)?;
        self.yAdvance.to_bytes(output)
    }
}

impl Deserialize for ValueRecord {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, otspec::DeserializationError> {
        let flags: ValueRecordFlags = c.de()?;
        let mut vr = ValueRecord::new();
        if flags.contains(ValueRecordFlags::X_PLACEMENT) {
            vr.xPlacement = Some(c.de()?);
        }
        if flags.contains(ValueRecordFlags::Y_PLACEMENT) {
            vr.yPlacement = Some(c.de()?);
        }
        if flags.contains(ValueRecordFlags::X_ADVANCE) {
            vr.xAdvance = Some(c.de()?);
        }
        if flags.contains(ValueRecordFlags::Y_ADVANCE) {
            vr.yAdvance = Some(c.de()?);
        }

        Ok(vr)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueRecords(pub Vec<ValueRecord>);

impl Serialize for ValueRecords {
    fn to_bytes(&self, output: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
        let mut flag = ValueRecordFlags::empty();
        for vr in &self.0 {
            flag |= vr.flags();
        }
        for _vr in &self.0 {
            unimplemented!()
        }
        Ok(())
    }
}

impl Deserialize for ValueRecords {
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, otspec::DeserializationError> {
        let flags: ValueRecordFlags = c.de()?;
        let count: uint16 = c.de()?;
        let mut v = ValueRecords(vec![]);
        for _ in 0..count {
            let mut vr = ValueRecord::new();
            if flags.contains(ValueRecordFlags::X_PLACEMENT) {
                vr.xPlacement = Some(c.de()?);
            }
            if flags.contains(ValueRecordFlags::Y_PLACEMENT) {
                vr.yPlacement = Some(c.de()?);
            }
            if flags.contains(ValueRecordFlags::X_ADVANCE) {
                vr.xAdvance = Some(c.de()?);
            }
            if flags.contains(ValueRecordFlags::Y_ADVANCE) {
                vr.yAdvance = Some(c.de()?);
            }
            v.0.push(vr)
        }

        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valuerecord_serde() {
        let mut vr = ValueRecord::new();
        vr.xAdvance = Some(-120);
        assert_eq!(vr.flags(), ValueRecordFlags::X_ADVANCE);
        let binary = otspec::ser::to_bytes(&vr).unwrap();
        assert_eq!(binary, vec![0x00, 0x04, 0xff, 0x88,]);
        let de: ValueRecord = otspec::de::from_bytes(&binary).unwrap();
        assert_eq!(de, vr);
    }
}
