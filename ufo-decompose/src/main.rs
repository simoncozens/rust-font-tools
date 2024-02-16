//! Decompose mixed glyphs in a UFO file

use clap::Parser;
use norad::{Contour, Font, Glyph, Layer};
use std::collections::BTreeMap;

/// Decompose mixed glyphs in a UFO file
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Increase logging
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    /// Layer to decompose
    #[clap(short, long)]
    layer: Option<String>,

    /// Input UFO
    input: String,
    /// Output UFO
    output: String,
}

fn to_kurbo(t: norad::AffineTransform) -> kurbo::Affine {
    kurbo::Affine::new([
        t.x_scale, t.xy_scale, t.yx_scale, t.y_scale, t.x_offset, t.y_offset,
    ])
}

fn decomposed_components(glyph: &Glyph, layer: &Layer) -> Vec<Contour> {
    let mut contours = Vec::new();

    let mut stack: Vec<(&norad::Component, kurbo::Affine)> = Vec::new();

    for component in &glyph.components {
        stack.push((component, to_kurbo(component.transform)));

        while let Some((component, transform)) = stack.pop() {
            let referenced_glyph = match layer.get_glyph(&component.base) {
                Some(g) => g,
                None => continue,
            };
            for contour in &referenced_glyph.contours {
                let mut decomposed_contour = Contour::default();
                for node in &contour.points {
                    let new_point = transform * kurbo::Point::new(node.x, node.y);
                    decomposed_contour.points.push(norad::ContourPoint::new(
                        new_point.x,
                        new_point.y,
                        node.typ.clone(),
                        node.smooth,
                        None,
                        None,
                        None,
                    ));
                }
                contours.push(decomposed_contour);
            }

            // We need to do this backwards.
            for new_component in referenced_glyph.components.iter().rev() {
                let new_transform: kurbo::Affine = to_kurbo(new_component.transform);
                stack.push((new_component, transform * new_transform));
            }
        }
    }

    contours
}

fn main() {
    // Command line handling
    let args = Args::parse();

    env_logger::init_from_env(env_logger::Env::default().filter_or(
        env_logger::DEFAULT_FILTER_ENV,
        match args.verbose {
            0 => "warn",
            1 => "info",
            _ => "debug",
        },
    ));

    let mut input = Font::load(args.input).expect("Couldn't open UFO file");
    let layer = if let Some(layername) = args.layer {
        input.layers.get_mut(&layername).expect("Layer not found")
    } else {
        input.layers.default_layer_mut()
    };
    let mut decomposed: BTreeMap<String, Vec<Contour>> = BTreeMap::new();
    for glyph in layer.iter() {
        decomposed.insert(glyph.name.to_string(), decomposed_components(glyph, layer));
    }

    for glyph in layer.iter_mut() {
        if glyph.component_count() == 0 || glyph.contours.is_empty() {
            continue;
        }
        log::debug!("Decomposing mixed glyph {:?}", glyph.name);
        if let Some(contours) = decomposed.get(&glyph.name.to_string()) {
            for c in contours {
                glyph.contours.push(c.clone());
            }
            glyph.components.clear();
        }
    }
    input.save(args.output).expect("Could not save");
}
