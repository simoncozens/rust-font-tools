use crate::types::*;
use crate::{
    DeserializationError, Deserialize, Deserializer, ReaderContext, SerializationError, Serialize,
    Serializer,
};

// These things have to be serialized/deserialized by hand because of annoying
// format switching things.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
#[allow(missing_docs, non_snake_case, non_camel_case_types)]
pub struct Anchor {
    pub xCoordinate: int16,
    pub yCoordinate: int16,
    pub anchorPoint: Option<uint16>,
    // xDeviceOffset: Option<Offset16<Device>>,
    // yDeviceOffset: Option<Offset16<Device>>,
}

impl Anchor {
    /// Returns a new anchor with no anchor point
    pub fn new(x: int16, y: int16) -> Anchor {
        Anchor {
            xCoordinate: x,
            yCoordinate: y,
            anchorPoint: None,
        }
    }
}
impl Deserialize for Anchor {
    #[allow(non_snake_case)]
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let format: uint16 = c.de()?;
        let xCoordinate: int16 = c.de()?;
        let yCoordinate: int16 = c.de()?;
        if format == 1 {
            Ok(Anchor {
                xCoordinate,
                yCoordinate,
                anchorPoint: None,
            })
        } else if format == 2 {
            let anchorPoint: uint16 = c.de()?;
            Ok(Anchor {
                xCoordinate,
                yCoordinate,
                anchorPoint: Some(anchorPoint),
            })
        } else {
            Err(DeserializationError(format!(
                "Invalid anchor format {:}",
                format
            )))
        }
    }
}

impl Serialize for Anchor {
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        let format: uint16 = if self.anchorPoint.is_some() { 2 } else { 1 };
        data.put(format)?;
        data.put(self.xCoordinate)?;
        data.put(self.yCoordinate)?;
        if let Some(anchor) = self.anchorPoint {
            data.put(anchor)?;
        }
        Ok(())
    }
}
