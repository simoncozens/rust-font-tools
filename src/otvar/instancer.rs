#![allow(missing_docs)]
use crate::font::{Font, Table};
use crate::tables::{
    avar::{self, SegmentMap},
    fvar, glyf,
    gvar::{self, Coords, DeltaSet, GlyphVariationData},
};
//use crate::font::Table;
//use crate::fvar;
//use crate::glyf;
//use crate::gvar;
//use crate::gvar::{Coords, DeltaSet, GlyphVariationData};
use super::support_scalar;
use crate::types::Tag;
use otspec::types::*;
use std::collections::BTreeMap;

type Location = BTreeMap<Tag, f32>;

#[derive(Debug, Clone, PartialEq)]
pub struct AxisRange {
    minimum: f32,
    maximum: f32,
}

impl AxisRange {
    pub fn new(minimum: f32, maximum: f32) -> Self {
        if maximum < minimum {
            panic!("Range minimum must be more than maximum")
        }
        AxisRange { minimum, maximum }
    }
}
#[derive(Debug, Clone, PartialEq)]
struct NormalizedAxisRange {
    minimum: f32,
    maximum: f32,
}

#[derive(Debug, Clone)]
enum NormalizedAxisLimit {
    Full(f32),
    Partial(NormalizedAxisRange),
}

#[derive(Debug)]
pub struct NormalizedAxisLimits(BTreeMap<Tag, NormalizedAxisLimit>);
type FullNormalizedAxisLimits = Location;
type PartialNormalizedAxisLimits = BTreeMap<Tag, (f32, f32)>;

impl NormalizedAxisLimits {
    pub fn split_up(&self) -> (FullNormalizedAxisLimits, PartialNormalizedAxisLimits) {
        let mut full: FullNormalizedAxisLimits = BTreeMap::new();
        let mut partial: PartialNormalizedAxisLimits = BTreeMap::new();
        for (&tag, limit) in &self.0 {
            match limit {
                NormalizedAxisLimit::Full(loc) => {
                    full.insert(tag, *loc);
                }
                NormalizedAxisLimit::Partial(NormalizedAxisRange { minimum, maximum }) => {
                    partial.insert(tag, (*minimum, *maximum));
                }
            };
        }
        (full, partial)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum UserAxisLimit {
    Full(f32),
    Partial(AxisRange),
    Drop,
}

#[derive(Debug)]
pub struct UserAxisLimits(pub BTreeMap<Tag, UserAxisLimit>);
type FullUserAxisLimits = Location;
type PartialUserAxisLimits = BTreeMap<Tag, (f32, f32)>;

impl UserAxisLimits {
    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn split_up(&self) -> (FullUserAxisLimits, PartialUserAxisLimits) {
        let mut full: FullUserAxisLimits = BTreeMap::new();
        let mut partial: PartialUserAxisLimits = BTreeMap::new();
        for (&tag, limit) in &self.0 {
            match limit {
                UserAxisLimit::Full(loc) => {
                    full.insert(tag, *loc);
                }
                UserAxisLimit::Partial(AxisRange { minimum, maximum }) => {
                    partial.insert(tag, (*minimum, *maximum));
                }
                UserAxisLimit::Drop => {}
            };
        }
        (full, partial)
    }
}

// #[derive(Debug)]
// enum OverlapMode {
//     KeepAndDontSetFlags,
//     KeepAndSetFlags,
//     Remove,
//     RemoveAndIgnoreErrors,
// }

fn instantiate_gvar_data(
    variations: &mut GlyphVariationData,
    axis_tags: &[Tag],
    axis_limits: &NormalizedAxisLimits,
) -> Coords {
    let mut new_variations = variations.clone();
    let (pinned, axis_ranges): (FullNormalizedAxisLimits, PartialNormalizedAxisLimits) =
        axis_limits.split_up();

    if !pinned.is_empty() {
        new_variations = pin_tuple_variation_axes(&mut new_variations, &pinned, axis_tags)
    }
    if !axis_ranges.is_empty() {
        new_variations = limit_tuple_variation_axis_ranges(new_variations, axis_ranges)
    }
    let mut merged_variations: BTreeMap<Vec<(Tag, F2DOT14, F2DOT14, F2DOT14)>, DeltaSet> =
        BTreeMap::new();
    for deltaset in &new_variations.deltasets {
        // We don't need to IUP here as Python does, because we're working on "cooked" delta sets
        let mut tent = vec![];
        for (ix, ax) in axis_tags.iter().enumerate() {
            let peak = deltaset.peak.get(ix).expect("Where'd my axis go?");
            if pinned.contains_key(ax) {
                continue;
            }
            let start = deltaset.start.get(ix).expect("Where'd my axis go?");
            let end = deltaset.end.get(ix).expect("Where'd my axis go?");
            tent.push((*ax, F2DOT14(*start), F2DOT14(*peak), F2DOT14(*end)))
        }

        if let Some(var) = merged_variations.get(&tent) {
            merged_variations.insert(tent, var.combine(deltaset));
        } else {
            merged_variations.insert(tent, deltaset.clone());
        }
    }

    println!("Merged variations: {:?}", merged_variations);
    // XXX - wait - axis_tags has the old set of axes...
    let default_tent: Vec<(Tag, F2DOT14, F2DOT14, F2DOT14)> = vec![];
    let default_var = merged_variations.remove(&default_tent);

    // Our deltas are i32, that seems bad for this?

    // for v in merged_variations.values_mut() {
    // v.round_deltas();
    // }

    variations.deltasets = merged_variations.values().cloned().collect();
    if let Some(default) = default_var {
        default.deltas
    } else {
        vec![]
    }
}

fn pin_tuple_variation_axes(
    variations: &mut GlyphVariationData,
    location: &FullNormalizedAxisLimits,
    axis_tags: &[Tag],
) -> GlyphVariationData {
    let mut new_deltas: Vec<gvar::DeltaSet> = vec![];
    for var in variations.deltasets.iter_mut() {
        println!("Deltaset : {:?}", var);

        // Deltaset is a set of tuples using the font's existing axes
        let mut support = BTreeMap::new();
        for tag in location.keys() {
            let index = axis_tags
                .iter()
                .position(|t| t == tag)
                .expect("Axis in location wasn't in font");
            let support_for_this_axis = (var.start[index], var.peak[index], var.end[index]);
            println!("Support for {}: {:?}", tag, support_for_this_axis);
            support.insert(*tag, support_for_this_axis);
        }
        let scalar = support_scalar(location, &support);
        println!("Support scalar for {:?}: {:?}", location, scalar);
        if scalar == 0.0 {
            continue;
        }
        var.scale_deltas(scalar);
        new_deltas.push(var.clone());
    }
    println!("Pinned deltas: {:?}", new_deltas);
    GlyphVariationData {
        deltasets: new_deltas,
    }
}

fn limit_tuple_variation_axis_ranges(
    _tvs: GlyphVariationData,
    _axis_ranges: PartialNormalizedAxisLimits,
) -> GlyphVariationData {
    unimplemented!()
}

fn sanity_check(font: &Font) {
    if !font.tables.contains_key(b"fvar") {
        panic!("Missing required table fvar")
    }
    if font.tables.contains_key(b"CFF2") {
        panic!("I don't speak CFF")
    }
}

fn instantiate_gvar_glyph(
    ix: usize,
    axis_tags: &[Tag],
    glyf: &mut glyf::glyf,
    gvar: &mut gvar::gvar,
    axis_limits: &NormalizedAxisLimits,
) {
    let glyph = glyf.glyphs.get_mut(ix).unwrap();
    println!("Handling glyph {:?}", ix);

    if let Some(var) = gvar.variations.get_mut(ix).unwrap() {
        let mut deltas = instantiate_gvar_data(var, axis_tags, axis_limits).into_iter();
        println!("New deltas: {:?}", deltas);
        for contour in glyph.contours.iter_mut() {
            for point in contour.iter_mut() {
                let delta = deltas.next().expect("Not enough deltas for glyph");
                point.x += delta.0;
                point.y += delta.1;
            }
        }
        // XXX phantom points
        if var.deltasets.is_empty() {
            log::info!("No delta sets left, dropping variation");
            gvar.variations[ix] = None;
        }
    }
}

fn instantiate_gvar(font: &mut Font, axis_limits: &NormalizedAxisLimits) {
    log::info!("Instantiating gvar/glyf table");
    let axis_tags: Vec<Tag> = font
        .tables
        .get(b"fvar")
        .unwrap()
        .fvar_unchecked()
        .axes
        .iter()
        .map(|x| x.axisTag)
        .collect();
    let mut glyf: glyf::glyf = font.tables.get(b"glyf").unwrap().glyf_unchecked().clone();
    let gvar_table = font.get_table(tag!("gvar")).unwrap().unwrap();

    if let Table::Gvar(gvar) = gvar_table {
        for gid in 0..glyf.glyphs.len() {
            instantiate_gvar_glyph(gid, &axis_tags, &mut glyf, gvar, axis_limits)
        }
        if !gvar.variations.iter().any(|x| x.is_some()) {
            log::info!("Dropping gvar table");
            font.tables.remove(b"gvar");
        }
    }
    font.tables.insert(tag!("glyf"), Table::Glyf(glyf));
}

fn instantiate_avar(font: &mut Font, axis_limits: &UserAxisLimits) {
    let (location, _axis_ranges): (FullUserAxisLimits, PartialUserAxisLimits) =
        axis_limits.split_up();
    let (_, normalized_ranges) = normalize_axis_limits(font, axis_limits, false).split_up();

    // Drop avar if we instantiate everything
    let fvar_table = font.tables.get(b"fvar").unwrap();
    let mut axis_tags = vec![];
    if let Table::Fvar(fvar) = fvar_table {
        if fvar
            .axes
            .iter()
            .all(|ax| location.contains_key(&ax.axisTag))
        {
            log::info!("Dropping avar table");
            font.tables.remove(b"avar");
            return;
        }
        for ax in &fvar.axes {
            axis_tags.push(ax.axisTag)
        }
    }

    if let Table::Avar(avar_table) = font.get_table(tag!("avar")).unwrap().unwrap() {
        // We are doing avar first, so the fvar table contains the full set of axes.

        let mut segments_map: BTreeMap<Tag, SegmentMap> = axis_tags
            .iter()
            .zip(avar_table.axisSegmentMaps.iter())
            .map(|(&tag, seg)| (tag, seg.clone()))
            .collect();
        for pinned in location.keys() {
            segments_map.remove(pinned);
            axis_tags.retain(|tag| tag != pinned);
        }

        let mut new_segments: BTreeMap<Tag, SegmentMap> = BTreeMap::new();
        for (axis_tag, segment) in segments_map {
            if !segment.is_valid() {
                continue;
            }
            if let Some(&(minimum, maximum)) = normalized_ranges.get(&axis_tag) {
                let mapped_min = F2DOT14::round(segment.piecewise_linear_map(minimum));
                let mapped_max = F2DOT14::round(segment.piecewise_linear_map(maximum));
                let mut new_mapping: Vec<(f32, f32)> = vec![];
                for avm in &segment.axisValueMaps {
                    let (mut from_coord, mut to_coord) = (avm.fromCoordinate, avm.toCoordinate);
                    if from_coord < 0.0 {
                        if minimum == 0.0 || from_coord < minimum {
                            continue;
                        } else {
                            from_coord /= minimum.abs();
                        }
                    } else if from_coord > 0.0 {
                        if maximum == 0.0 || from_coord > maximum {
                            continue;
                        } else {
                            from_coord /= maximum;
                        }
                    }
                    if to_coord < 0.0 {
                        assert!(mapped_min != 0.0);
                        assert!(to_coord >= mapped_min);
                        to_coord /= mapped_min.abs()
                    } else if to_coord > 0.0 {
                        assert!(mapped_max != 0.0);
                        assert!(to_coord <= mapped_max);
                        to_coord /= mapped_max.abs()
                    }
                    from_coord = F2DOT14::round(from_coord);
                    to_coord = F2DOT14::round(to_coord);
                    new_mapping.push((from_coord, to_coord));
                }
                new_segments.insert(axis_tag, avar::SegmentMap::new(new_mapping));
            } else {
                new_segments.insert(axis_tag, segment);
            }
        }
        // Put back the segments map into the avar table, in the right order.
        avar_table.axisSegmentMaps = axis_tags
            .iter()
            .map(|tag| new_segments.get(tag).unwrap().clone())
            .collect();
    }
}

fn is_instance_within_axis_ranges(loc: &Location, axis_ranges: &PartialUserAxisLimits) -> bool {
    for (tag, coord) in loc {
        if let Some((min, max)) = axis_ranges.get(tag) {
            if coord < min || coord > max {
                return false;
            }
        }
    }
    true
}

fn instantiate_fvar(font: &mut Font, axis_limits: &UserAxisLimits) {
    let (location, axis_ranges): (FullUserAxisLimits, PartialUserAxisLimits) =
        axis_limits.split_up();

    let fvar_table = font.get_table(tag!("fvar")).unwrap().unwrap();
    if let Table::Fvar(fvar) = fvar_table {
        if fvar
            .axes
            .iter()
            .all(|ax| location.contains_key(&ax.axisTag))
        {
            log::info!("Dropping fvar table");
            font.tables.remove(b"fvar");
            return;
        }
    }

    log::info!("Instantiating fvar table");
    if let Table::Fvar(fvar) = fvar_table {
        let mut new_axes = vec![];
        for axis in fvar.axes.iter_mut() {
            let axis_tag = axis.axisTag;
            if location.contains_key(&axis_tag) {
                continue;
            }
            if let Some(&(minimum, maximum)) = axis_ranges.get(&axis_tag) {
                axis.minValue = minimum;
                axis.maxValue = maximum;
            }
            new_axes.push(axis.clone());
        }

        let mut new_instances = vec![];
        for instance in fvar.instances.iter_mut() {
            let mut keep = true;
            // We need to convert this instance's tuple into a location
            let mut instance_location: Location = fvar
                .axes
                .iter()
                .zip(instance.coordinates.iter())
                .map(|(ax, &l)| (ax.axisTag, l))
                .collect();

            // only keep NamedInstances whose coordinates == pinned axis location
            for (loc_tag, loc_value) in location.iter() {
                if (instance_location
                    .get(loc_tag)
                    .expect("Tag mismatch in instance table")
                    - loc_value)
                    .abs()
                    > f32::EPSILON
                {
                    keep = false;
                    break;
                }
                // Delete the pinned tag from our mapping
                instance_location.remove(loc_tag);
            }

            if !keep {
                continue;
            }
            // XXX
            if !is_instance_within_axis_ranges(&instance_location, &axis_ranges) {
                continue;
            }
            // Now set the location from the *new* axes list
            let new_tuple: Tuple = new_axes
                .iter()
                .map(|x| instance_location.get(&x.axisTag).unwrap())
                .copied()
                .collect();
            instance.coordinates = new_tuple;
            new_instances.push(instance.clone());
        }

        fvar.axes = new_axes;
        fvar.instances = new_instances;
    }
}

#[allow(non_snake_case)]
fn instantiate_STAT(font: &mut Font, axis_limits: &UserAxisLimits) {
    let stat_table = font.get_table(tag!("STAT")).unwrap().unwrap();
    if let Table::STAT(stat) = stat_table {
        if stat.design_axes.is_empty() || stat.axis_values.is_empty() {
            return;
        }
        log::info!("Instantiating STAT table");
        let (location, axis_ranges): (FullUserAxisLimits, PartialUserAxisLimits) =
            axis_limits.split_up();

        let is_axis_value_outside_limits = |tag: &Tag, value: f32| {
            if let Some(&f) = location.get(tag) {
                if (value - f).abs() > f32::EPSILON {
                    return true;
                }
            }
            if let Some(&(minimum, maximum)) = axis_ranges.get(tag) {
                if value < minimum || value > maximum {
                    return true;
                }
            }
            false
        };

        let mut new_axis_value_tables: Vec<crate::tables::STAT::AxisValue> = vec![];
        let av = stat.axis_values.clone();
        for axis_value in av {
            if let Some(nominal) = axis_value.nominal_value {
                let axis_tag = stat
                    .design_axes
                    .get(axis_value.axis_index.unwrap() as usize)
                    .unwrap()
                    .axisTag;
                if is_axis_value_outside_limits(&axis_tag, nominal) {
                    continue;
                }
            }
            if let Some(locations) = &axis_value.locations {
                let mut drop_axis_table = false;
                for (&axis_index, &axis_value) in locations {
                    let axis_tag = stat.design_axes.get(axis_index as usize).unwrap().axisTag;
                    if is_axis_value_outside_limits(&axis_tag, axis_value) {
                        drop_axis_table = true;
                        break;
                    }
                }
                if drop_axis_table {
                    continue;
                }
            }
            new_axis_value_tables.push(axis_value);
        }
        stat.axis_values = new_axis_value_tables;
    }
}

fn set_mac_overlap_flags(glyf: &mut glyf::glyf) {
    for mut g in glyf.glyphs.iter_mut() {
        g.overlap = true;
    }
}

fn populate_axis_defaults(font: &mut Font, limits: UserAxisLimits) -> UserAxisLimits {
    let mut limits = limits;
    let fvar: &fvar::fvar = font
        .get_table(tag!("fvar"))
        .unwrap()
        .unwrap()
        .fvar_unchecked();
    let defaults: Location = fvar
        .axes
        .iter()
        .map(|ax| (ax.axisTag, ax.defaultValue))
        .collect();
    for (k, v) in limits
        .0
        .iter_mut()
        .filter(|(_k, v)| matches!(v, UserAxisLimit::Drop))
    {
        *v = UserAxisLimit::Full(*defaults.get(k).expect("Unknown axis"));
    }
    limits
}

fn normalize(value: f32, triple: (f32, f32, f32), avar_segment: Option<&SegmentMap>) -> f32 {
    let (minv, _default, maxv) = triple;
    let mut value = (value.clamp(minv, maxv) - minv) / (maxv - minv);
    if let Some(map) = avar_segment {
        value = map.piecewise_linear_map(value);
    }
    F2DOT14::round(value)
}

fn normalize_axis_limits(
    font: &mut Font,
    limits: &UserAxisLimits,
    use_avar: bool,
) -> NormalizedAxisLimits {
    let fvar: &fvar::fvar = font
        .get_table(tag!("fvar"))
        .unwrap()
        .unwrap()
        .fvar_unchecked();
    let all_axes: Vec<Tag> = fvar.axes.iter().map(|x| x.axisTag).collect();
    for ax in limits.0.keys() {
        if !all_axes.contains(ax) {
            panic!("Can't limit {} - axis not in font", ax,)
        }
    }
    let axes: BTreeMap<Tag, (f32, f32, f32)> = fvar
        .axes
        .iter()
        .filter(|ax| limits.0.contains_key(&ax.axisTag))
        .map(|ax| (ax.axisTag, (ax.minValue, ax.defaultValue, ax.maxValue)))
        .collect();
    let avar_segs: BTreeMap<Tag, &SegmentMap> = if use_avar && font.tables.contains_key(b"avar") {
        let avar: &avar::avar = font
            .get_table(tag!("avar"))
            .unwrap()
            .unwrap()
            .avar_unchecked();
        all_axes
            .iter()
            .zip(avar.axisSegmentMaps.iter())
            .map(|(a, b)| (*a, b))
            .collect()
    } else {
        BTreeMap::new()
    };

    for (tag, &(_, default, _)) in &axes {
        if let Some(&UserAxisLimit::Partial(AxisRange { minimum, maximum })) = limits.0.get(tag) {
            if minimum > default || maximum < default {
                panic!(
                    "Unsupported range {}:={}:{}; default position is {}",
                    tag, minimum, maximum, default
                )
            }
        }
    }

    let mut normalized_limits = BTreeMap::new();
    for (tag, tuple) in axes {
        let avar_mapping = avar_segs.get(&tag).copied();
        let value = limits.0.get(&tag).unwrap();
        match value {
            UserAxisLimit::Partial(AxisRange { minimum, maximum }) => {
                normalized_limits.insert(
                    tag,
                    NormalizedAxisLimit::Partial(NormalizedAxisRange {
                        minimum: normalize(*minimum, tuple, avar_mapping),
                        maximum: normalize(*maximum, tuple, avar_mapping),
                    }),
                );
            }
            UserAxisLimit::Full(v) => {
                normalized_limits.insert(
                    tag,
                    NormalizedAxisLimit::Full(normalize(*v, tuple, avar_mapping)),
                );
            }
            _ => {
                panic!("Can't happen")
            }
        }
    }
    NormalizedAxisLimits(normalized_limits)
}

pub fn instantiate_variable_font(font: &mut Font, limits: UserAxisLimits) -> bool {
    sanity_check(font);
    let limits = populate_axis_defaults(font, limits);
    log::debug!("Full limits: {:?}", limits);
    let normalized_limits = normalize_axis_limits(font, &limits, true);
    log::debug!("Normalized limits: {:?}", normalized_limits);
    font.get_table(tag!("fvar")).expect("Can't open fvar");
    font.get_table(tag!("glyf")).expect("Can't open glyf");
    font.get_table(tag!("gvar")).expect("Can't open gvar");
    // update name table (can't)
    if font.tables.contains_key(b"gvar") {
        // Deserialize what we need
        instantiate_gvar(font, &normalized_limits);
    }
    if font.tables.contains_key(b"cvar") {
        // instantiate_cvar(font, normalized_limits);
    }
    if font.tables.contains_key(b"MVAR") {
        // instantiate_MVAR(font, normalized_limits);
    }
    if font.tables.contains_key(b"HVAR") {
        // instantiate_HVAR(font, normalized_limits);
    }
    if font.tables.contains_key(b"VVAR") {
        // instantiate_VVAR(font, normalized_limits);
    }
    // instantiate_otl(font, normalized_limits);
    // instantiate_feature_variations(font, normalized_limits);
    if font.tables.contains_key(b"avar") {
        font.get_table(tag!("avar")).expect("Can't open avar");
        instantiate_avar(font, &limits);
    }
    if font.tables.contains_key(b"STAT") {
        instantiate_STAT(font, &limits);
    }
    instantiate_fvar(font, &limits);
    if !font.tables.contains_key(b"fvar") && !font.tables.contains_key(b"glyf") {
        // set overlap flags
        if let Table::Glyf(glyf) = font.get_table(tag!("glyf")).unwrap().unwrap() {
            set_mac_overlap_flags(glyf)
        }
    }
    // let (full, _) = limits.split_up();
    // set_default_weight_width_slant(font, full);
    true
}
