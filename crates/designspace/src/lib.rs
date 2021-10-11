//! A library for parsing variable font designspace files

#![warn(missing_docs, missing_crate_level_docs)]

use std::collections::HashMap;
use std::fs::File;
use std::path::Path;

use fonttools::otvar::{Location as OTVarLocation, NormalizedLocation, VariationModel};
use fonttools::tag;
use fonttools::types::Tag;
use serde::{Deserialize, Serialize};
pub use serde_xml_rs::from_reader;

use fonttools::font::{Font, Table};
use fonttools::tables;
use fonttools::tables::avar::{avar, SegmentMap};
use fonttools::tables::fvar::{fvar, InstanceRecord, VariationAxisRecord};
use fonttools::tables::name::NameRecord;

/// Loads and parses a designspace file
pub fn from_file(filename: &str) -> Result<Designspace, serde_xml_rs::Error> {
    from_reader(File::open(filename).unwrap())
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "designspace")]
/// A designspace object
pub struct Designspace {
    /// The format of this designspace file (we support 2 and 3)
    pub format: f32,
    /// An axes element (contains individual axes)
    pub axes: Axes,
    /// An sources element (contains individual sources)
    pub sources: Sources,
    /// An instance element (optional, contains individual instances)
    pub instances: Option<Instances>,
    // pub rules: Rules,
}

fn piecewise_linear_map(mapping: HashMap<i32, f32>, value: i32) -> f32 {
    if mapping.contains_key(&value) {
        return *mapping.get(&value).unwrap();
    }
    if mapping.keys().len() == 0 {
        return value as f32;
    }
    let min = *mapping.keys().min().unwrap();
    if value < min {
        return value as f32 + *mapping.get(&min).unwrap() - (min as f32);
    }
    let max = *mapping.keys().max().unwrap();
    if value > max {
        return value as f32 + mapping.get(&max).unwrap() - (max as f32);
    }
    let a = mapping.keys().filter(|k| *k < &value).max().unwrap();
    let b = mapping.keys().filter(|k| *k > &value).min().unwrap();
    let va = mapping.get(a).unwrap();
    let vb = mapping.get(b).unwrap();
    va + (vb - va) * (value - a) as f32 / (*b - *a) as f32
}

impl Designspace {
    /// Add information to a fonttools Font object (fvar and avar tables)
    /// expressed by this design space.
    pub fn add_to_font(&self, font: &mut Font) -> Result<(), &'static str> {
        let mut axes: Vec<VariationAxisRecord> = vec![];
        let mut maps: Vec<SegmentMap> = vec![];

        let mut ix = 255;

        for axis in self.axes.axis.iter() {
            axes.push(axis.to_variation_axis_record(ix as u16)?);
            if let Table::Name(name) = font
                .get_table(tag!("name"))
                .expect("No name table?")
                .expect("Couldn't open name table")
            {
                name.records
                    .push(NameRecord::windows_unicode(ix as u16, axis.name.clone()));
            }
            ix += 1;
            if axis.map.is_some() {
                let mut sm: Vec<(f32, f32)> = vec![(-1.0, -1.0)];
                sm.extend(
                    axis.map
                        .as_ref()
                        .unwrap()
                        .iter()
                        .map(|x| {
                            (
                                axis.normalize_userspace_value(x.input),
                                axis.normalize_designspace_value(x.output),
                            )
                        })
                        .collect::<Vec<(f32, f32)>>(),
                );
                maps.push(SegmentMap::new(sm));
            } else {
                maps.push(SegmentMap::new(vec![(-1.0, -1.0), (0.0, 0.0), (1.0, 1.0)]));
            }
        }
        let mut instances: Vec<InstanceRecord> = vec![];
        if let Some(i) = &self.instances {
            for instance in &i.instance {
                if let Table::Name(name) = font
                    .get_table(tag!("name"))
                    .expect("No name table?")
                    .expect("Couldn't open name table")
                {
                    name.records
                        .push(NameRecord::windows_unicode(ix, instance.stylename.clone()));
                }
                let mut ir = InstanceRecord {
                    subfamilyNameID: ix,
                    coordinates: self.location_to_tuple(&instance.location),
                    postscriptNameID: None,
                };
                ix += 1;
                if let Some(psname) = &instance.postscriptfontname {
                    if let Table::Name(name) = font
                        .get_table(tag!("name"))
                        .expect("No name table?")
                        .expect("Couldn't open name table")
                    {
                        name.records
                            .push(NameRecord::windows_unicode(ix, psname.clone()));
                    }
                    ir.postscriptNameID = Some(ix);
                    ix += 1;
                }
                instances.push(ir)
            }
        }
        let fvar_table = Table::Fvar(fvar { axes, instances });
        font.tables.insert(tag!("fvar"), fvar_table);

        // Handle avar here
        let avar_table = avar {
            majorVersion: 1,
            minorVersion: 0,
            reserved: 0,
            axisSegmentMaps: maps,
        };
        font.tables.insert(tag!("avar"), Table::Avar(avar_table));

        Ok(())
    }

    /// Returns a mapping between axis tags and their names
    pub fn tag_to_name(&self) -> HashMap<Tag, String> {
        let mut hm = HashMap::new();
        for axis in &self.axes.axis {
            hm.insert(axis.tag_as_tag(), axis.name.clone());
        }
        hm
    }

    /// Returns the axis order. Requires the tags to be validated; will panic
    /// if they are not four-byte tags.
    pub fn axis_order(&self) -> Vec<Tag> {
        self.axes.axis.iter().map(|ax| ax.tag_as_tag()).collect()
    }

    /// Returns the default master location in userspace coordinates
    pub fn default_location(&self) -> Vec<i32> {
        self.axes.axis.iter().map(|ax| ax.default).collect()
    }

    /// Returns the default master location in designspace coordinates
    pub fn default_designspace_location(&self) -> Vec<i32> {
        self.axes
            .axis
            .iter()
            .map(|ax| ax.userspace_to_designspace(ax.default) as i32)
            .collect()
    }

    /// Returns the location of a given source object in design space coordinates
    pub fn source_location(&self, source: &Source) -> Vec<i32> {
        self.location_to_tuple(&source.location)
            .iter()
            .map(|x| *x as i32)
            .collect()
    }

    /// Converts a location to a tuple
    pub fn location_to_tuple(&self, loc: &Location) -> Vec<f32> {
        let mut tuple = vec![];
        let tag_to_name = self.tag_to_name();
        for (tag, default) in self.axis_order().iter().zip(self.default_location().iter()) {
            let name = tag_to_name.get(tag).unwrap();
            let dim = loc.dimension.iter().find(|d| d.name == *name);
            if let Some(dim) = dim {
                tuple.push(dim.xvalue);
            } else {
                tuple.push(*default as f32);
            }
        }
        tuple
    }

    /// Returns the Source object for the master at default axis coordinates,
    /// if one can be found
    pub fn default_master(&self) -> Option<&Source> {
        let expected = self.default_designspace_location();
        self.sources
            .source
            .iter()
            .find(|s| self.source_location(s) == expected)
    }

    /// Normalizes a location between -1.0 and 1.0
    pub fn normalize_location(&self, loc: Vec<i32>) -> NormalizedLocation {
        let mut v: Vec<f32> = vec![];
        for (ax, iter_l) in self.axes.axis.iter().zip(loc.iter()) {
            let l = *iter_l;
            v.push(ax.normalize_designspace_value(l as f32));
        }
        NormalizedLocation(v)
    }

    /// Constructs a fonttools variation model for this designspace
    pub fn variation_model(&self) -> VariationModel {
        let mut locations: Vec<OTVarLocation> = vec![];
        for source in self.sources.source.iter() {
            let source_loc = self.normalize_location(self.source_location(source));
            let mut loc = OTVarLocation::new();
            for (ax, iter_l) in self.axes.axis.iter().zip(source_loc.0.iter()) {
                loc.insert(ax.tag_as_tag(), *iter_l);
            }
            locations.push(loc);
        }
        VariationModel::new(locations, self.axis_order())
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "axes")]
/// A collection of axes
pub struct Axes {
    /// A vector of axes
    pub axis: Vec<Axis>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "axes")]
/// A single axis
pub struct Axis {
    /// Axis name (user-facing)
    pub name: String,
    /// Axis tag (internal; four bytes)
    pub tag: String,
    /// Axis minimum value
    pub minimum: i32,
    /// Axis maximum value
    pub maximum: i32,
    /// Axis default value
    pub default: i32,
    /// Whether the axis should be exposed to the user
    pub hidden: Option<bool>,
    /// Internationalized name
    pub labelname: Option<Vec<LabelName>>,
    /// Mapping between userspace and designspace values
    pub map: Option<Vec<Mapping>>,
}

impl Axis {
    fn to_variation_axis_record(&self, name_id: u16) -> Result<VariationAxisRecord, &'static str> {
        if self.tag.len() != 4 {
            return Err("Badly formatted axis tag");
        }
        Ok(VariationAxisRecord {
            axisTag: Tag::from_raw(&self.tag).unwrap(),
            defaultValue: self.default as f32,
            maxValue: self.maximum as f32,
            minValue: self.minimum as f32,
            flags: if self.hidden.unwrap_or(false) {
                0x0001
            } else {
                0x0000
            },
            axisNameID: name_id,
        })
    }

    /// Converts a position on this axis in userspace coordinates to designspace coordinates
    pub fn userspace_to_designspace(&self, l: i32) -> f32 {
        let mut mapping: HashMap<i32, f32> = HashMap::new();
        if self.map.is_some() {
            for m in self.map.as_ref().unwrap().iter() {
                mapping.insert(m.input as i32, m.output);
            }
        } else {
            mapping.insert(self.minimum, self.minimum as f32);
            mapping.insert(self.default, self.default as f32);
            mapping.insert(self.maximum, self.maximum as f32);
        }

        piecewise_linear_map(mapping, l)
    }

    /// Converts a position on this axis from designspace coordinates to userspace coordinates
    pub fn designspace_to_userspace(&self, l: i32) -> f32 {
        let mut mapping: HashMap<i32, f32> = HashMap::new();
        if self.map.is_some() {
            for m in self.map.as_ref().unwrap().iter() {
                mapping.insert(m.output as i32, m.input);
            }
        } else {
            mapping.insert(self.minimum, self.minimum as f32);
            mapping.insert(self.default, self.default as f32);
            mapping.insert(self.maximum, self.maximum as f32);
        }

        piecewise_linear_map(mapping, l)
    }
    fn normalize_userspace_value(&self, mut l: f32) -> f32 {
        if l < self.minimum as f32 {
            l = self.minimum as f32;
        }
        if l > self.maximum as f32 {
            l = self.maximum as f32;
        }
        if l < self.default as f32 {
            -(self.default as f32 - l) / (self.default - self.minimum) as f32
        } else if l > self.default as f32 {
            (l - self.default as f32) / (self.maximum - self.default) as f32
        } else {
            0_f32
        }
    }

    fn tag_as_tag(&self) -> Tag {
        Tag::from_raw(&self.tag).unwrap()
    }

    fn normalize_designspace_value(&self, mut l: f32) -> f32 {
        if self.map.is_none() || self.map.as_ref().unwrap().is_empty() {
            return self.normalize_userspace_value(l);
        }
        let designspace_minimum = self
            .map
            .as_ref()
            .unwrap()
            .iter()
            .map(|m| m.output)
            .fold(1. / 0., f32::min);
        let designspace_maximum = self
            .map
            .as_ref()
            .unwrap()
            .iter()
            .map(|m| m.output)
            .fold(-1. / 0., f32::max);
        if l < designspace_minimum {
            l = designspace_minimum;
        }
        if l > designspace_maximum {
            l = designspace_maximum;
        }
        (l - designspace_minimum) / (designspace_maximum - designspace_minimum)
    }
}

#[derive(Debug, Deserialize, Serialize)]
/// A name record for internationalization of an axis
pub struct LabelName {
    /// A language string
    pub lang: String,
    /// The axis's name in that language
    #[serde(rename = "$value")]
    pub value: String,
}

/// A mapping between userspace coordinates and designspace coordinates
#[derive(Debug, Deserialize, Serialize)]
pub struct Mapping {
    /// The value in userspace coordinates
    pub input: f32,
    /// Its equivalent in userspace coordinates
    pub output: f32,
}

/// A collection of source descriptors
#[derive(Debug, Deserialize, Serialize)]
pub struct Sources {
    /// A vector of source descriptors
    pub source: Vec<Source>,
}

#[derive(Debug, Deserialize, Serialize)]
/// An individual source descriptor
pub struct Source {
    /// The family name for this source
    pub familyname: Option<String>,
    /// The stylename for this source
    pub stylename: Option<String>,
    /// The complete name for this source
    pub name: Option<String>,
    /// The filename for this source
    pub filename: String,
    /// The name of the layer in the source to look for outline data
    pub layer: Option<String>,
    /// The location of this source within the coordinates
    pub location: Location,
}

impl Source {
    #[cfg(feature = "norad")]
    /// Load the source from a UFO file
    pub fn ufo(&self, designspace_filename: &Path) -> Result<norad::Font, norad::Error> {
        log::info!("Loading {:}", self.filename);
        norad::Font::load(designspace_filename.parent().unwrap().join(&self.filename))
    }
}

/// A location element
#[derive(Debug, Deserialize, Serialize)]
pub struct Location {
    /// A vector of location components (dimensions)
    pub dimension: Vec<Dimension>,
}

/// An individual location component within a location tag
#[derive(Debug, Deserialize, Serialize)]
pub struct Dimension {
    /// The name of the axis (not the axis tag!)
    pub name: String,
    /// The value on the axis
    pub xvalue: f32,
    /// Separate value for anisotropic interpolations
    pub yvalue: Option<f32>,
}

/// A collection of instances
#[derive(Debug, Deserialize, Serialize)]
pub struct Instances {
    /// A vector of instances
    pub instance: Vec<Instance>,
}

/// An individual instance descriptor
#[derive(Debug, Deserialize, Serialize)]
pub struct Instance {
    /// The family name of this instance
    pub familyname: String,
    /// The style name of this instance
    pub stylename: String,
    /// The full name of this instance
    pub name: Option<String>,
    /// The filename for this instance
    pub filename: Option<String>,
    /// The PostScript family name for this instance
    pub postscriptfontname: Option<String>,
    /// The style map family name for this instance
    pub stylemapfamilyname: Option<String>,
    /// The style map style name for this instance
    pub stylemapstylename: Option<String>,
    /// The location of this instance in the designspace
    pub location: Location,
}

#[cfg(test)]
mod tests {
    use crate::Designspace;
    use serde_xml_rs::from_reader;
    #[test]
    fn test_de() {
        let s = r##"
        <designspace format="2">
        <axes>
    <axis default="1" maximum="1000" minimum="0" name="weight" tag="wght">
        <labelname xml:lang="fa-IR">قطر</labelname>
        <labelname xml:lang="en">Wéíght</labelname>
    </axis>
    <axis default="100" maximum="200" minimum="50" name="width" tag="wdth">
        <map input="50.0" output="10.0" />
        <map input="100.0" output="66.0" />
        <map input="200.0" output="990.0" />
    </axis>
</axes>
<sources>
    <source familyname="MasterFamilyName" filename="masters/masterTest1.ufo" name="master.ufo1" stylename="MasterStyleNameOne">
    <lib copy="1" />
    <features copy="1" />
    <info copy="1" />
    <glyph mute="1" name="A" />
    <glyph mute="1" name="Z" />
    <location>
        <dimension name="width" xvalue="150" />
    </location>
    </source>
    <source familyname="MasterFamilyName" filename="masters/default.ufo" name="default.ufo" stylename="MasterStyleNameOne">
    <location>
        <dimension name="weight" xvalue="1" />
        <dimension name="width" xvalue="66" />
    </location>
    </source>
</sources>
<instances>
<instance familyname="InstanceFamilyName" filename="instances/instanceTest2.ufo" name="instance.ufo2" postscriptfontname="InstancePostscriptName" stylemapfamilyname="InstanceStyleMapFamilyName" stylemapstylename="InstanceStyleMapStyleName" stylename="InstanceStyleName">
<location>
    <dimension name="width" xvalue="400" yvalue="300" />
    <dimension name="weight" xvalue="66" />
</location>
<kerning />
<info />
<lib>
    <dict>
        <key>com.coolDesignspaceApp.specimenText</key>
        <string>Hamburgerwhatever</string>
    </dict>
</lib>
</instance>
</instances>
</designspace>
    "##;
        let designspace: Designspace = from_reader(s.as_bytes()).unwrap();
        println!("{:#?}", designspace);
        assert_eq!(designspace.default_location(), vec![1, 100]);
        assert_eq!(
            designspace.source_location(&designspace.sources.source[0]),
            vec![1, 150]
        );
        let dm = designspace.default_master();
        assert!(dm.is_some());
        assert_eq!(dm.unwrap().filename, "masters/default.ufo");
    }
}
