use crate::Problem;
use designspace::{Axis, Designspace, Mapping};

pub(crate) fn check_designspace(ds: &Designspace) -> impl Iterator<Item = Problem> + '_ {
    let axis_problems = ds.axes.axis.iter().map(check_ds_axis).flatten();
    let mut other_problems: Vec<Problem> = vec![];
    if ds.default_master().is_none() {
        other_problems.push(Problem {
            area: "designspace".to_string(),
            glyph: None,
            location: None,
            master: None,
            description: "couldn't find default master".to_string(),
        })
    }
    axis_problems.chain(other_problems.into_iter())
}

fn check_ds_axis(axis: &Axis) -> impl Iterator<Item = Problem> {
    let mut problems: Vec<Problem> = vec![];
    if axis.default < axis.minimum {
        problems.push(Problem {
            area: "designspace".to_string(),
            glyph: None,
            master: None,
            location: Some(format!("axis {}", axis.tag)),
            description: format!(
                "default {} is less than minimum {}",
                axis.default, axis.minimum
            ),
        })
    }
    if axis.default > axis.maximum {
        problems.push(Problem {
            area: "designspace".to_string(),
            glyph: None,
            master: None,
            location: Some(format!("{} axis", axis.tag)),
            description: format!(
                "default {} is more than maximum {}",
                axis.default, axis.maximum
            ),
        })
    }
    if let Some(map) = &axis.map {
        problems.extend(check_map(map, axis));
    }
    problems.into_iter()
}

fn check_map(map: &[Mapping], axis: &Axis) -> impl Iterator<Item = Problem> {
    let mut problems: Vec<Problem> = vec![];
    // Input mapping should be sorted
    let inputs: Vec<f32> = map.iter().map(|x| x.input).collect();
    if !inputs.is_sorted() {
        problems.push(Problem {
            area: "designspace".to_string(),
            glyph: None,
            master: None,
            location: Some(format!("{} axis", axis.tag)),
            description: "mapping is not sorted".to_string(),
        })
    }

    // Mapping should contain min/default/max values
    if !inputs
        .iter()
        .any(|i| (i - axis.minimum as f32).abs() < f32::EPSILON)
    {
        problems.push(Problem {
            area: "designspace".to_string(),
            glyph: None,
            master: None,
            location: Some(format!("{} axis", axis.tag)),
            description: format!("mapping does not contain minimum value {}", axis.minimum),
        })
    }
    if !inputs
        .iter()
        .any(|i| (i - axis.maximum as f32).abs() < f32::EPSILON)
    {
        problems.push(Problem {
            area: "designspace".to_string(),
            glyph: None,
            master: None,
            location: Some(format!("{} axis", axis.tag)),
            description: format!("mapping does not contain maximum value {}", axis.maximum),
        })
    }
    if !inputs
        .iter()
        .any(|i| (i - axis.default as f32).abs() < f32::EPSILON)
    {
        problems.push(Problem {
            area: "designspace".to_string(),
            glyph: None,
            master: None,
            location: Some(format!("{} axis", axis.tag)),
            description: format!("mapping does not contain default value {}", axis.default),
        })
    }
    problems.into_iter()
}
