use crate::layout::device::Device;
use crate::types::*;
use crate::{DeserializationError, Deserializer, ReaderContext};
use bitflags::bitflags;
use otspec_macros::{Deserialize, Serialize};

use crate::utils::is_all_the_same;

// These things have to be deserialized by hand because of annoying
// data dependencies. (The flags required to deserialize them correctly
// are stored outside the structure, how "clever" is that.)
// Serialization is done automatically, but it is the owner's
// responsibility to set the Options to reflect the flags they
// have serialized elsewhere.
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
#[otspec(embedded)]
pub struct ValueRecord {
    // This is *not* an offset base!!!
    pub xPlacement: Option<int16>,
    pub yPlacement: Option<int16>,
    pub xAdvance: Option<int16>,
    pub yAdvance: Option<int16>,
    // "Offset to Device table... from beginning of the immediate parent table"
    // I can't even.
    pub xPlaDevice: Option<Offset16<Device>>,
    pub yPlaDevice: Option<Offset16<Device>>,
    pub xAdvDevice: Option<Offset16<Device>>,
    pub yAdvDevice: Option<Offset16<Device>>,
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
    /// Creates a value record
    pub fn new() -> ValueRecord {
        ValueRecord::default()
    }

    /// Returns true if any of the members are set
    pub fn has_any(&self) -> bool {
        self.xPlacement.is_some()
            || self.yPlacement.is_some()
            || self.xAdvance.is_some()
            || self.yAdvance.is_some()
            || self.xPlaDevice.is_some()
            || self.yPlaDevice.is_some()
            || self.xAdvDevice.is_some()
            || self.yAdvDevice.is_some()
    }

    /// Determines the appropriate flags to serialize a value record
    pub fn flags(&self) -> ValueRecordFlags {
        let mut f = ValueRecordFlags::empty();
        if self.xPlacement.is_some() {
            f |= ValueRecordFlags::X_PLACEMENT
        }
        if self.yPlacement.is_some() {
            f |= ValueRecordFlags::Y_PLACEMENT
        }
        if self.xAdvance.is_some() {
            f |= ValueRecordFlags::X_ADVANCE
        }
        if self.yAdvance.is_some() {
            f |= ValueRecordFlags::Y_ADVANCE
        }
        if self.xPlaDevice.is_some() {
            f |= ValueRecordFlags::X_PLACEMENT_DEVICE
        }
        if self.xPlaDevice.is_some() {
            f |= ValueRecordFlags::Y_PLACEMENT_DEVICE
        }
        if self.xAdvDevice.is_some() {
            f |= ValueRecordFlags::X_ADVANCE_DEVICE
        }
        if self.yAdvDevice.is_some() {
            f |= ValueRecordFlags::Y_ADVANCE_DEVICE
        }
        f
    }

    /// Deserializes a value record
    pub fn from_bytes(
        c: &mut ReaderContext,
        flags: ValueRecordFlags,
    ) -> Result<Self, DeserializationError> {
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
        if flags.contains(ValueRecordFlags::X_PLACEMENT_DEVICE) {
            vr.xPlaDevice = Some(c.de()?);
        }
        if flags.contains(ValueRecordFlags::Y_PLACEMENT_DEVICE) {
            vr.yPlaDevice = Some(c.de()?);
        }
        if flags.contains(ValueRecordFlags::X_ADVANCE_DEVICE) {
            vr.xAdvDevice = Some(c.de()?);
        }
        if flags.contains(ValueRecordFlags::Y_ADVANCE_DEVICE) {
            vr.yAdvDevice = Some(c.de()?);
        }

        Ok(vr)
    }

    // Only goes "up", never "down"!
    fn coerce_to_format(&mut self, flags: ValueRecordFlags) {
        if flags.contains(ValueRecordFlags::X_PLACEMENT) && self.xPlacement.is_none() {
            self.xPlacement = Some(0);
        }
        if flags.contains(ValueRecordFlags::Y_PLACEMENT) && self.yPlacement.is_none() {
            self.yPlacement = Some(0);
        }
        if flags.contains(ValueRecordFlags::X_ADVANCE) && self.xAdvance.is_none() {
            self.xAdvance = Some(0);
        }
        if flags.contains(ValueRecordFlags::Y_ADVANCE) && self.yAdvance.is_none() {
            self.yAdvance = Some(0);
        }
    }

    /// Replaces Some(0) fields with None fields to provide a compact representation of a value record
    pub fn simplify(&mut self) {
        if let Some(xp) = self.xPlacement {
            if xp == 0 {
                self.xPlacement = None;
            }
        }
        if let Some(yp) = self.yPlacement {
            if yp == 0 {
                self.yPlacement = None;
            }
        }
        if let Some(xa) = self.xAdvance {
            if xa == 0 {
                self.xAdvance = None;
            }
        }
        if let Some(ya) = self.yAdvance {
            if ya == 0 {
                self.yAdvance = None;
            }
        }
    }
}

/// Returns the "highest" value record format for an iter of valuerecords
pub fn highest_format<'a, T>(iter: T) -> ValueRecordFlags
where
    T: Iterator<Item = &'a ValueRecord>,
{
    iter.map(|x| x.flags())
        .reduce(|a, b| a | b)
        .unwrap_or_else(ValueRecordFlags::empty)
}

/// Ensure that all value records in a list have the same format
pub fn coerce_to_same_format(vrs: Vec<ValueRecord>) -> Vec<ValueRecord> {
    // Needed?
    if is_all_the_same(vrs.iter().map(|x| x.flags())) {
        return vrs;
    }
    let mut new_vec = vec![];
    let maximum = highest_format(vrs.iter());
    for mut vr in vrs {
        vr.coerce_to_format(maximum);
        new_vec.push(vr);
    }
    new_vec
}

/// Helper macro to create valuerecords from fields.
#[macro_export]
macro_rules! valuerecord {
        ($($k:ident = $v:expr),* $(,)?) => {{
	        	#[allow(unused_mut)]
            let mut v = ValueRecord::new();
            $( v.$k = Some($v); )*
            v
        }};
    }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::layout::gpos1::SinglePos;
    use crate::valuerecord;

    #[test]
    fn test_valuerecord_serde() {
        let mut vr = ValueRecord::new();
        vr.xAdvance = Some(-120);
        assert_eq!(vr.flags(), ValueRecordFlags::X_ADVANCE);
        let binary = otspec::ser::to_bytes(&vr).unwrap();
        assert_eq!(binary, vec![0xff, 0x88,]);
        let mut rc = otspec::ReaderContext::new(binary);
        let de: ValueRecord =
            ValueRecord::from_bytes(&mut rc, ValueRecordFlags::X_ADVANCE).unwrap();
        assert_eq!(de, vr);
    }

    #[test]
    fn test_valuerecord_device_deser() {
        let binary_gpos1 = vec![
            0x00, 0x01, 0x00, 0x16, 0x00, 0xFF, 0x00, 0x01, 0x00, 0x02, 0x00, 0x03, 0x00, 0x04,
            0x00, 0x1C, 0x00, 0x24, 0x00, 0x2C, 0x00, 0x34, 0x00, 0x01, 0x00, 0x01, 0x00, 0x42,
            0x00, 0x0B, 0x00, 0x0E, 0x00, 0x01, 0x81, 0x00, 0x00, 0x0D, 0x00, 0x0F, 0x00, 0x02,
            0xD0, 0x10, 0x00, 0x0B, 0x00, 0x0E, 0x00, 0x02, 0x80, 0x07, 0x00, 0x0D, 0x00, 0x0F,
            0x00, 0x03, 0x08, 0x00, 0x01, 0x00,
        ];

        let de: SinglePos = otspec::de::from_bytes(&binary_gpos1).unwrap();
        let mut vr = valuerecord!(xPlacement = 1, yPlacement = 2, xAdvance = 3, yAdvance = 4);
        vr.xPlaDevice = Some(Offset16::to(Device {
            startSize: 11,
            endSize: 14,
            deltaFormat: Some(1),
            deltaValues: vec![-2, 0, 0, 1],
        }));
        vr.yPlaDevice = Some(Offset16::to(Device {
            startSize: 13,
            endSize: 15,
            deltaFormat: Some(2),
            deltaValues: vec![-3, 0, 1],
        }));
        vr.xAdvDevice = Some(Offset16::to(Device {
            startSize: 11,
            endSize: 14,
            deltaFormat: Some(2),
            deltaValues: vec![-8, 0, 0, 7],
        }));
        vr.yAdvDevice = Some(Offset16::to(Device {
            startSize: 13,
            endSize: 15,
            deltaFormat: Some(3),
            deltaValues: vec![8, 0, 1],
        }));
        assert_eq!(de.mapping.values().next().unwrap(), &vr);

        assert_eq!(otspec::ser::to_bytes(&de).unwrap(), binary_gpos1);
    }
}
