use norad::{Font, Name};
use otmath::{support_scalar, Location, VariationModel};
use std::collections::BTreeSet;

fn get_kerning(f: &Font, l: &Name, r: &Name) -> f64 {
    f.kerning
        .get(l)
        .and_then(|pair| pair.get(r))
        .copied()
        .unwrap_or(0.0)
}

fn set_kerning(f: &mut Font, l: &Name, r: &Name, value: f64) {
    f.kerning
        .entry(l.clone())
        .or_default()
        .insert(r.clone(), value);
}

fn delete_kerning(f: &mut Font, l: &Name, r: &Name) {
    if let Some(l) = f.kerning.get_mut(l) {
        l.remove(r);
    }
}

fn tidy_kerning(f: &mut Font) {
    let mut kill_list = vec![];
    for (l, r) in f.kerning.iter() {
        if r.is_empty() {
            kill_list.push(l.clone());
        }
    }
    for l in kill_list {
        f.kerning.remove(&l);
    }
}
pub fn interpolate_kerning(
    output: &mut Font,
    masters: &[Font],
    model: &VariationModel<String>,
    location: &Location<String>,
) {
    // Gather all kern pairs
    let mut pairs: BTreeSet<(Name, Name)> = BTreeSet::new();
    for master in masters {
        let this_kerning = &master.kerning;
        for (left, kerns) in this_kerning {
            for right in kerns.keys() {
                pairs.insert((left.clone(), right.clone()));
            }
        }
    }

    for (l, r) in pairs.iter() {
        let default_kerning: f64 = get_kerning(output, l, r);
        let all_kerning: Vec<Option<f32>> = masters
            .iter()
            .map(|x| Some((get_kerning(x, l, r) - default_kerning) as f32))
            .collect();
        let deltas_and_supports = model.get_deltas_and_supports(&all_kerning);
        let (deltas, support_scalars): (Vec<f32>, Vec<f32>) = deltas_and_supports
            .into_iter()
            .map(|(x, y)| (x, support_scalar(location, &y)))
            .unzip();

        let interpolated_kern = model
            .interpolate_from_deltas_and_scalars(&deltas, &support_scalars)
            .expect("Couldn't interpolate");
        let new_kern = default_kerning + interpolated_kern as f64;
        if l == &"V" {
            println!("Kerning for {}/{}", l, r);
            println!(" default: {}", default_kerning);
            println!(
                " others: {:?}",
                masters
                    .iter()
                    .map(|x| get_kerning(x, l, r))
                    .collect::<Vec<f64>>()
            );
            println!(" Deltas: {:?}", all_kerning);
            println!(" Interpolated kern: {:?}", interpolated_kern);
            println!(" New kern: {:?}", new_kern);
        }
        if new_kern != 0.0 {
            set_kerning(output, l, r, new_kern)
        } else if default_kerning != 0.0 {
            delete_kerning(output, l, r)
        }
    }
    tidy_kerning(output);
}
