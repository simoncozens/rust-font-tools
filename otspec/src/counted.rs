use crate::{
    types, uint16, uint32, DeserializationError, Deserialize, Deserializer, ReaderContext,
    SerializationError, Serialize,
};

#[derive(Shrinkwrap, Debug, PartialEq, Eq)]
pub struct Counted<T>(pub Vec<T>);

impl<T> Serialize for Counted<T>
where
    T: Serialize,
{
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        (self.len() as uint16).to_bytes(data)?;
        self.0.to_bytes(data)?;
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        2 + self.0.ot_binary_size()
    }
    fn offset_fields(&self) -> Vec<&dyn types::OffsetMarkerTrait> {
        let mut v = vec![];
        for el in &self.0 {
            v.extend(el.offset_fields())
        }
        v
    }
}

impl<T> Deserialize for Counted<T>
where
    T: Deserialize,
{
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let len: uint16 = c.de()?;
        let mut res: Vec<T> = vec![];
        for _ in 0..len {
            res.push(c.de()?)
        }
        Ok(Counted(res))
    }
}

impl<T> From<Vec<T>> for Counted<T> {
    fn from(v: Vec<T>) -> Self {
        Counted(v)
    }
}

impl<T> From<Counted<T>> for Vec<T> {
    fn from(v: Counted<T>) -> Self {
        v.0
    }
}

impl<T> PartialEq<Vec<T>> for Counted<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &std::vec::Vec<T>) -> bool {
        &self.0 == other
    }
}

#[derive(Shrinkwrap, Debug, PartialEq, Eq)]
pub struct Counted32<T>(pub Vec<T>);

impl<T> Serialize for Counted32<T>
where
    T: Serialize,
{
    fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
        (self.len() as uint32).to_bytes(data)?;
        self.0.to_bytes(data)?;
        Ok(())
    }
    fn ot_binary_size(&self) -> usize {
        4 + self.0.ot_binary_size()
    }
    fn offset_fields(&self) -> Vec<&dyn types::OffsetMarkerTrait> {
        let mut v = vec![];
        for el in &self.0 {
            v.extend(el.offset_fields())
        }
        v
    }
}

impl<T> Deserialize for Counted32<T>
where
    T: Deserialize,
{
    fn from_bytes(c: &mut ReaderContext) -> Result<Self, DeserializationError> {
        let len: uint32 = c.de()?;
        let mut res: Vec<T> = vec![];
        for _ in 0..len {
            res.push(c.de()?)
        }
        Ok(Counted32(res))
    }
}

impl<T> From<Vec<T>> for Counted32<T> {
    fn from(v: Vec<T>) -> Self {
        Counted32(v)
    }
}

impl<T> From<Counted32<T>> for Vec<T> {
    fn from(v: Counted32<T>) -> Self {
        v.0
    }
}

impl<T> PartialEq<Vec<T>> for Counted32<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &std::vec::Vec<T>) -> bool {
        &self.0 == other
    }
}
