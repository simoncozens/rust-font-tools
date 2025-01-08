use core::fmt;
use std::error::Error;

use itertools::Itertools;
use serde::de::Visitor;
use serde::ser::SerializeSeq;

// Well, this is going to get interesting.
pub(crate) trait CurlyBraceReceiver<T, const N: usize> {
    fn new(parts: [T; N]) -> Self;
    fn as_parts(&self) -> [T; N];
}

impl CurlyBraceReceiver<f32, 2> for (f32, f32) {
    fn new(parts: [f32; 2]) -> Self {
        (parts[0], parts[1])
    }
    fn as_parts(&self) -> [f32; 2] {
        [self.0, self.1]
    }
}

pub(crate) struct CurlyBraceVisitor<const SIZE: usize, T>
where
    T: CurlyBraceReceiver<f32, SIZE>, // Maybe there's an argument for being EVEN MORE GENERIC but I think we're quite generic enough
{
    pub(crate) _marker: std::marker::PhantomData<T>,
}

impl<const SIZE: usize, T> Default for CurlyBraceVisitor<SIZE, T>
where
    T: CurlyBraceReceiver<f32, SIZE>,
{
    fn default() -> Self {
        CurlyBraceVisitor {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<'de, const SIZE: usize, T> Visitor<'de> for CurlyBraceVisitor<SIZE, T>
where
    T: CurlyBraceReceiver<f32, SIZE>,
{
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a string with curly braces (e.g. \"{800, 15}\")")
    }

    fn visit_str<E>(self, value: &str) -> Result<T, E>
    where
        E: serde::de::Error,
    {
        let parts = value.trim_matches(|c| c == '{' || c == '}').split(',');
        let part_len = parts.clone().count();
        if part_len != SIZE {
            return Err(E::custom(format!(
                "wrong number of parts: expected {}, found {}",
                SIZE, part_len
            )));
        }
        Ok(T::new(
            parts
                .map(|s| {
                    s.trim()
                        .parse::<f32>()
                        .map_err(|e| E::custom(format!("failed to parse '{}' as f32: {}", s, e)))
                })
                .collect::<Result<Vec<_>, _>>()?
                .try_into()
                .map_err(|e: Vec<f32>| {
                    E::custom(format!("failed to parse '{}' as f32: got {:?}", value, e))
                })?,
        ))
    }
}

pub(crate) fn serialize_commify<S, T, const SIZE: usize>(
    value: &T,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
    T: CurlyBraceReceiver<f32, SIZE>,
{
    let mut seq = serializer.serialize_seq(None)?;
    let middle: String = value
        .as_parts()
        .into_iter()
        .map(|x| x.to_string())
        .join(",");
    seq.serialize_element(&format!("{{{}}}", middle))?;
    seq.end()
}

pub(crate) fn deserialize_commify<'de, D, T, const SIZE: usize>(
    deserializer: D,
) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: CurlyBraceReceiver<f32, SIZE>,
{
    deserializer.deserialize_str(CurlyBraceVisitor::<SIZE, T>::default())
}

// So complicated our nice generic solution above doesn't work
pub(crate) trait CropRectReceiver {
    fn new(top: i32, left: i32, bottom: i32, right: i32) -> Self;
}
pub(crate) struct CropRectVisitor<T: CropRectReceiver> {
    _marker: std::marker::PhantomData<T>,
}

impl<T> Default for CropRectVisitor<T>
where
    T: CropRectReceiver,
{
    fn default() -> Self {
        CropRectVisitor {
            _marker: std::marker::PhantomData,
        }
    }
}
impl<'de, T> Visitor<'de> for CropRectVisitor<T>
where
    T: CropRectReceiver,
{
    type Value = T;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("a crop rectangle (e.g. \"{{1,2},{3,4}}\")")
    }

    fn visit_str<E>(self, value: &str) -> Result<T, E>
    where
        E: serde::de::Error,
    {
        let chunks = value
            .chars()
            .chunk_by(|&element| element != '{' && element != '}' && element != ',');
        let mut number_groups = chunks
            .into_iter()
            .filter(|(k, _v)| *k)
            .map(|(_k, v)| v.collect::<String>());
        let top = number_groups
            .next()
            .ok_or_else(|| E::custom("missing top"))?
            .parse::<i32>()
            .map_err(|_| E::custom("top not a number"))?;
        let left = number_groups
            .next()
            .ok_or_else(|| E::custom("missing left"))?
            .parse::<i32>()
            .map_err(|_| E::custom("left not a number"))?;
        let bottom = number_groups
            .next()
            .ok_or_else(|| E::custom("missing bottom"))?
            .parse::<i32>()
            .map_err(|_| E::custom("bottom not a number"))?;
        let right = number_groups
            .next()
            .ok_or_else(|| E::custom("missing right"))?
            .parse::<i32>()
            .map_err(|_| E::custom("right not a number"))?;
        Ok(T::new(top, left, bottom, right))
    }
}
