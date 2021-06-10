use otspec::{DeserializationError, Deserializer, ReaderContext, Serialize};
#[allow(non_snake_case, non_camel_case_types)]
#[derive(Debug, PartialEq)]
pub struct loca {
    pub indices: Vec<Option<u32>>,
}

pub fn from_bytes(
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
    use crate::loca;
    use otspec::ReaderContext;

    #[test]
    fn loca_de_16bit() {
        let binary_loca = vec![0x00, 0x00, 0x01, 0x30, 0x01, 0x30, 0x01, 0x4c];
        let mut reader = ReaderContext::new(binary_loca);
        let floca = loca::from_bytes(&mut reader, false).unwrap();
        let locations = [Some(0), None, Some(608)];
        // println!("{:?}", floca);
        assert_eq!(floca.indices, locations);
    }
}
