use crate::Problem;
use designspace::{Axis, Designspace};

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
    problems.into_iter()
}
