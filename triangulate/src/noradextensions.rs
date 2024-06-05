use norad::designspace::{Axis, DesignSpaceDocument, Dimension, Source};
use norad::fontinfo::StyleMapStyle;
use otmath::{normalize_value, piecewise_linear_map, Location, VariationModel};
use std::collections::BTreeMap;
use std::path::Path;

type Tuple = Vec<f32>;
pub struct NormalizedLocation(Tuple);

pub trait BetterAxis {
    fn normalize_userspace_value(&self, l: f32) -> f32;
    fn normalize_designspace_value(&self, l: f32) -> f32;
    fn userspace_to_designspace(&self, l: f32) -> f32;
    #[allow(dead_code)]
    fn designspace_to_userspace(&self, l: f32) -> f32;
    fn default_map(&self) -> Vec<(f32, f32)>;
}

impl BetterAxis for Axis {
    fn normalize_userspace_value(&self, l: f32) -> f32 {
        log::debug!(
            "{} in userspace is {} in designspace",
            l,
            self.userspace_to_designspace(l)
        );
        self.normalize_designspace_value(self.userspace_to_designspace(l))
    }
    fn normalize_designspace_value(&self, l: f32) -> f32 {
        log::debug!("Minimum value is {}", self.minimum.unwrap_or(0.0));
        log::debug!("Maximum value is {}", self.maximum.unwrap_or(0.0));
        log::debug!(
            "Minimum value in designspace is {}",
            self.userspace_to_designspace(self.minimum.unwrap_or(0.0))
        );
        log::debug!(
            "Maximum value in designspace is {}",
            self.userspace_to_designspace(self.maximum.unwrap_or(0.0))
        );
        normalize_value(
            l,
            self.userspace_to_designspace(self.minimum.unwrap_or(0.0)),
            self.userspace_to_designspace(self.maximum.unwrap_or(0.0)),
            self.userspace_to_designspace(self.default),
        )
    }
    fn default_map(&self) -> Vec<(f32, f32)> {
        vec![
            (self.minimum.unwrap(), self.minimum.unwrap()),
            (self.default, self.default),
            (self.maximum.unwrap(), self.maximum.unwrap()),
        ]
    }

    fn userspace_to_designspace(&self, l: f32) -> f32 {
        let mapping: Vec<(f32, f32)> = self.map.as_ref().map_or_else(
            || self.default_map(),
            |map| {
                map.iter()
                    .map(|mapping| (mapping.input, mapping.output))
                    .collect()
            },
        );
        piecewise_linear_map(&mapping, l)
    }
    fn designspace_to_userspace(&self, l: f32) -> f32 {
        let mapping: Vec<(f32, f32)> = self.map.as_ref().map_or_else(
            || self.default_map(),
            |map| {
                map.iter()
                    .map(|mapping| (mapping.output, mapping.input))
                    .collect()
            },
        );

        piecewise_linear_map(&mapping, l)
    }
}

pub trait BetterDesignspace {
    fn location_to_tuple(&self, loc: &[Dimension]) -> Vec<f32>;
    fn default_master(&self) -> Option<&Source>;
    fn variation_model(&self) -> VariationModel<String>;
    fn normalize_location(&self, loc: &[Dimension]) -> NormalizedLocation;
}
impl BetterDesignspace for DesignSpaceDocument {
    /// Converts a location to a tuple
    fn location_to_tuple(&self, loc: &[Dimension]) -> Vec<f32> {
        let mut tuple = vec![];
        let defaults = self.axes.iter().map(|ax| ax.default);
        for (axis, default) in self.axes.iter().zip(defaults) {
            let name = &axis.name;
            let dim = loc.iter().find(|d| d.name == *name);
            if let Some(dim) = dim {
                tuple.push(dim.xvalue.unwrap_or(0.0));
            } else {
                tuple.push(default);
            }
        }
        tuple
    }
    fn default_master(&self) -> Option<&Source> {
        let defaults: BTreeMap<String, f32> = self
            .axes
            .iter()
            .map(|ax| (ax.name.clone(), ax.userspace_to_designspace(ax.default)))
            .collect();
        for source in self.sources.iter() {
            let mut maybe = true;
            for loc in source.location.iter() {
                if defaults.get(&loc.name) != loc.xvalue.as_ref() {
                    maybe = false;
                    break;
                }
            }
            if maybe {
                return Some(source);
            }
        }
        None
    }
    fn variation_model(&self) -> VariationModel<String> {
        let mut locations: Vec<Location<String>> = vec![];
        for source in self.sources.iter() {
            let source_loc = self.normalize_location(&source.location);
            let mut loc = Location::new();
            for (ax, iter_l) in self.axes.iter().zip(source_loc.0.iter()) {
                loc.insert(ax.tag.clone(), *iter_l);
            }
            locations.push(loc);
        }
        VariationModel::new(locations, self.axes.iter().map(|x| x.tag.clone()).collect())
    }
    fn normalize_location(&self, loc: &[Dimension]) -> NormalizedLocation {
        let mut v: Vec<f32> = vec![];
        for (ax, l) in self.axes.iter().zip(loc.iter()) {
            v.push(ax.normalize_designspace_value(l.xvalue.unwrap_or(0.0)));
        }
        NormalizedLocation(v)
    }
}

pub trait BetterSource {
    fn ufo(&self, designspace_filename: &Path) -> Result<norad::Font, norad::error::FontLoadError>;
}

impl BetterSource for Source {
    fn ufo(&self, designspace_filename: &Path) -> Result<norad::Font, norad::error::FontLoadError> {
        norad::Font::load(designspace_filename.parent().unwrap().join(&self.filename))
    }
}

pub(crate) trait FromString {
    fn from_string(s: &str) -> Self;
}

impl FromString for StyleMapStyle {
    fn from_string(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "regular" => StyleMapStyle::Regular,
            "italic" => StyleMapStyle::Italic,
            "bold" => StyleMapStyle::Bold,
            "bold italic" => StyleMapStyle::BoldItalic,
            _ => StyleMapStyle::Regular,
        }
    }
}
