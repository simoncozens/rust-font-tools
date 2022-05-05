use crate::axis::Axis;
use crate::common::OTScalar;
use crate::common::OTValue;
use crate::glyph::GlyphList;
use crate::instance::Instance;
use crate::master::Master;
use crate::names::Names;
use crate::utils::otcmp;
use crate::Location;
use crate::{BabelfontError, Layer};
use chrono::Local;
use fonttools::font::Font as FTFont;
use fonttools::otvar::Location as OTVarLocation;
use fonttools::otvar::{NormalizedLocation, VariationModel};
use fonttools::{
    tables::{
        avar::{avar, SegmentMap},
        fvar::{fvar, InstanceRecord, VariationAxisRecord},
        name::NameRecord,
    },
    types::Tag,
};
use std::collections::BTreeMap;
use std::collections::HashMap;

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
    pub variation_sequences: BTreeMap<(u32, u32), String>,
    // features: ????
    // The below is temporary
    pub features: Option<String>,
    pub kern_groups: HashMap<String, Vec<String>>,
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
            variation_sequences: BTreeMap::new(),
            kern_groups: HashMap::new(),
            features: None,
        }
    }

    pub fn default_location(&self) -> Location {
        Location(
            self.axes
                .iter()
                .map(|axis| {
                    (
                        axis.tag.clone(),
                        axis.userspace_to_designspace(axis.default.unwrap_or(0.0)),
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

    /// Add information to a fonttools Font object (fvar and avar tables)
    /// expressed by this design space.
    pub fn add_variation_tables(&self, font: &mut FTFont) -> Result<(), BabelfontError> {
        let mut axes: Vec<VariationAxisRecord> = vec![];
        let mut maps: Vec<SegmentMap> = vec![];

        let mut ix = 256;
        let mut name = font
            .tables
            .name()
            .expect("No name table?")
            .expect("Couldn't open name table");

        for axis in self.axes.iter() {
            axes.push(axis.to_variation_axis_record(ix as u16)?);
            name.records.push(NameRecord::windows_unicode(
                ix as u16,
                axis.name.default().clone().expect("Bad axis name"),
            ));
            ix += 1;
            if axis.map.is_some() {
                let mut sm: Vec<(f32, f32)> = vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)];
                sm.extend(
                    axis.map
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|x| {
                            (
                                axis.normalize_userspace_value(x.0).expect("Bad map"),
                                axis.normalize_designspace_value(x.1).expect("Bad map"),
                            )
                        })
                        .collect::<Vec<(f32, f32)>>(),
                );
                sm.sort_by(|a, b| otcmp(a.0, b.0));
                sm.dedup();
                maps.push(SegmentMap::new(sm));
            } else {
                maps.push(SegmentMap::new(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)]));
            }
        }
        let mut instances: Vec<InstanceRecord> = vec![];
        for instance in &self.instances {
            name.records.push(NameRecord::windows_unicode(
                ix,
                instance.style_name.default().expect("Bad instance name"),
            ));
            let ir = InstanceRecord {
                subfamilyNameID: ix,
                coordinates: self.location_to_tuple(&instance.location),
                postscriptNameID: None,
                flags: 0,
            };
            ix += 1;
            // if let Some(psname) = &instance.postscriptfontname {
            //     if let Table::Name(name) = font
            //         .get_table(b"name")
            //         .expect("No name table?")
            //         .expect("Couldn't open name table")
            //     {
            //         name.records
            //             .push(NameRecord::windows_unicode(ix, psname.clone()));
            //     }
            //     ir.postscriptNameID = Some(ix);
            //     ix += 1;
            // }
            instances.push(ir)
        }
        font.tables.insert(fvar { axes, instances });

        font.tables.insert(avar { maps });
        font.tables.insert(name);

        Ok(())
    }

    fn tag_to_name(&self) -> HashMap<Tag, String> {
        let mut hm = HashMap::new();
        for axis in &self.axes {
            hm.insert(axis.tag_as_tag(), axis.name.default().unwrap());
        }
        hm
    }

    pub fn location_to_tuple(&self, loc: &Location) -> Vec<f32> {
        let mut tuple = vec![];
        let tag_to_name = self.tag_to_name();
        for (tag, default) in self
            .axis_order()
            .iter()
            .zip(self.default_location().0.iter())
        {
            let name = tag_to_name.get(tag).unwrap();
            let dim = loc.0.iter().find(|d| d.0 == name);
            if let Some(dim) = dim {
                tuple.push(*dim.1);
            } else {
                tuple.push(*default.1);
            }
        }
        tuple
    }
}
