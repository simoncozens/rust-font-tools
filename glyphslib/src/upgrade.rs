use std::collections::{BTreeMap, BTreeSet};

use crate::glyphs2;
use crate::glyphs3::{self, Axis, LocalizedPropertyKey, Metric, MetricType, MetricValue, Property};

impl From<glyphs2::Node> for glyphs3::Node {
    fn from(val: glyphs2::Node) -> Self {
        glyphs3::Node {
            x: val.x,
            y: val.y,
            node_type: val.node_type,
            user_data: None,
        }
    }
}
impl From<glyphs2::Guide> for glyphs3::Guide {
    fn from(val: glyphs2::Guide) -> Self {
        glyphs3::Guide {
            alignment: val.alignment,
            angle: val.angle,
            locked: val.locked,
            pos: val.pos,
            scale: val.scale,
        }
    }
}
impl From<glyphs2::Anchor> for glyphs3::Anchor {
    fn from(val: glyphs2::Anchor) -> Self {
        glyphs3::Anchor {
            pos: val.position,
            name: val.name,
        }
    }
}
impl From<glyphs2::BackgroundImage> for glyphs3::BackgroundImage {
    fn from(val: glyphs2::BackgroundImage) -> Self {
        let decomposed = decompose(&val.transform);
        glyphs3::BackgroundImage {
            angle: decomposed.rotation.to_degrees(), // I think it's degrees?
            image_path: val.image_path,
            locked: val.locked,
            scale: decomposed.scale,
            pos: decomposed.translation,
        }
    }
}

struct DecomposedAffine {
    translation: (f32, f32),
    scale: (f32, f32),
    rotation: f32,
    // I don't care about skew
}

fn decompose(t: &glyphs2::Transform) -> DecomposedAffine {
    let delta = t.m11 * t.m22 - t.m12 * t.m21;
    let translation = (t.t_x, t.t_y);
    let (rotation, scale) = if t.m11 != 0.0 || t.m12 != 0.0 {
        let r = (t.m11 * t.m11 + t.m12 * t.m12).sqrt();
        let angle = if t.m12 > 0.0 {
            (t.m11 / r).acos()
        } else {
            -(t.m11 / r).acos()
        };
        (angle, (r, delta / r))
    } else if t.m21 != 0.0 || t.m22 != 0.0 {
        let s = (t.m21 * t.m21 + t.m22 * t.m22).sqrt();
        let angle = if t.m22 > 0.0 {
            (t.m22 / s).asin()
        } else {
            -(t.m22 / s).asin()
        };
        ((std::f32::consts::PI / 2.0) - angle, (delta / s, s))
    } else {
        (0.0, (0.0, 0.0))
    };
    DecomposedAffine {
        translation,
        scale,
        rotation,
    }
}

impl From<glyphs2::Layer> for glyphs3::Layer {
    fn from(val: glyphs2::Layer) -> Self {
        let attrs = BTreeMap::new();
        let shapes = val
            .components
            .into_iter()
            .map(Into::into)
            .map(glyphs3::Shape::Component)
            .chain(
                val.paths
                    .into_iter()
                    .map(Into::into)
                    .map(glyphs3::Shape::Path),
            )
            .collect();
        glyphs3::Layer {
            anchors: val.anchors.into_iter().map(Into::into).collect(),
            annotations: val.annotations,
            associated_master_id: val.associated_master_id,
            attr: attrs,
            background: val
                .background
                .map(|x| Box::new(std::convert::Into::<glyphs3::Layer>::into(*x))),
            background_image: val.background_image.map(Into::into),
            color: None,
            guides: val.guidelines.into_iter().map(Into::into).collect(),
            hints: vec![], // XXX Todo, one day
            layer_id: val.layer_id,
            metric_bottom: None,
            metric_left: val.metric_left,
            metric_right: val.metric_right,
            metric_top: None,
            metric_vert_width: None,
            metric_width: val.metric_width,
            name: val.name,
            part_selection: BTreeMap::new(), // Maybe Glyphs2 smart component data is stored in user data?
            shapes,
            user_data: val.user_data,
            vert_origin: None,
            vert_width: val.vert_width,
            visible: val.visible,
            width: val.width,
        }
    }
}

impl From<glyphs2::Component> for glyphs3::Component {
    fn from(val: glyphs2::Component) -> Self {
        let decomposed = decompose(&val.transform);
        glyphs3::Component {
            alignment: val.alignment,
            anchor: val.anchor,
            angle: decomposed.rotation.to_degrees(),
            position: decomposed.translation,
            component_glyph: val.component_glyph,
            scale: decomposed.scale,
            ..Default::default()
        }
    }
}

impl From<glyphs2::Path> for glyphs3::Path {
    fn from(val: glyphs2::Path) -> Self {
        glyphs3::Path {
            closed: val.closed,
            nodes: val.nodes.into_iter().map(Into::into).collect(),
            attr: BTreeMap::new(),
        }
    }
}

impl From<glyphs2::Glyph> for glyphs3::Glyph {
    fn from(val: glyphs2::Glyph) -> Self {
        glyphs3::Glyph {
            name: val.name,
            production: val.production,
            script: val.script,
            category: val.category,
            color: val.color,
            export: val.export,
            kern_left: val.kern_left,
            kern_right: val.kern_right,
            kern_top: val.kern_top,
            last_change: val.last_change,
            layers: val.layers.into_iter().map(Into::into).collect(),
            unicode: val.unicode,
            ..Default::default()
        }
    }
}

impl From<glyphs2::Glyphs2> for glyphs3::Glyphs3 {
    fn from(val: glyphs2::Glyphs2) -> Self {
        let axes = val.determine_axes();
        let properties = val.glyphs3_properties();
        let metrics = val.glyphs3_metrics();
        let mut font = glyphs3::Glyphs3 {
            app_version: val.app_version,
            format_version: 3,
            display_strings: val.display_strings,
            axes: vec![], // Fix you later
            classes: val.classes,
            custom_parameters: val.custom_parameters,
            date: val.date,
            family_name: val.family_name,
            feature_prefixes: val.feature_prefixes,
            features: val.features,
            masters: val
                .masters
                .iter()
                .map(|x| x.to_glyphs3(&axes, &metrics))
                .collect(),
            glyphs: val.glyphs.into_iter().map(Into::into).collect(),
            instances: val.instances.iter().map(|x| x.to_glyphs3(&axes)).collect(),
            keep_alternates_together: val.keep_alternates_together,
            kerning: val.kerning,
            kerning_rtl: BTreeMap::new(),
            kerning_vertical: val.kerning_vertical,
            metrics,
            note: "".to_string(),
            numbers: vec![],
            properties,
            settings: glyphs3::Settings {
                disables_automatic_alignment: val.disables_automatic_alignment,
                disables_nice_names: val.disables_nice_names,
                grid_length: val.grid_length,
                grid_sub_division: val.grid_sub_division,
                keyboard_increment: val.keyboard_increment,
                keyboard_increment_big: None,
                keyboard_increment_huge: None,
            },
            stems: vec![], // XXX
            units_per_em: val.units_per_em,
            user_data: val.user_data,
            version_major: val.version_major,
            version_minor: val.version_minor,
        };
        font.axes = axes;
        font
    }
}

impl glyphs2::Master {
    fn axis_values(&self, num_axes: usize) -> Vec<i32> {
        match num_axes {
            0 => vec![self.weight_value],
            1 => vec![self.weight_value, self.width_value],
            2 => vec![self.weight_value, self.width_value, self.custom_value],
            3 => vec![
                self.weight_value,
                self.width_value,
                self.custom_value,
                self.custom_value_1,
            ],
            4 => vec![
                self.weight_value,
                self.width_value,
                self.custom_value,
                self.custom_value_1,
                self.custom_value_2,
            ],
            _ => vec![
                self.weight_value,
                self.width_value,
                self.custom_value,
                self.custom_value_1,
                self.custom_value_2,
                self.custom_value_3,
            ],
        }
    }

    fn to_glyphs3(&self, axes: &[Axis], metrics: &[Metric]) -> glyphs3::Master {
        let alignment_to_overshoot: Vec<(f32, f32)> = self
            .alignment_zones
            .iter()
            .map(|z| (z.position, z.overshoot))
            .collect();
        let find_overshoot = |v| {
            alignment_to_overshoot
                .iter()
                .find(|(pos, _)| *pos == v)
                .map(|(_, over)| *over)
                .unwrap_or(0.0)
        };
        let metric_values = metrics
            .iter()
            .map(|m| match m.metric_type {
                Some(MetricType::Ascender) => MetricValue {
                    pos: self.ascender.unwrap_or_default(),
                    over: find_overshoot(self.ascender.unwrap_or_default()),
                },
                Some(MetricType::Baseline) => MetricValue {
                    pos: 0.0,
                    over: find_overshoot(0.0),
                },
                Some(MetricType::CapHeight) => MetricValue {
                    pos: self.cap_height.unwrap_or_default(),
                    over: find_overshoot(self.cap_height.unwrap_or_default()),
                },
                Some(MetricType::Descender) => MetricValue {
                    pos: self.descender.unwrap_or_default(),
                    over: find_overshoot(self.descender.unwrap_or_default()),
                },
                Some(MetricType::XHeight) => MetricValue {
                    pos: self.x_height.unwrap_or_default(),
                    over: find_overshoot(self.x_height.unwrap_or_default()),
                },
                _ => panic!("Can't happen"),
            })
            .collect();

        let mut name_particles = vec![];
        let axis_tags: BTreeSet<&str> = axes.iter().map(|a| a.tag.as_str()).collect();
        if axis_tags.contains("wght") && self.weight != "Regular" {
            name_particles.push(self.weight.as_str());
        }
        if axis_tags.contains("wdth") && self.width != "Regular" {
            name_particles.push(self.width.as_str());
        }
        if let Some(custom) = self.custom.as_ref() {
            name_particles.push(custom.as_str());
        }
        let name = if name_particles.is_empty() {
            "Regular".to_string()
        } else {
            name_particles.join(" ")
        };
        glyphs3::Master {
            id: self.id.clone(),
            user_data: self.user_data.clone(),
            axes_values: self
                .axis_values(axes.len())
                .iter()
                .copied()
                .map(|x| x as f32)
                .collect(),
            custom_parameters: self.custom_parameters.clone(),
            guides: vec![],
            icon_name: self.icon_name.clone(),
            metric_values,
            name,
            number_values: vec![],
            properties: vec![],  // XXX - maybe some custom parameters?
            stem_values: vec![], // XXX
            visible: self.visible,
        }
    }
}

impl glyphs2::Glyphs2 {
    pub fn determine_axes(&self) -> Vec<glyphs3::Axis> {
        // If we have an Axes custom parameter, start with that.
        if let Some(axes_param) = self.custom_parameters.iter().find(|x| x.name == "Axes") {
            if let Some(axes_cp) = axes_param.value.as_array() {
                let axes = axes_cp
                    .iter()
                    .flat_map(|x| x.as_dict())
                    .map(|d| glyphs3::Axis {
                        name: d.get("Name").map(|x| x.to_string()).unwrap_or_default(),
                        tag: d.get("Tag").map(|x| x.to_string()).unwrap_or_default(),
                        hidden: d.contains_key("Hidden"),
                    })
                    .collect::<Vec<_>>();
                return axes;
            }
        }
        // Else we only have one or two "default" axes (weight/width/both); work it out the hard way
        let mut axes = vec![];
        let weight_values: Vec<i32> = self.masters.iter().map(|x| x.weight_value).collect();
        let (weight_min, weight_max) = (
            weight_values.iter().copied().min().unwrap_or(100),
            weight_values.iter().copied().max().unwrap_or(100),
        );
        if weight_min != weight_max {
            axes.push(glyphs3::Axis {
                name: "Weight".to_string(),
                tag: "wght".to_string(),
                hidden: false,
            });
        }
        let width_values: Vec<i32> = self.masters.iter().map(|x| x.width_value).collect();
        let (width_min, width_max) = (
            width_values.iter().copied().min().unwrap_or(100),
            width_values.iter().copied().max().unwrap_or(100),
        );
        if width_min != width_max {
            axes.push(glyphs3::Axis {
                name: "Width".to_string(),
                tag: "wdth".to_string(),
                hidden: false,
            });
        }
        axes
    }

    fn glyphs3_properties(&self) -> Vec<Property> {
        let mut properties = vec![];
        if let Some(designer) = self.designer.as_ref() {
            properties.push(Property::localized_with_default(
                LocalizedPropertyKey::Designers,
                designer.clone(),
            ));
        }
        if let Some(design_url) = self.designer_url.as_ref() {
            properties.push(Property::singular(
                glyphs3::SingularPropertyKey::DesignerUrl,
                design_url.clone(),
            ))
        }
        if let Some(manufacturer) = self.manufacturer.as_ref() {
            properties.push(Property::localized_with_default(
                LocalizedPropertyKey::Manufacturers,
                manufacturer.clone(),
            ));
        }
        if let Some(manufacturer_url) = self.manufacturer_url.as_ref() {
            properties.push(Property::singular(
                glyphs3::SingularPropertyKey::ManufacturerUrl,
                manufacturer_url.clone(),
            ))
        }
        properties
    }

    fn glyphs3_metrics(&self) -> Vec<Metric> {
        let mut metrics = vec![];
        if self.masters.iter().any(|m| m.ascender.is_some()) {
            metrics.push(Metric {
                name: "ascender".to_string(),
                filter: None,
                metric_type: Some(MetricType::Ascender),
            });
        }
        metrics.push(Metric {
            name: "baseline".to_string(),
            filter: None,
            metric_type: Some(MetricType::Baseline),
        });
        if self.masters.iter().any(|m| m.cap_height.is_some()) {
            metrics.push(Metric {
                name: "capHeight".to_string(),
                filter: None,
                metric_type: Some(MetricType::CapHeight),
            });
        }
        if self.masters.iter().any(|m| m.descender.is_some()) {
            metrics.push(Metric {
                name: "descender".to_string(),
                filter: None,
                metric_type: Some(MetricType::Descender),
            });
        }
        if self.masters.iter().any(|m| m.x_height.is_some()) {
            metrics.push(Metric {
                name: "xHeight".to_string(),
                filter: None,
                metric_type: Some(MetricType::XHeight),
            });
        }
        metrics
    }
}

impl glyphs2::Instance {
    fn axis_values(&self, num_axes: usize) -> Vec<f32> {
        match num_axes {
            0 => vec![self.weight_value],
            1 => vec![self.weight_value, self.width_value],
            2 => vec![self.weight_value, self.width_value, self.custom_value],
            3 => vec![
                self.weight_value,
                self.width_value,
                self.custom_value,
                self.custom_value_1,
            ],
            4 => vec![
                self.weight_value,
                self.width_value,
                self.custom_value,
                self.custom_value_1,
                self.custom_value_2,
            ],
            _ => vec![
                self.weight_value,
                self.width_value,
                self.custom_value,
                self.custom_value_1,
                self.custom_value_2,
                self.custom_value_3,
            ],
        }
    }

    fn to_glyphs3(&self, axes: &[Axis]) -> glyphs3::Instance {
        glyphs3::Instance {
            axes_values: self.axis_values(axes.len()),
            custom_parameters: self.custom_parameters.clone(),
            exports: self.exports,
            is_bold: self.is_bold,
            is_italic: self.is_italic,
            link_style: self.link_style.clone(),
            name: self.name.clone(),
            properties: vec![],
            user_data: self.user_data.clone(),
            weight_class: self.weight_class.clone().map(openstep_plist::Plist::String),
            width_class: self.width_class.clone().map(openstep_plist::Plist::String),
            ..Default::default()
        }
    }
}
