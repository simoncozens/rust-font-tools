extern crate serde;
extern crate serde_xml_rs;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::convert::TryInto;
use std::fs::File;
extern crate fonttools;
extern crate norad;
use fonttools::avar::{avar, SegmentMap};
use fonttools::font::{Font, Table};
use fonttools::fvar::{fvar, InstanceRecord, VariationAxisRecord};
use fonttools::otvar::NormalizedLocation;
use otspec::types::Tag;
use serde_xml_rs::from_reader;

pub fn from_file(filename: &str) -> Result<Designspace, serde_xml_rs::Error> {
    from_reader(File::open(filename).unwrap())
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "designspace")]
pub struct Designspace {
    pub format: f32,
    pub axes: Axes,
    pub sources: Sources,
    pub instances: Option<Instances>,
    // pub rules: Rules,
}

impl Designspace {
    /// Add information to a font (fvar and avar tables) expressed by this
    /// design space.
    pub fn add_to_font(&self, font: &mut Font) -> Result<(), &'static str> {
        let mut axes: Vec<VariationAxisRecord> = vec![];
        let mut maps: Vec<SegmentMap> = vec![];
        for axis in &self.axes.axis {
            axes.push(axis.to_variation_axis_record()?);
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
        println!("{:?}", maps);
        let mut instances: Vec<InstanceRecord> = vec![];
        // if let Some(i) = self.instances {
        //     for instance in i.instance {
        //         instances.push(instance.to_instance_record()?);
        //     }
        // }
        let fvar_table = Table::Fvar(fvar { axes, instances });
        font.tables.insert(*b"fvar", fvar_table);

        // Handle avar here
        let avar_table = avar {
            majorVersion: 1,
            minorVersion: 0,
            reserved: 0,
            axisSegmentMaps: maps,
        };
        font.tables.insert(*b"avar", Table::Avar(avar_table));

        Ok(())
    }

    pub fn tag_to_name(&self) -> HashMap<Tag, String> {
        let mut hm = HashMap::new();
        for axis in &self.axes.axis {
            hm.insert(
                axis.tag.as_bytes()[0..4].try_into().unwrap(),
                axis.name.clone(),
            );
        }
        hm
    }

    /// Returns the axis order. Requires the tags to be validated; will panic
    /// if they are not four-byte tags.
    pub fn axis_order(&self) -> Vec<Tag> {
        self.axes
            .axis
            .iter()
            .map(|ax| ax.tag.as_bytes()[0..4].try_into().unwrap())
            .collect()
    }

    pub fn default_location(&self) -> Vec<i32> {
        self.axes.axis.iter().map(|ax| ax.default).collect()
    }

    // Returns the location of a given source object in design space coordinates
    pub fn source_location(&self, source: &Source) -> Vec<i32> {
        let tag_to_name = self.tag_to_name();
        let mut location = vec![];
        for (tag, default) in self.axis_order().iter().zip(self.default_location().iter()) {
            let name = tag_to_name.get(tag).unwrap();
            // Find this in the source
            let dim = source.location.dimension.iter().find(|d| d.name == *name);
            if let Some(dim) = dim {
                location.push(dim.xvalue as i32);
            } else {
                location.push(*default);
            }
        }
        location
    }

    /// Returns the Source object for the master at default axis coordinates,
    /// if one can be found
    pub fn default_master(&self) -> Option<&Source> {
        let expected = self.default_location();
        self.sources
            .source
            .iter()
            .find(|s| self.source_location(s) == expected)
    }

    pub fn normalize_location(&self, loc: Vec<i32>) -> NormalizedLocation {
        let mut v: Vec<f32> = vec![];
        for (ax, iter_l) in self.axes.axis.iter().zip(loc.iter()) {
            let l = *iter_l;
            v.push(ax.normalize_designspace_value(l as f32));
        }
        NormalizedLocation(v)
    }
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "axes")]
pub struct Axes {
    pub axis: Vec<Axis>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "axes")]
pub struct Axis {
    pub name: String,
    pub tag: String,
    pub minimum: i32,
    pub maximum: i32,
    pub default: i32,
    pub hidden: Option<bool>,
    pub labelname: Option<Vec<LabelName>>,
    pub map: Option<Vec<Mapping>>,
}

impl Axis {
    fn to_variation_axis_record(&self) -> Result<VariationAxisRecord, &'static str> {
        if self.tag.len() != 4 {
            return Err("Badly formatted axis tag");
        }
        Ok(VariationAxisRecord {
            axisTag: self.tag.as_bytes()[0..4].try_into().unwrap(),
            defaultValue: self.default as f32,
            maxValue: self.maximum as f32,
            minValue: self.minimum as f32,
            flags: if self.hidden.unwrap_or(false) {
                0x0001
            } else {
                0x0000
            },
            axisNameID: 255, /* XXX */
        })
    }

    fn normalize_designspace_value(&self, mut l: f32) -> f32 {
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

    fn normalize_userspace_value(&self, mut l: f32) -> f32 {
        if self.map.is_none() || self.map.as_ref().unwrap().is_empty() {
            return self.normalize_designspace_value(l);
        }
        let userspace_minimum = self
            .map
            .as_ref()
            .unwrap()
            .iter()
            .map(|m| m.input)
            .fold(1. / 0., f32::min);
        let userspace_maximum = self
            .map
            .as_ref()
            .unwrap()
            .iter()
            .map(|m| m.input)
            .fold(-1. / 0., f32::max);
        if l < userspace_minimum {
            l = userspace_minimum;
        }
        if l > userspace_maximum {
            l = userspace_maximum;
        }
        let v = (l - userspace_minimum) / (userspace_maximum - userspace_minimum);
        v
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct LabelName {
    pub lang: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Mapping {
    pub input: f32,
    pub output: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Sources {
    pub source: Vec<Source>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Source {
    pub familyname: Option<String>,
    pub stylename: Option<String>,
    pub name: Option<String>,
    pub filename: String,
    pub layer: Option<String>,
    pub location: Location,
}

impl Source {
    pub fn ufo(&self) -> Result<norad::Font, norad::Error> {
        log::info!("Loading {:}", self.filename);
        norad::Font::load(&self.filename)
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Location {
    pub dimension: Vec<Dimension>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Dimension {
    pub name: String,
    pub xvalue: f32,
    pub yvalue: Option<f32>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Instances {
    pub instance: Vec<Instance>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Instance {
    pub familyname: String,
    pub stylename: String,
    pub name: Option<String>,
    pub filename: Option<String>,
    pub postscriptfontname: Option<String>,
    pub stylemapfamilyname: Option<String>,
    pub stylemapstylename: Option<String>,
    pub location: Vec<Location>,
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
        <dimension name="width" xvalue="100" />
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
