//! Interpolate an instance UFO in a designspace
use clap::Parser;
use nalgebra::DVector;
use norad::designspace::{Axis, DesignSpaceDocument, Dimension, Source};
use norad::Glyph;
use otmath::{normalize_value, ot_round, piecewise_linear_map, Location, VariationModel};
use rayon::prelude::*;
use rbf_interp::Scatter;
use regex::Regex;
use std::collections::BTreeMap;
use std::path::Path;

type Tuple = Vec<f32>;
struct NormalizedLocation(Tuple);

trait BetterAxis {
    fn normalize_userspace_value(&self, l: f32) -> f32;
    fn normalize_designspace_value(&self, l: f32) -> f32;
    fn userspace_to_designspace(&self, l: f32) -> f32;
    fn designspace_to_userspace(&self, l: f32) -> f32;
    fn default_map(&self) -> Vec<(f32, f32)>;
}

impl BetterAxis for Axis {
    fn normalize_userspace_value(&self, l: f32) -> f32 {
        normalize_value(
            l,
            self.minimum.unwrap_or(0.0),
            self.maximum.unwrap_or(0.0),
            self.default,
        )
    }
    fn normalize_designspace_value(&self, l: f32) -> f32 {
        if self.map.is_none() || self.map.as_ref().unwrap().is_empty() {
            return self.normalize_userspace_value(l);
        }

        normalize_value(
            self.designspace_to_userspace(l),
            self.minimum.unwrap_or(0.0),
            self.maximum.unwrap_or(0.0),
            self.default as f32,
        )
    }
    fn default_map(&self) -> Vec<(f32, f32)> {
        vec![
            (self.minimum.unwrap(), self.minimum.unwrap()),
            (self.default as f32, self.default as f32),
            (self.maximum.unwrap(), self.maximum.unwrap()),
        ]
    }

    fn userspace_to_designspace(&self, l: f32) -> f32 {
        let mapping: Vec<(f32, f32)> = self.map.as_ref().map_or_else(
            || self.default_map(),
            |map| {
                map.iter()
                    .map(|mapping| (mapping.input, mapping.output))
                    .collect()
            },
        );
        piecewise_linear_map(&mapping, l as f32)
    }
    fn designspace_to_userspace(&self, l: f32) -> f32 {
        let mapping: Vec<(f32, f32)> = self.map.as_ref().map_or_else(
            || self.default_map(),
            |map| {
                map.iter()
                    .map(|mapping| (mapping.output, mapping.input))
                    .collect()
            },
        );

        piecewise_linear_map(&mapping, l)
    }
}

trait BetterDesignspace {
    fn location_to_tuple(&self, loc: &[Dimension]) -> Vec<f32>;
    fn default_master(&self) -> Option<&Source>;
    fn variation_model(&self) -> VariationModel<String>;
    fn normalize_location(&self, loc: &[Dimension]) -> NormalizedLocation;
}
impl BetterDesignspace for DesignSpaceDocument {
    /// Converts a location to a tuple
    fn location_to_tuple(&self, loc: &[Dimension]) -> Vec<f32> {
        let mut tuple = vec![];
        let defaults = self.axes.iter().map(|ax| ax.default);
        for (axis, default) in self.axes.iter().zip(defaults) {
            let name = &axis.name;
            let dim = loc.iter().find(|d| d.name == *name);
            if let Some(dim) = dim {
                tuple.push(dim.xvalue.unwrap_or(0.0));
            } else {
                tuple.push(default);
            }
        }
        tuple
    }
    fn default_master(&self) -> Option<&Source> {
        let defaults: BTreeMap<String, f32> = self
            .axes
            .iter()
            .map(|ax| (ax.name.clone(), ax.userspace_to_designspace(ax.default)))
            .collect();
        for source in self.sources.iter() {
            let mut maybe = true;
            for loc in source.location.iter() {
                if defaults.get(&loc.name) != loc.xvalue.as_ref() {
                    maybe = false;
                    break;
                }
            }
            if maybe {
                return Some(source);
            }
        }
        return None;
    }
    fn variation_model(&self) -> VariationModel<String> {
        let mut locations: Vec<Location<String>> = vec![];
        for source in self.sources.iter() {
            let source_loc = self.normalize_location(&source.location);
            let mut loc = Location::new();
            for (ax, iter_l) in self.axes.iter().zip(source_loc.0.iter()) {
                loc.insert(ax.tag.clone(), *iter_l);
            }
            locations.push(loc);
        }
        VariationModel::new(locations, self.axes.iter().map(|x| x.tag.clone()).collect())
    }
    fn normalize_location(&self, loc: &[Dimension]) -> NormalizedLocation {
        let mut v: Vec<f32> = vec![];
        for (ax, l) in self.axes.iter().zip(loc.iter()) {
            v.push(ax.normalize_designspace_value(l.xvalue.unwrap_or(0.0)));
        }
        NormalizedLocation(v)
    }
}

trait BetterSource {
    fn ufo(&self, designspace_filename: &Path) -> Result<norad::Font, norad::error::FontLoadError>;
}

impl BetterSource for Source {
    fn ufo(&self, designspace_filename: &Path) -> Result<norad::Font, norad::error::FontLoadError> {
        norad::Font::load(designspace_filename.parent().unwrap().join(&self.filename))
    }
}

mod kerning;
use crate::kerning::interpolate_kerning;

/// Interpolate an instance UFO in a designspace
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    /// Increase logging
    #[clap(short, long, parse(from_occurrences))]
    verbose: usize,

    /// Output UFO
    #[clap(short, long)]
    output: Option<String>,

    /// Input designspace
    input: String,

    /// List of space separated locations. A location consists of the tag of a variation axis, followed by '=' and a number
    loc_args: Vec<String>,
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

    let unnormalized_target_location = parse_locargs(&args.loc_args);
    let ds: DesignSpaceDocument =
        DesignSpaceDocument::load(&args.input).expect("Couldn't read designspace file");
    // Ensure locations are sensible
    let mut ok = true;
    for axis in &ds.axes {
        let tag: &str = &axis.tag;
        if let Some(&location) = unnormalized_target_location.get(tag) {
            if axis.minimum.is_some() && Some(location) < axis.minimum {
                log::warn!(
                    "Location {} is less than minimum {} on axis {}",
                    location,
                    axis.minimum.unwrap(),
                    axis.tag
                );
            }
            if axis.maximum.is_some() && Some(location) > axis.maximum {
                log::warn!(
                    "Location {} is more than maximum {} on axis {}",
                    location,
                    axis.maximum.unwrap(),
                    axis.tag
                );
            }
        } else {
            log::error!("Tag {} needs a location", axis.tag);
            ok = false;
        }
    }
    if !ok {
        std::process::exit(1);
    }
    let target_location = ds
        .axes
        .iter()
        .map(|ax| {
            let this_axis_loc = unnormalized_target_location.get(&ax.tag).unwrap();
            (
                ax.tag.to_string(),
                ax.normalize_userspace_value(*this_axis_loc),
            )
        })
        .collect();

    let mut source_locations: Vec<BTreeMap<&str, f32>> = Vec::new();
    for source in &ds.sources {
        let this_loc: BTreeMap<&str, f32> = ds
            .axes
            .iter()
            .map(|x| x.tag.as_str())
            .zip(ds.location_to_tuple(&source.location))
            .collect();

        source_locations.push(this_loc);
    }
    let source_ufos: Vec<norad::Font> = ds
        .sources
        .par_iter()
        .map(|s| s.ufo(Path::new(&args.input)).expect("Couldn't load UFO"))
        .collect();
    let default_master = ds.default_master().expect("Can't find default master");
    let mut output_ufo = default_master
        .ufo(Path::new(&args.input))
        .expect("Couldn't load UFO");
    log::info!("Source locations: {:?}", source_locations);
    log::info!("Target location: {:?}", target_location);
    let vm = ds.variation_model();

    for g in output_ufo.default_layer_mut().iter_mut() {
        let glyph_name = &g.name();
        let others: Vec<Option<&Glyph>> = source_ufos
            .iter()
            .map(|u| u.default_layer().get_glyph(glyph_name))
            .collect();
        interpolate_contours(g, &others, &vm, &target_location);
        interpolate_anchors(g, &others, &vm, &target_location);
        interpolate_components(g, &others, &vm, &target_location);
        interpolate_advance_widths(g, &others, &vm, &target_location);
    }

    interpolate_kerning(&mut output_ufo, &source_ufos, &vm, &target_location);

    if let Some(p) = args.output {
        println!("Saved on {}", p);
        output_ufo.save(p).expect("Couldn't save UFO");
    } else {
        let location_str: Vec<String> = unnormalized_target_location
            .iter()
            .map(|(tag, val)| format!("{}{}", tag, val))
            .collect();
        let joined = location_str.join("-");
        let output_name = args
            .input
            .replace(".designspace", &format!("-{}.ufo", &joined));
        println!("Saved on {}", output_name);
        output_ufo.save(output_name).expect("Couldn't save UFO");
    }
}

fn str_to_fixed_to_float(s: &str) -> f32 {
    ot_round(str::parse::<f32>(s).unwrap()) as f32
}

fn parse_locargs(locargs: &[String]) -> BTreeMap<String, f32> {
    let mut res = BTreeMap::new();
    let matcher = Regex::new(r"^(\w{1,4})=([\d\.]+)$").unwrap();
    for limit_string in locargs {
        let captures = matcher
            .captures(limit_string)
            .expect("Couldn't parse location format");
        let tag = captures.get(1).unwrap().as_str();
        let location: f32 = str_to_fixed_to_float(captures.get(2).unwrap().as_str());
        res.insert(tag.to_string(), location);
    }
    res
}

trait QuickGetSet {
    fn get_contour_numbers(&self) -> ndarray::Array1<f32>;
    fn add_contour_numbers(
        &mut self,
        contours: &[Option<ndarray::Array1<f32>>],
        model: &VariationModel<String>,
        location: &Location<String>,
    );
    fn get_anchor_numbers(&self) -> ndarray::Array1<f32>;
    fn add_anchor_numbers(
        &mut self,
        contours: &[Option<ndarray::Array1<f32>>],
        model: &VariationModel<String>,
        location: &Location<String>,
    );
    fn get_component_numbers(&self) -> ndarray::Array1<f32>;
    fn add_component_numbers(
        &mut self,
        contours: &[Option<ndarray::Array1<f32>>],
        model: &VariationModel<String>,
        location: &Location<String>,
    );
}

fn interpolate(
    numbers: &[Option<ndarray::Array1<f32>>],
    model: &VariationModel<String>,
    location: &Location<String>,
) -> Vec<f32> {
    // log::debug!("Interpolating {:?} at {:?}", numbers, location);

    let locations = &model.original_locations;
    let mut vals: Vec<DVector<f64>> = vec![];
    let axis_count = location.len();
    let mut centers: Vec<DVector<f64>> = vec![];
    for (maybe_number, master_location) in numbers.iter().zip(locations.iter()) {
        if let Some(number) = maybe_number {
            let this_location: DVector<f64> = DVector::from_fn(axis_count, |i, _| {
                let axis = model
                    .axis_order
                    .get(i)
                    .expect("Location had wrong axis count?");
                let val = master_location.get(axis).expect("Axis not found?!");
                *val as f64
            });
            centers.push(this_location);
            let this_val_vec = number.to_vec().iter().map(|x| *x as f64).collect();
            let this_val = DVector::from_vec(this_val_vec);
            vals.push(this_val);
        }
    }
    let scatter = Scatter::create(centers, vals, rbf_interp::Basis::PolyHarmonic(1), 2);

    let coords = DVector::from_fn(axis_count, |i, _| {
        let axis = model
            .axis_order
            .get(i)
            .expect("Location had wrong axis count?");
        let val = location.get(axis).expect("Axis not found?!");
        *val as f64
    });
    let interpolated_numbers = scatter
        .eval(coords)
        .as_slice()
        .iter()
        .map(|x| *x as f32)
        .collect();
    // log::debug!("Interpolated value = {:?}", interpolated_numbers);

    interpolated_numbers
    // let deltas_and_supports = model.get_deltas_and_supports(numbers);
    // let (deltas, support_scalars): (Vec<ndarray::Array1<f32>>, Vec<f32>) = deltas_and_supports
    //     .into_iter()
    //     .map(|(x, y)| (x, support_scalar(location, &y)))
    //     .unzip();

    // let interpolated_numbers = model
    //     .interpolate_from_deltas_and_scalars(&deltas, &support_scalars)
    //     .expect("Couldn't interpolate");

    // interpolated_numbers.to_vec()
}

impl QuickGetSet for Glyph {
    fn get_contour_numbers(&self) -> ndarray::Array1<f32> {
        let mut v = vec![];
        for contour in &self.contours {
            for p in &contour.points {
                v.push(p.x as f32);
                v.push(p.y as f32);
            }
        }
        let len = v.len();
        ndarray::Array1::from_shape_vec(len, v).unwrap()
    }

    fn add_contour_numbers(
        &mut self,
        numbers: &[Option<ndarray::Array1<f32>>],
        model: &VariationModel<String>,
        location: &Location<String>,
    ) {
        let v = interpolate(numbers, model, location);
        let mut i = 0;
        for contour in self.contours.iter_mut() {
            for p in contour.points.iter_mut() {
                p.x += (*v.get(i).unwrap()) as f64;
                i += 1;
                p.y += (*v.get(i).unwrap()) as f64;
                i += 1;
            }
        }
    }

    fn get_anchor_numbers(&self) -> ndarray::Array1<f32> {
        let mut v = vec![];
        for anchor in &self.anchors {
            v.push(anchor.x as f32);
            v.push(anchor.y as f32);
        }
        let len = v.len();
        ndarray::Array1::from_shape_vec(len, v).unwrap()
    }

    fn add_anchor_numbers(
        &mut self,
        numbers: &[Option<ndarray::Array1<f32>>],
        model: &VariationModel<String>,
        location: &Location<String>,
    ) {
        let v = interpolate(numbers, model, location);
        let mut i = 0;
        for anchor in self.anchors.iter_mut() {
            anchor.x += (*v.get(i).unwrap()) as f64;
            i += 1;
            anchor.y += (*v.get(i).unwrap()) as f64;
            i += 1;
        }
    }

    fn get_component_numbers(&self) -> ndarray::Array1<f32> {
        let mut v = vec![];
        for component in &self.components {
            v.push(component.transform.x_scale as f32);
            v.push(component.transform.xy_scale as f32);
            v.push(component.transform.yx_scale as f32);
            v.push(component.transform.y_scale as f32);
            v.push(component.transform.x_offset as f32);
            v.push(component.transform.y_offset as f32);
        }
        let len = v.len();
        ndarray::Array1::from_shape_vec(len, v).unwrap()
    }

    fn add_component_numbers(
        &mut self,
        numbers: &[Option<ndarray::Array1<f32>>],
        model: &VariationModel<String>,
        location: &Location<String>,
    ) {
        let v = interpolate(numbers, model, location);
        let mut i = 0;
        for component in self.components.iter_mut() {
            component.transform.x_scale += (*v.get(i).unwrap()) as f64;
            i += 1;
            component.transform.xy_scale += (*v.get(i).unwrap()) as f64;
            i += 1;
            component.transform.yx_scale += (*v.get(i).unwrap()) as f64;
            i += 1;
            component.transform.y_scale += (*v.get(i).unwrap()) as f64;
            i += 1;
            component.transform.x_offset += (*v.get(i).unwrap()) as f64;
            i += 1;
            component.transform.y_offset += (*v.get(i).unwrap()) as f64;
            i += 1;
        }
    }
}

fn interpolate_contours(
    output: &mut Glyph,
    masters: &[Option<&Glyph>],
    model: &VariationModel<String>,
    location: &Location<String>,
) {
    let default_numbers: ndarray::Array1<f32> = output.get_contour_numbers();
    let contours: Vec<Option<ndarray::Array1<f32>>> = masters
        .iter()
        .map(|m| {
            m.and_then(|g| {
                let nums: ndarray::Array1<f32> = g.get_contour_numbers();
                if nums.shape() == default_numbers.shape() {
                    Some(g.get_contour_numbers() - default_numbers.clone())
                } else {
                    log::warn!("Incompatible masters in {}", g.name());
                    None
                }
            })
        })
        .collect();
    output.add_contour_numbers(&contours, model, location);
}

fn interpolate_anchors(
    output: &mut Glyph,
    masters: &[Option<&Glyph>],
    model: &VariationModel<String>,
    location: &Location<String>,
) {
    let default_numbers: ndarray::Array1<f32> = output.get_anchor_numbers();
    let anchors: Vec<Option<ndarray::Array1<f32>>> = masters
        .iter()
        .map(|m| {
            m.and_then(|g| {
                let nums: ndarray::Array1<f32> = g.get_anchor_numbers();
                if nums.shape() == default_numbers.shape() {
                    Some(g.get_anchor_numbers() - default_numbers.clone())
                } else {
                    log::warn!("Incompatible masters in {}", g.name());
                    None
                }
            })
        })
        .collect();
    output.add_anchor_numbers(&anchors, model, location);
}

fn interpolate_components(
    output: &mut Glyph,
    masters: &[Option<&Glyph>],
    model: &VariationModel<String>,
    location: &Location<String>,
) {
    let default_numbers: ndarray::Array1<f32> = output.get_component_numbers();
    let components: Vec<Option<ndarray::Array1<f32>>> = masters
        .iter()
        .map(|m| {
            m.and_then(|g| {
                let nums: ndarray::Array1<f32> = g.get_component_numbers();
                if nums.shape() == default_numbers.shape() {
                    Some(g.get_component_numbers() - default_numbers.clone())
                } else {
                    log::warn!("Incompatible masters in {}", g.name());
                    None
                }
            })
        })
        .collect();
    output.add_component_numbers(&components, model, location);
}

fn interpolate_advance_widths(
    output: &mut Glyph,
    masters: &[Option<&Glyph>],
    model: &VariationModel<String>,
    location: &Location<String>,
) {
    let default_advance: f64 = output.width;
    let advances: Vec<Option<ndarray::Array1<f32>>> = masters
        .iter()
        .map(|m| m.map(|g| ndarray::Array1::from_elem(1, (g.width - default_advance) as f32)))
        .collect();
    let interpolated_width = interpolate(&advances, model, location);
    output.width += *interpolated_width.first().unwrap() as f64;
}
