use crate::common::OTValue;
use crate::glyph::GlyphCategory;
use crate::i18ndictionary::I18NDictionary;
use crate::OTScalar::Signed;
use crate::Shape::{ComponentShape, PathShape};
use crate::{
    Anchor, Axis, BabelfontError, Component, Font, Glyph, Guide, Instance, Layer, Location, Master,
    Node, NodeType, OTScalar, Path, Position, Shape,
};
use chrono::TimeZone;
use fonttools::types::Tag;
use lazy_static::lazy_static;
use openstep_plist::Plist;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

pub fn load(path: PathBuf) -> Result<Font, BabelfontError> {
    log::debug!("Reading to string");
    let s = fs::read_to_string(&path).map_err(|source| BabelfontError::IO {
        path: path.clone(),
        source,
    })?;
    log::debug!("Parsing PLIST");
    let plist = Plist::parse(&s).map_err(|orig| BabelfontError::PlistParse {
        path: path.clone(),
        orig,
    })?;
    log::debug!("Assembling babelfont");
    if plist.get(".formatVersion").is_none() {
        return Err(BabelfontError::WrongConvertor { path });
    }

    let mut font = Font::new();

    let custom_parameters = get_custom_parameters(&plist);
    load_axes(&mut font, &plist);
    font.kern_groups = load_kern_groups(&plist);
    load_masters(&mut font, &plist)?;
    let default_master_id = custom_parameters
        .get(&"Variable Font Origin".to_string())
        .and_then(|x| x.as_str())
        .map(|x| x.to_string())
        .or_else(|| font.masters.first().map(|m| m.id.clone()));

    fixup_axes(&mut font, default_master_id.as_ref());
    load_glyphs(&mut font, &plist);

    if let Some(instances) = plist.get("instances").and_then(|f| f.as_array()) {
        for instance in instances {
            load_instance(&mut font, instance);
        }
    }

    fixup_axis_mappings(&mut font);
    load_metadata(&mut font, &plist);

    load_custom_parameters(&mut font.custom_ot_values, custom_parameters);
    std::mem::forget(plist);
    // load_features(&mut font, &plist);
    Ok(font)
}

fn get_custom_parameters(plist: &Plist) -> HashMap<String, &Plist> {
    let mut cp: HashMap<String, &Plist> = HashMap::new();
    if let Some(param) = plist.get("customParameters") {
        for p in param.as_array().unwrap() {
            let key = p.get("name").and_then(|n| n.as_str());
            let value = p.get("value");
            if let Some(key) = key {
                if let Some(value) = value {
                    cp.insert(key.to_string(), value);
                }
            }
        }
    }

    cp
}

fn load_kern_groups(plist: &Plist) -> HashMap<String, Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    if let Some(glyphs) = plist.get("glyphs").and_then(|a| a.as_array()) {
        for g in glyphs {
            if let Some(glyphname) = g.get("glyphname").and_then(|s| s.as_str()) {
                let l_class = g
                    .get("leftKerningGroup")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| glyphname.to_string());
                let r_class = g
                    .get("rightKerningGroup")
                    .and_then(|s| s.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| glyphname.to_string());
                groups
                    .entry("MMK_L_".to_owned() + &l_class)
                    .or_insert_with(Vec::new)
                    .push(glyphname.to_string());
                groups
                    .entry("MMK_R_".to_owned() + &r_class)
                    .or_insert_with(Vec::new)
                    .push(glyphname.to_string());
            }
        }
    }
    groups
}

fn load_axes(font: &mut Font, plist: &Plist) {
    if let Some(axes) = plist.get("axes") {
        for axis in axes.as_array().unwrap() {
            let name = axis.get("name").and_then(|n| n.as_str());
            let tag = axis.get("tag").and_then(|n| n.as_str());
            if let Some(name) = name {
                if let Some(tag) = tag {
                    let mut new_axis = Axis::new(name, tag.to_string());
                    new_axis.hidden = axis.get("hidden").is_some();
                    font.axes.push(new_axis)
                }
            }
        }
    }
}

fn _to_loc(font: &Font, values: Option<&Plist>) -> Location {
    let axis_tags = font.axes.iter().map(|x| x.tag.clone());
    let mut loc = Location::new();
    if let Some(values) = values.and_then(|v| v.as_array()) {
        for (v, tag) in values.iter().zip(axis_tags) {
            loc.0.insert(tag, v.as_f64().unwrap() as f32);
        }
    }
    loc
}

fn convert_metric_name(n: &str) -> String {
    (match n {
        "x-height" => "xHeight",
        "cap height" => "capHeight",
        _ => n,
    })
    .to_string()
}
fn load_masters(font: &mut Font, plist: &Plist) -> Result<(), BabelfontError> {
    let metrics = plist.get("metrics");
    if let Some(masters) = plist.get("fontMaster") {
        for master in masters.as_array().unwrap() {
            let location = _to_loc(font, master.get("axesValues"));
            let name =
                master
                    .get("name")
                    .and_then(|n| n.as_str())
                    .ok_or(BabelfontError::General {
                        msg: "Master has no name!".to_string(),
                    })?;
            let id = master
                .get("id")
                .and_then(|n| n.as_str())
                .ok_or(BabelfontError::General {
                    msg: "Master has no id!".to_string(),
                })?;
            let mut new_master = Master::new(name, id, location);

            if let Some(guides) = master.get("guides").and_then(|a| a.as_array()) {
                new_master.guides = guides.iter().map(load_guide).collect();
            }

            load_metrics(&mut new_master, master, metrics);
            if let Some(kerning) = plist.get("kerningLTR").and_then(|d| d.get(id)) {
                load_kerning(&mut new_master, kerning);
            }
            let custom_parameters = get_custom_parameters(master);
            load_custom_parameters(&mut new_master.custom_ot_values, custom_parameters);
            font.masters.push(new_master)
        }
    }
    Ok(())
}

fn load_metrics(new_master: &mut Master, master: &Plist, metrics: Option<&Plist>) {
    if let Some(metric_values) = master.get("metricValues").and_then(|l| l.as_array()) {
        if let Some(metrics) = metrics {
            for (metric, metric_value) in
                metrics.as_array().unwrap().iter().zip(metric_values.iter())
            {
                if let Some(metric_name) = metric
                    .get("type")
                    .or_else(|| metric.get("name"))
                    .and_then(|x| x.as_str())
                {
                    let value: i32 = metric_value
                        .get("pos")
                        .unwrap_or(&Plist::Integer(0))
                        .as_i32()
                        .unwrap_or(0);
                    new_master
                        .metrics
                        .insert(convert_metric_name(metric_name), value);
                    // I don't care about overshoots today.
                }
            }
        }
    }
}

fn load_kerning(new_master: &mut Master, kerning: &Plist) {
    let mut out_kerning = HashMap::new();
    for (left, right_dict) in kerning.as_dict().unwrap().iter() {
        for (right, value) in right_dict.as_dict().unwrap().iter() {
            out_kerning.insert(
                (left.clone(), right.clone()),
                value.as_i32().unwrap_or(0) as i16,
            );
        }
    }
    new_master.kerning = out_kerning;
}

fn tuple_to_position(p: &[Plist]) -> Position {
    let mut x: f32 = 0.0;
    let mut y: f32 = 0.0;
    let mut angle: f32 = 0.0;
    let mut iter = p.iter();
    if let Some(x_plist) = iter.next() {
        x = x_plist.as_f32().unwrap();
    }
    if let Some(y_plist) = iter.next() {
        y = y_plist.as_f32().unwrap();
    }
    if let Some(angle_plist) = iter.next() {
        angle = angle_plist.as_f32().unwrap();
    }

    Position {
        x: x as i32,
        y: y as i32,
        angle,
    }
}

fn load_guide(g: &Plist) -> Guide {
    let mut guide = Guide::new();
    let default = vec![Plist::Integer(0), Plist::Integer(0)];
    if let Some(g) = g.as_dict() {
        let pos = g.get("pos").and_then(|x| x.as_array()).unwrap_or(&default);
        let angle: f32 = g
            .get("angle")
            .unwrap_or(&Plist::Float(0.0))
            .as_f32()
            .unwrap_or(0.0);
        guide.pos = tuple_to_position(pos);
        guide.pos.angle = angle;
    }
    guide
}

fn fixup_axes(f: &mut Font, default_master_id: Option<&String>) {
    for master in &f.masters {
        for mut axis in f.axes.iter_mut() {
            let this_loc = *(master.location.0.get(&axis.tag).unwrap_or(&0.0));
            if axis.min.is_none() || this_loc < axis.min.unwrap() {
                axis.min = Some(this_loc);
            }
            if axis.max.is_none() || this_loc > axis.max.unwrap() {
                axis.max = Some(this_loc);
            }
            if default_master_id == Some(&master.id) {
                axis.default = Some(this_loc);
            }
        }
    }
}

fn load_glyphs(font: &mut Font, plist: &Plist) {
    if let Some(glyphs) = plist.get("glyphs").and_then(|a| a.as_array()) {
        for g in glyphs {
            if let Ok(glyph) = load_glyph(g) {
                font.glyphs.push(glyph);
            }
        }
    }
}

fn load_glyph(g: &Plist) -> Result<Glyph, BabelfontError> {
    let name = g
        .get("glyphname")
        .and_then(|f| f.as_str())
        .ok_or(BabelfontError::General {
            msg: "Couldn't read a glyph name!".to_string(),
        })?;
    let category = g.get("category").and_then(|f| f.as_str());
    let subcategory = g.get("subcategory").and_then(|f| f.as_str());
    let codepoints = get_codepoints(g);
    let gc = if subcategory == Some("Ligature") {
        GlyphCategory::Ligature
    } else if category == Some("Mark") {
        GlyphCategory::Mark
    } else {
        GlyphCategory::Base
    };
    let mut layers = vec![];
    if let Some(plist_layers) = g.get("layers") {
        for layer in plist_layers.as_array().unwrap() {
            layers.push(load_layer(layer, name)?);
        }
    }
    Ok(Glyph {
        name: name.to_string(),
        category: gc,
        production_name: None,
        codepoints,
        layers,
        exported: g.get("export").is_none(),
        direction: None,
    })
}

fn load_layer(l: &Plist, glyph_name: &str) -> Result<Layer, BabelfontError> {
    let width = l.get("width").and_then(|x| x.as_i32()).unwrap_or(0);
    let mut layer = Layer::new(width);
    if let Some(name) = l.get("name").and_then(|l| l.as_str()) {
        layer.name = Some(name.to_string());
    }
    if let Some(id) = l.get("layerId").and_then(|l| l.as_str()) {
        layer.id = Some(id.to_string());
    }
    if let Some(guides) = l.get("guides").and_then(|l| l.as_array()) {
        layer.guides = guides.iter().map(load_guide).collect();
    }
    if let Some(anchors) = l.get("anchors").and_then(|l| l.as_array()) {
        for anchor in anchors {
            layer.anchors.push(load_anchor(anchor));
        }
    }
    if let Some(shapes) = l.get("shapes").and_then(|l| l.as_array()) {
        for shape in shapes {
            match load_shape(shape, glyph_name) {
                Ok(shape) => layer.shapes.push(shape),
                Err(e) => log::error!("{:}", e),
            }
        }
    }

    Ok(layer)
}

fn load_anchor(a: &Plist) -> Anchor {
    let default = vec![Plist::Integer(0), Plist::Integer(0)];
    let pos = a.get("pos").and_then(|x| x.as_array()).unwrap_or(&default);
    Anchor {
        x: pos.first().and_then(|x| x.as_i32()).unwrap_or(0),
        y: pos.last().and_then(|x| x.as_i32()).unwrap_or(0),
        name: a
            .get("name")
            .and_then(|x| x.as_str())
            .unwrap_or("Unknown")
            .to_string(),
    }
}

fn load_shape(a: &Plist, glyph_name: &str) -> Result<Shape, BabelfontError> {
    if a.get("nodes").is_some() {
        // It's a path
        let mut path = Path {
            nodes: vec![],
            closed: true,
            direction: crate::shape::PathDirection::Clockwise,
        };
        for node in a
            .get("nodes")
            .unwrap()
            .as_array()
            .ok_or(BabelfontError::General {
                msg: format!("Couldn't read nodes array in glyph {:}", glyph_name),
            })?
        {
            let (x, y, typ) = node.as_node().ok_or(BabelfontError::General {
                msg: format!(
                    "Couldn't convert {:?} to nodes in glyph {:}",
                    node, glyph_name
                ),
            })?;
            let typ = typ.chars().next().unwrap_or('l');
            let nodetype = match typ {
                'l' => NodeType::Line,
                'o' => NodeType::OffCurve,
                'c' => NodeType::Curve,
                _ => NodeType::Line,
            };
            path.nodes.push(Node {
                x: *x as f32,
                y: *y as f32,
                nodetype,
            })
        }
        Ok(PathShape(path))
    } else {
        // It's a component
        let reference = a
            .get("ref")
            .and_then(|f| f.as_str())
            .ok_or(BabelfontError::General {
                msg: format!(
                    "Couldn't understand component reference in glyph {:}",
                    glyph_name
                ),
            })?;
        let pos: Vec<f32> = a
            .get("pos")
            .and_then(|f| f.as_array())
            .unwrap_or(&[Plist::Integer(0), Plist::Integer(0)])
            .iter()
            .map(|x| x.as_f32().unwrap_or(0.0))
            .collect();

        let scale: Vec<f32> = a
            .get("scale")
            .and_then(|f| f.as_array())
            .unwrap_or(&[Plist::Integer(1), Plist::Integer(1)])
            .iter()
            .map(|x| x.as_f32().unwrap_or(0.0))
            .collect();
        let transform = kurbo::Affine::translate((
            *pos.first().unwrap_or(&0.0) as f64,
            *pos.last().unwrap_or(&0.0) as f64,
        ));
        let scalingtransform = kurbo::Affine::scale_non_uniform(
            *scale.first().unwrap_or(&1.0) as f64,
            *scale.last().unwrap_or(&1.0) as f64,
        );

        Ok(ComponentShape(Component {
            reference: reference.to_string(),
            transform: transform * scalingtransform,
        }))
    }
}

fn get_codepoints(g: &Plist) -> Vec<usize> {
    let unicode = g.get("unicode");
    if unicode.is_none() {
        return vec![];
    }
    let unicode = unicode.unwrap();
    if let Plist::Array(unicodes) = unicode {
        unicodes
            .iter()
            .map(|x| x.as_i32().unwrap_or(0) as usize)
            .collect()
    } else {
        vec![unicode.as_i32().unwrap_or(0) as usize]
    }
}

fn load_metadata(font: &mut Font, plist: &Plist) {
    font.upm = plist
        .get("unitsPerEm")
        .and_then(|x| x.as_i32())
        .unwrap_or(1000) as u16;
    font.version = (
        plist
            .get("versionMajor")
            .and_then(|x| x.as_i32())
            .unwrap_or(1) as u16,
        plist
            .get("versionMinor")
            .and_then(|x| x.as_i32())
            .unwrap_or(1) as u16,
    );
    font.names.family_name = plist
        .get("familyName")
        .and_then(|s| s.as_str())
        .unwrap_or("New font")
        .into();
    load_properties(font, plist);
    font.date = plist
        .get("date")
        .and_then(|x| x.as_str())
        .as_ref()
        .and_then(|x| chrono::NaiveDateTime::parse_from_str(x, "%Y-%m-%d %H:%M:%S +0000").ok())
        .map(|x| chrono::Local.from_local_datetime(&x).unwrap())
        .unwrap_or_else(chrono::Local::now);
    font.note = plist
        .get("note")
        .and_then(|x| x.as_str())
        .map(|x| x.to_string());
}

fn load_properties(font: &mut Font, plist: &Plist) {
    if let Some(props) = plist.get("properties").and_then(|d| d.as_array()) {
        for prop in props {
            if let Some(key) = prop.get("key").map(|f| f.to_string()) {
                let mut val = I18NDictionary::new();
                if let Some(pval) = prop.get("value").and_then(|f| f.as_str()) {
                    val.set_default(pval.to_string());
                } else if let Some(pvals) = prop.get("values").and_then(|f| f.as_array()) {
                    for entry in pvals {
                        if let Some(l) = entry.get("language").and_then(|f| f.as_str()) {
                            if let Some(v) = entry.get("value").and_then(|f| f.as_str()) {
                                if l.len() != 4 {
                                    continue;
                                };
                                let l = Tag::from_raw(l.as_bytes()).unwrap();
                                val.0.insert(l, v.to_string());
                            }
                        }
                    }
                }
                if key == "copyright" || key == "copyrights" {
                    font.names.copyright = val;
                } else if key == "designer" || key == "designers" {
                    font.names.designer = val;
                } else if key == "designerURL" {
                    font.names.designer_url = val;
                } else if key == "manufacturer" || key == "manufacturers" {
                    font.names.manufacturer = val;
                } else if key == "manufacturerURL" {
                    font.names.manufacturer_url = val;
                } else if key == "license" || key == "licenses" {
                    font.names.license = val;
                } else if key == "licenseURL" {
                    font.names.license_url = val;
                } else if key == "trademark" || key == "trademarks" {
                    font.names.trademark = val;
                } else if key == "description" || key == "descriptions" {
                    font.names.description = val;
                } else if key == "sampleText" || key == "sampleTexts" {
                    font.names.sample_text = val;
                } else if key == "postscriptFullName" { // ??
                } else if key == "WWSFamilyName" {
                    font.names.w_w_s_family_name = val;
                } else if key == "versionString" {
                    font.names.version = val;
                }
            }
        }
    }
}

lazy_static! {
    static ref UNSIGNED_CP: Vec<(&'static str, &'static str, &'static str)> =
        vec![
        ("openTypeHeadLowestRecPPEM", "head", "lowestRecPPEM"),
        ("openTypeOS2StrikeoutPosition", "OS2", "yStrikeoutPosition"),
        ("openTypeOS2WidthClass", "OS2", "usWidthClass"),
        ("openTypeOS2WeightClass", "OS2", "usWeightClass"),
        ("widthClass", "OS2", "usWidthClass"),
        ("weightClass", "OS2", "usWeightClass"),
        ("openTypeOS2WinAscent", "OS2", "usWinAscent"),
        ("openTypeOS2WinDescent", "OS2", "usWinDescent"),
        ("winAscent", "OS2", "usWinAscent"),
        ("winDescent", "OS2", "usWinDescent"),


        ];
    static ref SIGNED_CP: Vec<(&'static str, &'static str, &'static str)> = vec![
        ("hheaAscender", "hhea", "ascent"),
        ("openTypeHheaAscender", "hhea", "ascent"),
        ("hheaDescender", "hhea", "descent"),
        ("openTypeHheaDescender", "hhea", "descent"),
        ("hheaLineGap", "hhea", "lineGap"),
        ("openTypeHheaLineGap", "hhea", "lineGap"),
        ("openTypeOS2FamilyClass", "OS2", "sFamilyClass"),
        ("openTypeOS2StrikeoutPosition", "OS2", "yStrikeoutPosition"),
        ("openTypeOS2StrikeoutSize", "OS2", "yStrikeoutSize"),
        ("strikeoutPosition", "OS2", "yStrikeoutPosition"),
        ("strikeoutSize", "OS2", "yStrikeoutSize"),
        ("openTypeOS2SubscriptXOffset","OS2", "ySubscriptXOffset"),
        ("openTypeOS2SubscriptXSize","OS2", "ySubscriptXSize"),
        ("openTypeOS2SubscriptYOffset","OS2", "ySubscriptYOffset"),
        ("openTypeOS2SubscriptYSize","OS2", "ySubscriptYSize"),
        ("openTypeOS2SuperscriptXOffset","OS2", "ySuperscriptXOffset"),
        ("openTypeOS2SuperscriptXSize","OS2", "ySuperscriptXSize"),
        ("openTypeOS2SuperscriptYOffset","OS2", "ySuperscriptYOffset"),
        ("openTypeOS2SuperscriptYSize","OS2", "ySuperscriptYSize"),
        ("subscriptXOffset","OS2", "ySubscriptXOffset"),
        ("subscriptXSize","OS2", "ySubscriptXSize"),
        ("subscriptYOffset","OS2", "ySubscriptYOffset"),
        ("subscriptYSize","OS2", "ySubscriptYSize"),
        ("superscriptXOffset","OS2", "ySuperscriptXOffset"),
        ("superscriptXSize","OS2", "ySuperscriptXSize"),
        ("superscriptYOffset","OS2", "ySuperscriptYOffset"),
        ("superscriptYSize","OS2", "ySuperscriptYSize"),
        ("openTypeOS2TypoAscender","OS2", "sTypoAscender"),
        ("openTypeOS2TypoDescender","OS2", "sTypoDescender"),
        ("openTypeOS2TypoLineGap","OS2", "sTypoLineGap"),
        ("typoAscender","OS2", "sTypoAscender"),
        ("typoDescender","OS2", "sTypoDescender"),
        ("typoLineGap","OS2", "sTypoLineGap"),
        ("underlinePosition", "post", "underlinePosition"),
        ("postscriptUnderlinePosition", "post", "underlinePosition"),
        ("underlineThickness", "post", "underlineThickness"),
        ("postscriptUnderlineThickness", "post", "underlineThickness"),
        ("openTypeHheaCaretSlopeRun", "hhea", "caretSlopeRun"),
        ("openTypeVheaCaretSlopeRun", "vhea", "caretSlopeRun"),
        ("openTypeVheaCaretSlopeRise", "vhea", "caretSlopeRise"),
        ("openTypeHheaCaretSlopeRise", "hhea", "caretSlopeRise"),
        ("openTypeHheaCaretOffset", "hhea", "caretOffset"),

    ];
    static ref STRING_CP: Vec<(&'static str, &'static str, &'static str)> = vec![
        ("preferredFamilyName", "name", "preferredFamilyName"),
        ("openTypeNamePreferredFamilyName", "name", "preferredFamilyName"),
        ("preferredSubfamilyName", "name", "preferredSubfamilyName"),
        ("openTypeHheaDescender", "hhea", "descent"),
        ("compatibleFullName", "name", "compatibleFullName"),
        ("openTypeNameCompatibleFullName", "name", "compatibleFullName"),
        ("vendorID", "OS2", "achVendID"),
        ("openTypeOS2VendorID", "OS2", "achVendID"),
    ];
    static ref BOOL_CP: Vec<(&'static str, &'static str, &'static str)> = vec![
        ("isFixedPitch", "post", "isFixedPitch"),
        ("postscriptIsFixedPitch", "post", "isFixedPitch"),
    ];

    // XXX fsType
}

fn load_custom_parameters(ot_values: &mut Vec<OTValue>, params: HashMap<String, &Plist>) {
    for (key, table, field) in UNSIGNED_CP.iter() {
        if let Some(v) = params.get(&key.to_string()) {
            ot_values.push(OTValue {
                table: table.to_string(),
                field: field.to_string(),
                value: OTScalar::Unsigned(v.as_i32().unwrap_or(0) as u32),
            });
        }
    }
    for (key, table, field) in SIGNED_CP.iter() {
        if let Some(v) = params.get(&key.to_string()) {
            ot_values.push(OTValue {
                table: table.to_string(),
                field: field.to_string(),
                value: Signed((*v).as_i32().unwrap_or(0)),
            });
        }
    }
    for (key, table, field) in BOOL_CP.iter() {
        if let Some(v) = params.get(&key.to_string()) {
            ot_values.push(OTValue {
                table: table.to_string(),
                field: field.to_string(),
                value: OTScalar::Bool(v.as_i64().unwrap_or(0) > 0),
            });
        }
    }
}

fn load_instance(font: &mut Font, plist: &Plist) {
    let name = plist
        .get("name")
        .map(|f| f.to_string())
        .unwrap_or_else(|| "Unnamed Instance".to_string());
    let location = if plist.get("axesValues").is_some() {
        _to_loc(font, plist.get("axesValues"))
    } else {
        log::warn!(
            "Intermediate instance not implemented yet for instance {:?}",
            name
        );
        return;
    };
    let cp = get_custom_parameters(plist);
    let mut userspace_location: HashMap<String, f32> = HashMap::new();
    if let Some(axis_locs) = cp.get("Axis Location").and_then(|f| f.as_array()) {
        for loc in axis_locs {
            if let Some(axis_name) = loc.get("Axis").map(|f| f.to_string()) {
                // XXX map to tag here?
                let loc = loc.get("Location").and_then(|x| x.as_f32()).unwrap_or(0.0);
                userspace_location.insert(axis_name, loc);
            }
        }
    }

    // Weight and width are implicit, add them
    if !userspace_location.contains_key("wght") {
        let weightclass = plist
            .get("weightClass")
            .map(|f| f.to_string())
            .unwrap_or_else(|| "Regular".to_string());
        userspace_location.insert("wght".to_string(), weightclass_to_css(&weightclass));
    }
    if !userspace_location.contains_key("wdth") {
        let weightclass = plist
            .get("widthClass")
            .map(|f| f.to_string())
            .unwrap_or_else(|| "Regular".to_string());
        userspace_location.insert("wdth".to_string(), widthclass_to_css(&weightclass));
    }
    for (axis_name, loc) in userspace_location.iter() {
        if let Some(axis) = font.axes.iter_mut().find(|ax| ax.tag == *axis_name) {
            if let Some(designspace_value) = location.0.get(&axis.tag) {
                if axis.map.is_none() {
                    axis.map = Some(vec![]);
                }
                axis.map.as_mut().unwrap().push((*loc, *designspace_value));
            }
        }
    }
    font.instances.push(Instance {
        name: (&name).into(),
        location,
        style_name: (&name).into(),
    });
}

fn fixup_axis_mappings(font: &mut Font) {
    for axis in font.axes.iter_mut() {
        if axis.map.is_none() {
            continue;
        }
        if let Some((min, default, max)) = axis.bounds() {
            axis.min = Some(axis.designspace_to_userspace(min));
            axis.max = Some(axis.designspace_to_userspace(max));
            axis.default = Some(axis.designspace_to_userspace(default));
        }
    }
}

fn weightclass_to_css(s: &str) -> f32 {
    match s {
        "Thin" => 100.0,
        "ExtraLight" => 200.0,
        "UltraLight" => 200.0,
        "Light" => 300.0,
        "Regular" => 400.0,
        "Normal" => 400.0,
        "Medium" => 500.0,
        "DemiBold" => 600.0,
        "SemiBold" => 600.0,
        "Bold" => 700.0,
        "UltraBold" => 800.0,
        "ExtraBold" => 800.0,
        "Black" => 900.0,
        "Heavy" => 900.0,
        _ => 400.0,
    }
}
fn widthclass_to_css(s: &str) -> f32 {
    match s {
        "Ultra Condensed" => 1.0,
        "Extra Condensed" => 2.0,
        "Condensed" => 3.0,
        "SemiCondensed" => 4.0,
        "Medium" => 5.0,
        "Medium (normal)" => 5.0,
        "Semi Expanded" => 6.0,
        "Expanded" => 7.0,
        "Extra Expanded" => 8.0,
        "Ultra Expanded" => 9.0,
        _ => 5.0,
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn do_something() {
        let _f = load("data/Nunito3.glyphs".into()).unwrap();
    }
}
