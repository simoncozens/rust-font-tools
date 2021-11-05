use babelfont::Font;
use fonttools::layout::common::{LanguageSystem, Lookup, LookupFlags, Script, ScriptList};
use fonttools::layout::gpos2::{PairPos, PairPositioningMap};
use fonttools::layout::valuerecord::ValueRecord;
use fonttools::tables::GPOS::{Positioning, GPOS};
use fonttools::{tag, valuerecord};
use std::collections::BTreeMap;
use std::iter::FromIterator;

macro_rules! hashmap {
        ($($k:expr => $v:expr),* $(,)?) => {
            std::collections::BTreeMap::<_, _>::from_iter(std::array::IntoIter::new([$(($k, $v),)*]))
        };
    }

pub fn build_kerning(font: &Font, mapping: &BTreeMap<String, u16>) -> GPOS {
    let master = font.default_master().unwrap();
    let mut kerntable: PairPositioningMap = BTreeMap::new();
    for ((l, r), value) in master.kerning.iter() {
        let l_array: Vec<String> = if let Some(stripped) = l.strip_prefix('@') {
            font.kern_groups.get(stripped).unwrap_or(&vec![]).to_vec()
        } else {
            vec![l.clone()]
        };
        let r_array: Vec<String> = if let Some(stripped) = r.strip_prefix('@') {
            font.kern_groups.get(stripped).unwrap_or(&vec![]).to_vec()
        } else {
            vec![r.clone()]
        };

        for l in &l_array {
            for r in &r_array {
                add_single_kern(
                    &mut kerntable,
                    l.to_string(),
                    r.to_string(),
                    *value,
                    mapping,
                );
            }
        }
    }
    let pairpos = PairPos { mapping: kerntable };
    GPOS {
        lookups: vec![Lookup {
            flags: LookupFlags::empty(),
            mark_filtering_set: None,
            rule: Positioning::Pair(vec![pairpos]),
        }],
        scripts: ScriptList {
            scripts: hashmap!(tag!("DFLT") => Script {
                default_language_system: Some(
                    LanguageSystem {
                        required_feature: None,
                        feature_indices: vec![
                            0,
                       ],
                    },
                ),
                language_systems: BTreeMap::new()
            }),
        },
        features: vec![(tag!("kern"), vec![0], None)],
    }
}

fn add_single_kern(
    kerntable: &mut PairPositioningMap,
    l: String,
    r: String,
    value: i16,
    mapping: &BTreeMap<String, u16>,
) {
    let l_gid = mapping.get(&l);
    let r_gid = mapping.get(&r);
    if l_gid.is_none() {
        // println!("Unknown glyph {:?} in kerning table", l);
        return;
    }
    let l_gid = l_gid.unwrap();
    if r_gid.is_none() {
        // println!("Unknown glyph {:?} in kerning table", r);
        return;
    }
    let r_gid = r_gid.unwrap();
    kerntable.insert(
        (*l_gid, *r_gid),
        (valuerecord!(xAdvance = value), valuerecord!()),
    );
}
/*
PairPos {
                mapping: btreemap!(
                    (0,289)   => (valuerecord!(xAdvance=-90),  valuerecord!()),
                    (0,332)   => (valuerecord!(xAdvance=-150), valuerecord!()),
                    (332,833) => (valuerecord!(xAdvance=100),  valuerecord!()),
                )
            }
            */
