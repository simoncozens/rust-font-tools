use otspec::{DeserializationError, Deserializer, ReaderContext, Serialize};

/// The 'loca' OpenType tag.
pub const TAG: otspec::types::Tag = crate::tag!("loca");

/// A [`loca`] table.
///
/// [`loca`]: https://docs.microsoft.com/en-us/typography/opentype/spec/loca
#[allow(non_snake_case, non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct loca {
    /// The offset position of each glyph in the font.
    ///
    /// Offsets are relative to the begining of the [`glyf`] table.
    ///
    /// [`glyf`]: super::glyf::glyf
    pub indices: Vec<Option<u32>>,
}

pub(crate) fn from_bytes(
    c: &mut ReaderContext,
    loca_is_32bit: bool,
) -> Result<loca, DeserializationError> {
    let mut res = loca {
        indices: Vec::new(),
    };
    let raw_indices: Vec<u32> = if loca_is_32bit {
        c.de()?
    } else {
        let x: Vec<u16> = c.de()?;
        x.iter().map(|x| (*x as u32) * 2).collect()
    };
    if raw_indices.is_empty() {
        // No glyphs, eh?
        return Ok(res);
    }
    for ab in raw_indices.windows(2) {
        if let [a, b] = ab {
            if *a == *b {
                res.indices.push(None);
            } else {
                res.indices.push(Some(*a));
            }
        }
    }
    Ok(res)
}

impl Serialize for loca {
    fn to_bytes(
        &self,
        _: &mut std::vec::Vec<u8>,
    ) -> std::result::Result<(), otspec::SerializationError> {
        Err(otspec::SerializationError(
            "Can't serialize loca directly".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use otspec::ReaderContext;

    #[test]
    fn loca_de_16bit() {
        let binary_loca = vec![0x00, 0x00, 0x01, 0x30, 0x01, 0x30, 0x01, 0x4c];
        let mut reader = ReaderContext::new(binary_loca);
        let floca = super::from_bytes(&mut reader, false).unwrap();
        let locations = [Some(0), None, Some(608)];
        // println!("{:?}", floca);
        assert_eq!(floca.indices, locations);
    }
}
