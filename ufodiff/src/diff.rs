use indexmap::{IndexMap, IndexSet};
use std::collections::BTreeSet;

use serde_json_diff::{Difference, EntryDifference};

pub type DiffResult = IndexMap<String, String>;

pub trait Diff {
    fn diff(&self, other: &Self) -> DiffResult;
}

fn extend_with<S: Into<String>>(original: &mut DiffResult, title: S, more_diffs: DiffResult) {
    let title = title.into();
    for (key, value) in more_diffs {
        original.insert(format!("{}/{}", &title, key), value);
    }
}

impl<T: Diff> Diff for Vec<T> {
    fn diff(&self, other: &Self) -> DiffResult
    where
        T: Diff,
    {
        let mut result: DiffResult = IndexMap::new();
        if self.len() != other.len() {
            result.insert(
                "length".to_string(),
                format!("{} v {}", self.len(), other.len()),
            );
        } else {
            for (i, (a, b)) in self.iter().zip(other.iter()).enumerate() {
                extend_with(&mut result, format!("{}", i), a.diff(b));
            }
        }
        result
    }
}

fn clean_nulls(o: &mut serde_json::Value) {
    // Delete all null values on both sides
    let mut nulls = vec![];
    for (k, v) in o.as_object().unwrap().iter() {
        if v.is_null() {
            nulls.push(k.clone());
        }
    }
    for k in nulls {
        o.as_object_mut().unwrap().remove(&k);
    }
}

impl Diff for norad::Layer {
    fn diff(&self, other: &Self) -> DiffResult {
        let mut result: DiffResult = IndexMap::new();
        let all_glyphs: IndexSet<&norad::Name> =
            self.iter().chain(other.iter()).map(|g| g.name()).collect();
        for glyph in all_glyphs {
            let g1 = self.get_glyph(glyph);
            let g2 = other.get_glyph(glyph);
            if g1.is_none() {
                result.insert(glyph.as_str().to_string(), "Not present in LHS".to_string());
            } else if g2.is_none() {
                result.insert(glyph.as_str().to_string(), "Not present in RHS".to_string());
            } else {
                extend_with(
                    &mut result,
                    format!("glyph {}", glyph),
                    g1.unwrap().diff(g2.unwrap()),
                );
            }
        }
        if self.color != other.color {
            result.insert(
                "color".to_string(),
                format!("{:?} v {:?}", self.color, other.color),
            );
        }
        result.extend(self.lib.diff(&other.lib));
        result
    }
}

impl Diff for norad::Plist {
    fn diff(&self, other: &Self) -> DiffResult {
        flat_dict_diff(self, other)
    }
}

impl Diff for norad::Anchor {
    fn diff(&self, other: &Self) -> DiffResult {
        let mut result = IndexMap::new();
        if self.name != other.name {
            result.insert(
                "name".to_string(),
                format!("{:?} v {:?}", self.name, other.name),
            );
        }
        if (self.x - other.x).abs() > 0.001 {
            result.insert("x".to_string(), format!("{} v {}", self.x, other.x));
        }
        if (self.y - other.y).abs() > 0.001 {
            result.insert("y".to_string(), format!("{} v {}", self.y, other.y));
        }
        result
    }
}

impl Diff for norad::Component {
    fn diff(&self, other: &Self) -> DiffResult {
        let mut result = IndexMap::new();
        if self.base != other.base {
            result.insert(
                "base".to_string(),
                format!("{:?} v {:?}", self.base, other.base),
            );
        }
        extend_with(
            &mut result,
            "transform",
            self.transform.diff(&other.transform),
        );
        result
    }
}
impl Diff for norad::AffineTransform {
    fn diff(&self, other: &Self) -> DiffResult {
        let mut result = IndexMap::new();
        if (self.x_scale - other.x_scale).abs() > 0.001 {
            result.insert(
                "x_scale".to_string(),
                format!("{} v {}", self.x_scale, other.x_scale),
            );
        }
        if (self.xy_scale - other.xy_scale).abs() > 0.001 {
            result.insert(
                "xy_scale".to_string(),
                format!("{} v {}", self.xy_scale, other.xy_scale),
            );
        }
        if (self.yx_scale - other.yx_scale).abs() > 0.001 {
            result.insert(
                "yx_scale".to_string(),
                format!("{} v {}", self.yx_scale, other.yx_scale),
            );
        }
        if (self.y_scale - other.y_scale).abs() > 0.001 {
            result.insert(
                "y_scale".to_string(),
                format!("{} v {}", self.y_scale, other.y_scale),
            );
        }
        if (self.x_offset - other.x_offset).abs() > 0.001 {
            result.insert(
                "x_offset".to_string(),
                format!("{} v {}", self.x_offset, other.x_offset),
            );
        }
        if (self.y_offset - other.y_offset).abs() > 0.001 {
            result.insert(
                "y_offset".to_string(),
                format!("{} v {}", self.y_offset, other.y_offset),
            );
        }
        result
    }
}

impl Diff for norad::Contour {
    fn diff(&self, other: &Self) -> DiffResult {
        let mut result = IndexMap::new();
        extend_with(
            &mut result,
            "Points".to_string(),
            self.points.diff(&other.points),
        );
        result
    }
}

impl Diff for norad::ContourPoint {
    fn diff(&self, other: &Self) -> DiffResult {
        let mut result = IndexMap::new();
        if (self.x - other.x).abs() > 0.001 {
            result.insert("x".to_string(), format!("{} v {}", self.x, other.x));
        }
        if (self.y - other.y).abs() > 0.001 {
            result.insert("y".to_string(), format!("{} v {}", self.y, other.y));
        }
        if self.typ != other.typ {
            result.insert(
                "type".to_string(),
                format!("{:?} v {:?}", self.typ, other.typ),
            );
        }
        result
    }
}

impl Diff for norad::FontInfo {
    fn diff(&self, other: &Self) -> DiffResult {
        flat_dict_diff(self, other)
    }
}

impl Diff for norad::Groups {
    fn diff(&self, other: &Self) -> DiffResult {
        flat_dict_diff(self, other)
    }
}

impl Diff for norad::Glyph {
    fn diff(&self, other: &Self) -> DiffResult {
        let mut result = IndexMap::new();
        if (self.height - other.height).abs() > 0.001 {
            result.insert(
                "height".to_string(),
                format!("{} v {}", self.height, other.height),
            );
        }
        if (self.width - other.width).abs() > 0.001 {
            result.insert(
                "width".to_string(),
                format!("{} v {}", self.width, other.width),
            );
        }
        if self.codepoints != other.codepoints {
            result.insert(
                "codepoints".to_string(),
                format!("{:?} v {:?}", self.codepoints, other.codepoints),
            );
        }
        extend_with(
            &mut result,
            "Anchors".to_string(),
            self.anchors.diff(&other.anchors),
        );
        extend_with(
            &mut result,
            "Components".to_string(),
            self.components.diff(&other.components),
        );
        extend_with(
            &mut result,
            "Contours".to_string(),
            self.contours.diff(&other.contours),
        );
        result
    }
}

pub fn flat_dict_diff<T>(this: &T, other: &T) -> DiffResult
where
    T: serde::Serialize,
{
    let mut lhs = serde_json::to_value(this).unwrap_or_else(|err| {
        panic!(
            "Couldn't convert left hand side value to JSON. Serde error: {}",
            err
        )
    });
    let mut rhs = serde_json::to_value(other).unwrap_or_else(|err| {
        panic!(
            "Couldn't convert right hand side value to JSON. Serde error: {}",
            err
        )
    });
    let mut result: DiffResult = IndexMap::new();
    clean_nulls(&mut lhs);
    clean_nulls(&mut rhs);

    if let Some(diffs) = serde_json_diff::values(lhs.clone(), rhs.clone()) {
        match diffs {
            Difference::Object { different_entries } => {
                for (key, value) in different_entries.0 {
                    match value {
                        EntryDifference::Missing { .. } => {
                            result.insert(key, "Not present in LHS".to_string());
                        }
                        EntryDifference::Extra => {
                            result.insert(key, "Not present in RHS".to_string());
                        }
                        EntryDifference::Value { value_diff } => match value_diff {
                            Difference::Scalar(_) => {
                                let lhs_value = lhs.get(&key).unwrap();
                                let rhs_value = rhs.get(&key).unwrap();
                                result.insert(
                                    key,
                                    format!(
                                        "Different scalar values:\n\t\tLHS: {:}\n\t\tRHS: {:}",
                                        lhs_value, rhs_value
                                    ),
                                );
                            }
                            Difference::Type {
                                source_type,
                                target_type,
                                ..
                            } => {
                                result.insert(
                                    key,
                                    format!(
                                        "LHS was a {:?} but RHS was a {:?}",
                                        source_type, target_type
                                    ),
                                );
                            }
                            Difference::Array(_) => {
                                let lhs_value = lhs.get(&key).unwrap();
                                let rhs_value = rhs.get(&key).unwrap();

                                result.insert(
                                    key,
                                    format!(
                                        "Different array values:\n\t\tLHS: {:}\n\t\tRHS: {:}",
                                        lhs_value, rhs_value
                                    ),
                                );
                            }
                            Difference::Object { .. } => {
                                result.insert(key, "Different object values".to_string());
                            }
                        },
                    }
                }
            }
            _ => panic!("Font info was not an object?!"),
        }
    }
    result
}

impl Diff for norad::Kerning {
    fn diff(&self, other: &Self) -> DiffResult {
        let mut result: DiffResult = IndexMap::new();
        let mut all_pairs = BTreeSet::new();
        for (left, our_left) in self.iter() {
            for right in our_left.keys() {
                all_pairs.insert((left, right));
            }
        }
        for (left, their_left) in other.iter() {
            for right in their_left.keys() {
                all_pairs.insert((left, right));
            }
        }

        for (left, right) in all_pairs {
            let our_value = self.get(left).and_then(|m| m.get(right));
            let their_value = other.get(left).and_then(|m| m.get(right));
            if let Some(our_value) = our_value {
                if let Some(theirs) = their_value {
                    if (our_value - theirs).abs() > 0.001 {
                        result.insert(
                            format!("{}/{}", left, right),
                            format!("{} v {}", our_value, theirs),
                        );
                    }
                } else {
                    result.insert(
                        format!("{}/{}", left, right),
                        format!("Not present in RHS, {} in LHS", our_value),
                    );
                }
            }
            // Ours is none, what about theirs
            else if their_value.is_some() {
                result.insert(
                    format!("{}/{}", left, right),
                    format!("Not present in LHS, {} in RHS", their_value.unwrap()),
                );
            }
        }
        result
    }
}
