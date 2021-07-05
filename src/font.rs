use crate::axis::Axis;
use crate::common::OTScalar;
use crate::common::OTValue;
use crate::common::Tag;
use crate::glyph::GlyphList;
use crate::instance::Instance;
use crate::master::Master;
use crate::names::Names;
use crate::Location;
use crate::{BabelfontError, Layer};
use chrono::Local;
use fonttools::otvar::Location as OTVarLocation;
use fonttools::otvar::{NormalizedLocation, VariationModel};

#[derive(Debug)]
pub struct Font {
    pub upm: u16,
    pub version: (u16, u16),
    pub axes: Vec<Axis>,
    pub instances: Vec<Instance>,
    pub masters: Vec<Master>,
    pub glyphs: GlyphList,
    pub note: Option<String>,
    pub date: chrono::DateTime<Local>,
    pub names: Names,
    pub custom_ot_values: Vec<OTValue>,
    // features: ????
}
impl Default for Font {
    fn default() -> Self {
        Self::new()
    }
}

impl Font {
    pub fn new() -> Self {
        Font {
            upm: 1000,
            version: (1, 0),
            axes: vec![],
            instances: vec![],
            masters: vec![],
            glyphs: GlyphList(vec![]),
            note: None,
            date: chrono::Local::now(),
            names: Names::new(),
            custom_ot_values: vec![],
        }
    }

    pub fn default_location(&self) -> Location {
        Location(
            self.axes
                .iter()
                .map(|axis| {
                    (
                        axis.tag.clone(),
                        axis.map_forward(axis.default.unwrap_or(0.0)),
                    )
                })
                .collect(),
        )
    }
    pub fn default_master(&self) -> Option<&Master> {
        let default_location: Location = self.default_location();
        for m in &self.masters {
            if m.location == default_location {
                return Some(m);
            }
        }
        None
    }

    pub fn default_master_index(&self) -> Option<usize> {
        let default_location: Location = self.default_location();
        for (ix, m) in self.masters.iter().enumerate() {
            if m.location == default_location {
                return Some(ix);
            }
        }
        None
    }

    pub fn master_layer_for(&self, glyphname: &str, master: &Master) -> Option<&Layer> {
        if let Some(glyph) = self.glyphs.get(glyphname) {
            for layer in &glyph.layers {
                if layer.id == Some(master.id.clone()) {
                    return Some(layer);
                }
            }
        }
        None
    }

    pub fn ot_value(&self, table: &str, field: &str) -> Option<OTScalar> {
        for i in &self.custom_ot_values {
            if i.table == table && i.field == field {
                return Some(i.value.clone());
            }
        }
        None
    }

    pub fn default_metric(&self, name: &str) -> Option<i32> {
        self.default_master()
            .and_then(|m| m.metrics.get(name))
            .copied()
    }

    /// Normalizes a location between -1.0 and 1.0
    pub fn normalize_location(&self, loc: &Location) -> Result<NormalizedLocation, BabelfontError> {
        let mut v: Vec<f32> = vec![];
        for axis in self.axes.iter() {
            let default = axis.default.ok_or_else(|| BabelfontError::IllDefinedAxis {
                axis_name: axis.name.default(),
            })?;
            let val =
                axis.normalize_designspace_value(*loc.0.get(&axis.tag).unwrap_or(&default))?;
            v.push(val);
        }
        Ok(NormalizedLocation(v))
    }

    /// Constructs a fonttools variation model for this designspace
    pub fn variation_model(&self) -> Result<VariationModel, BabelfontError> {
        let mut locations: Vec<OTVarLocation> = vec![];
        for master in self.masters.iter() {
            let source_loc = self.normalize_location(&master.location)?;
            let mut loc = OTVarLocation::new();
            for (ax, iter_l) in self.axes.iter().zip(source_loc.0.iter()) {
                loc.insert(ax.tag_as_tag(), *iter_l);
            }
            locations.push(loc);
        }
        Ok(VariationModel::new(locations, self.axis_order()))
    }

    fn axis_order(&self) -> Vec<Tag> {
        self.axes.iter().map(|ax| ax.tag_as_tag()).collect()
    }
}
