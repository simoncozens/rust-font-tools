use crate::common::FormatSpecific;
use crate::i18ndictionary::I18NDictionary;
use crate::BabelfontError;
use fontdrasil::coords::{CoordConverter, DesignCoord, NormalizedCoord, UserCoord};
use uuid::Uuid;
pub use write_fonts::types::Tag;

#[derive(Debug, Clone, Default)]
pub struct Axis {
    pub name: I18NDictionary,
    pub tag: Tag,
    pub id: Uuid,
    pub min: Option<UserCoord>,
    pub max: Option<UserCoord>,
    pub default: Option<UserCoord>,
    pub map: Option<Vec<(UserCoord, DesignCoord)>>,
    pub hidden: bool,
    pub formatspecific: FormatSpecific,
}

impl Axis {
    pub fn new<T>(name: T, tag: Tag) -> Self
    where
        T: Into<I18NDictionary>,
    {
        Axis {
            name: name.into(),
            tag,
            id: Uuid::new_v4(),
            ..Default::default()
        }
    }

    fn _converter(&self) -> Result<CoordConverter, BabelfontError> {
        if let Some(map) = self.map.as_ref() {
            // Find default
            let default_idx = map
                .iter()
                .position(|(coord, _)| Some(coord) == self.default.as_ref())
                .ok_or_else(|| BabelfontError::IllDefinedAxis {
                    axis_name: self.name(),
                })?;
            Ok(CoordConverter::new(map.to_vec(), default_idx))
        } else {
            let (min, default, max) =
                self.bounds()
                    .ok_or_else(|| BabelfontError::IllDefinedAxis {
                        axis_name: self.name(),
                    })?;
            Ok(CoordConverter::unmapped(min, default, max))
        }
    }

    pub fn bounds(&self) -> Option<(UserCoord, UserCoord, UserCoord)> {
        if self.min.is_none() || self.default.is_none() || self.max.is_none() {
            return None;
        }
        Some((self.min.unwrap(), self.default.unwrap(), self.max.unwrap()))
    }

    /// Converts a position on this axis from designspace coordinates to userspace coordinates
    pub fn designspace_to_userspace(&self, l: DesignCoord) -> Result<UserCoord, BabelfontError> {
        self._converter().map(|c| l.convert(&c))
    }

    /// Converts a position on this axis in userspace coordinates to designspace coordinates
    pub fn userspace_to_designspace(&self, l: UserCoord) -> Result<DesignCoord, BabelfontError> {
        self._converter().map(|c| l.convert(&c))
    }

    pub fn normalize_userspace_value(
        &self,
        l: UserCoord,
    ) -> Result<NormalizedCoord, BabelfontError> {
        self._converter().map(|c| l.convert(&c))
    }

    pub fn normalize_designspace_value(
        &self,
        l: DesignCoord,
    ) -> Result<NormalizedCoord, BabelfontError> {
        self._converter().map(|c| l.convert(&c))
    }

    pub fn name(&self) -> String {
        self.name
            .get_default()
            .unwrap_or(&"Unnamed axis".to_string())
            .to_string()
    }
}

impl TryInto<fontdrasil::types::Axis> for Axis {
    type Error = BabelfontError;

    fn try_into(self) -> Result<fontdrasil::types::Axis, Self::Error> {
        let (min, default, max) = self
            .bounds()
            .ok_or_else(|| BabelfontError::IllDefinedAxis {
                axis_name: self
                    .name
                    .get_default()
                    .unwrap_or(&"Unnamed axis".to_string())
                    .to_string(),
            })?;
        let converter = self._converter()?;
        Ok(fontdrasil::types::Axis {
            name: self.name(),
            tag: self.tag,
            min,
            max,
            default,
            hidden: self.hidden,
            converter,
        })
    }
}

impl TryInto<fontdrasil::types::Axis> for &Axis {
    type Error = BabelfontError;
    fn try_into(self) -> Result<fontdrasil::types::Axis, Self::Error> {
        let (min, default, max) = self
            .bounds()
            .ok_or_else(|| BabelfontError::IllDefinedAxis {
                axis_name: self
                    .name
                    .get_default()
                    .unwrap_or(&"Unnamed axis".to_string())
                    .to_string(),
            })?;
        let converter = self._converter()?;
        Ok(fontdrasil::types::Axis {
            name: self.name(),
            tag: self.tag,
            min,
            max,
            default,
            hidden: self.hidden,
            converter,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    macro_rules! uc {
        ($val:expr) => {
            UserCoord::new($val)
        };
    }
    macro_rules! dc {
        ($val:expr) => {
            DesignCoord::new($val)
        };
    }

    #[test]
    fn test_linear_map() {
        let mut weight = Axis::new("Weight".to_string(), Tag::from_be_bytes(*b"wght"));
        weight.min = Some(uc!(100.0));
        weight.max = Some(uc!(900.0));
        weight.default = Some(uc!(100.0));
        weight.map = Some(vec![(uc!(100.0), dc!(10.0)), (uc!(900.0), dc!(90.0))]);

        assert_eq!(weight.userspace_to_designspace(uc!(400.0)).unwrap(), 40.0);
        assert_eq!(
            weight.designspace_to_userspace(dc!(40.0)).unwrap(),
            uc!(400.0)
        );
    }

    #[test]
    fn test_nonlinear_map() {
        let mut weight = Axis::new("Weight".to_string(), Tag::from_be_bytes(*b"wght"));
        weight.min = Some(uc!(200.0));
        weight.max = Some(uc!(1000.0));
        weight.default = Some(uc!(200.0));
        weight.map = Some(vec![
            (uc!(200.0), dc!(42.0)),
            (uc!(300.0), dc!(61.0)),
            (uc!(400.0), dc!(81.0)),
            (uc!(600.0), dc!(101.0)),
            (uc!(700.0), dc!(125.0)),
            (uc!(800.0), dc!(151.0)),
            (uc!(900.0), dc!(178.0)),
            (uc!(1000.0), dc!(208.0)),
        ]);

        assert_eq!(
            weight.userspace_to_designspace(uc!(250.0)).unwrap(),
            dc!(51.5)
        );
        assert_eq!(
            weight.designspace_to_userspace(dc!(138.0)).unwrap(),
            uc!(750.0)
        );
    }

    #[test]
    fn test_normalize_map() {
        let mut opsz = Axis::new("Optical Size".to_string(), Tag::from_be_bytes(*b"opsz"));
        opsz.min = Some(uc!(17.0));
        opsz.max = Some(uc!(18.0));
        opsz.default = Some(uc!(18.0));
        opsz.map = Some(vec![
            (uc!(17.0), dc!(17.0)),
            (uc!(17.99), dc!(17.1)),
            (uc!(18.0), dc!(18.0)),
        ]);
        assert_eq!(opsz.normalize_userspace_value(uc!(17.99)).unwrap(), -0.01);
        assert_eq!(opsz.normalize_designspace_value(dc!(17.1)).unwrap(), -0.9);
    }
}
