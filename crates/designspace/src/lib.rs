extern crate serde;
extern crate serde_xml_rs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename = "designspace")]
pub struct Designspace {
    pub format: i32,
    pub axes: Axes,
    pub sources: Sources,
    pub instances: Option<Instances>,
    // pub rules: Rules,
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
    pub name: String,
    pub filename: String,
    pub layer: Option<String>,
    pub location: Vec<Location>,
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
    pub name: String,
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
        <dimension name="width" xvalue="0.000000" />
        <dimension name="weight" xvalue="0.000000" />
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
        assert!(false);
    }
}
