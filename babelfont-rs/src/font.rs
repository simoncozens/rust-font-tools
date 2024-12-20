use crate::axis::Axis;
use crate::common::{OTScalar, OTValue};
use crate::glyph::GlyphList;
use crate::instance::Instance;
use crate::master::Master;
use crate::names::Names;
use crate::{BabelfontError, Layer};
use chrono::Local;
use fontdrasil::coords::{
    DesignCoord, DesignLocation, Location, NormalizedLocation, NormalizedSpace, UserCoord,
};
use std::collections::{BTreeMap, HashMap};
use write_fonts::types::Tag;

#[derive(Debug, Clone)]
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
    pub variation_sequences: BTreeMap<(u32, u32), String>,
    // features: ????
    // The below is temporary
    pub features: Option<String>,
    pub first_kern_groups: HashMap<String, Vec<String>>,
    pub second_kern_groups: HashMap<String, Vec<String>>,
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
            names: Names::default(),
            custom_ot_values: vec![],
            variation_sequences: BTreeMap::new(),
            first_kern_groups: HashMap::new(),
            second_kern_groups: HashMap::new(),
            features: None,
        }
    }

    pub fn default_location(&self) -> Result<DesignLocation, BabelfontError> {
        let iter: Result<Vec<(Tag, DesignCoord)>, _> = self
            .axes
            .iter()
            .map(|axis| {
                axis.userspace_to_designspace(axis.default.unwrap_or(UserCoord::new(0.0)))
                    .map(|coord| (axis.tag, coord))
            })
            .collect();
        Ok(DesignLocation::from_iter(iter?))
    }
    pub fn default_master(&self) -> Option<&Master> {
        let default_location: DesignLocation = self.default_location().ok()?;
        if self.masters.len() == 1 {
            return Some(&self.masters[0]);
        }
        self.masters
            .iter()
            .find(|&m| m.location == default_location)
    }

    pub fn default_master_index(&self) -> Option<usize> {
        let default_location: DesignLocation = self.default_location().ok()?;
        self.masters
            .iter()
            .enumerate()
            .find_map(|(ix, m)| (m.location == default_location).then(|| ix))
    }

    pub fn master(&self, master_name: &str) -> Option<&Master> {
        self.masters
            .iter()
            .find(|m| m.name.get_default().map(|x| x.as_str()) == Some(master_name))
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

    pub fn ot_value(
        &self,
        table: &str,
        field: &str,
        search_default_master: bool,
    ) -> Option<OTScalar> {
        for i in &self.custom_ot_values {
            if i.table == table && i.field == field {
                return Some(i.value.clone());
            }
        }
        if !search_default_master {
            return None;
        }
        if let Some(dm) = self.default_master() {
            return dm.ot_value(table, field);
        }
        None
    }

    pub fn set_ot_value(&mut self, table: &str, field: &str, value: OTScalar) {
        self.custom_ot_values.push(OTValue {
            table: table.to_string(),
            field: field.to_string(),
            value,
        })
    }

    pub fn default_metric(&self, name: &str) -> Option<i32> {
        self.default_master()
            .and_then(|m| m.metrics.get(name))
            .copied()
    }

    /// Normalizes a location between -1.0 and 1.0
    pub fn normalize_location<Space>(
        &self,
        loc: Location<Space>,
    ) -> Result<NormalizedLocation, Box<BabelfontError>>
    where
        Space: fontdrasil::coords::ConvertSpace<NormalizedSpace>,
    {
        let axes: Result<HashMap<Tag, fontdrasil::types::Axis>, _> = self
            .axes
            .iter()
            .map(|ax| {
                let fds_ax: Result<fontdrasil::types::Axis, _> = ax.clone().try_into();
                fds_ax.map(|fds_ax| (ax.tag, fds_ax))
            })
            .collect();
        let axes = axes?;
        Ok(loc.convert(&axes.iter().map(|(k, v)| (k.clone(), v)).collect()))
    }

    fn axis_order(&self) -> Vec<Tag> {
        self.axes.iter().map(|ax| ax.tag.clone()).collect()
    }
}
