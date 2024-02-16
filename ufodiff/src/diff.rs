use std::collections::{HashMap, HashSet};

use serde_json_diff::{Difference, EntryDifference};

pub trait Diff {
    fn diff(&self, other: &Self) -> HashMap<String, String>;
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
    fn diff(&self, other: &Self) -> HashMap<String, String> {
        let mut result: HashMap<String, String> = HashMap::new();
        let all_glyphs: HashSet<&norad::Name> =
            self.iter().chain(other.iter()).map(|g| g.name()).collect();
        for glyph in all_glyphs {
            let g1 = self.get_glyph(glyph);
            let g2 = other.get_glyph(glyph);
            if g1.is_none() {
                result.insert(glyph.as_str().to_string(), "Not present in LHS".to_string());
            } else if g2.is_none() {
                result.insert(glyph.as_str().to_string(), "Not present in RHS".to_string());
            } else {
                let diffs = g1.unwrap().diff(g2.unwrap());
                if !diffs.is_empty() {
                    println!("Differences in glyph {}", glyph);
                    for (key, value) in diffs {
                        println!("\t{:30}{}", key, value);
                    }
                }
            }
        }
        result
    }
}

impl Diff for norad::Plist {
    fn diff(&self, other: &Self) -> HashMap<String, String> {
        flat_dict_diff(self, other)
    }
}

impl Diff for norad::FontInfo {
    fn diff(&self, other: &Self) -> HashMap<String, String> {
        flat_dict_diff(self, other)
    }
}

impl Diff for norad::Glyph {
    fn diff(&self, other: &Self) -> HashMap<String, String> {
        let mut result = HashMap::new();
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

        result
    }
}

pub fn flat_dict_diff<T>(this: &T, other: &T) -> HashMap<String, String>
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
    let mut result: HashMap<String, String> = HashMap::new();
    clean_nulls(&mut lhs);
    clean_nulls(&mut rhs);

    if let Some(diffs) = serde_json_diff::values(lhs.clone(), rhs.clone()) {
        match diffs {
            Difference::Object { different_entries } => {
                for (key, value) in different_entries.0 {
                    match value {
                        EntryDifference::Missing { value } => {
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
                                        "Different scalar values: {} v {}",
                                        lhs_value, rhs_value
                                    ),
                                );
                            }
                            Difference::Type {
                                source_type,
                                target_type,
                                target_value,
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
                                result.insert(key, "Different array values".to_string());
                            }
                            Difference::Object { different_entries } => {
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
