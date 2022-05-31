use crate::utils::is_all_same;
use fonttools::otvar::VariationModel;
use fonttools::tables::glyf;
use fonttools::tables::gvar::{DeltaSet, GlyphVariationData};
use kurbo::{cubics_to_quadratic_splines, BezPath, CubicBez, PathEl, PathSeg};
use otmath::ot_round;
use otspec::utils::is_all_the_same;
use std::collections::BTreeMap;
use std::mem;
use unzip_n::unzip_n;

unzip_n!(3);

// "Once the data structures are laid out, the algorithms tend to fall
// into place, and the coding is comparatively easy."

type GlyphContour = Vec<Vec<glyf::Point>>;

#[derive(Default)]
struct GlyphForConversion<'a> {
    masters: Vec<Option<UnconvertedMaster<'a>>>,
    default_master_ix: usize,
    model: Option<&'a VariationModel<String>>,
    /// The glyph's name, for debugging purposes
    glif_name: &'a str,
}

impl<'a> GlyphForConversion<'a> {
    fn default_master(&self) -> &UnconvertedMaster<'a> {
        self.masters[self.default_master_ix].as_ref().unwrap()
    }

    fn convert(mut self) -> GlyphReadyToGo<'a> {
        assert!(
            self.masters.get(self.default_master_ix).is_some(),
            "Default master for {:} was not in master list",
            self.glif_name
        );
        assert!(
            self.masters[self.default_master_ix].is_some(),
            "Default master for {:} was in master list but was not present",
            self.glif_name
        );

        let mut result = GlyphReadyToGo {
            masters: vec![],
            default_master_ix: self.default_master_ix,
            model: self.model,
            glif_name: self.glif_name,
        };
        /* OK, we're doing the contours of a variable font. Some of the masters
        may be sparse, i.e. not containing a layer for this glyph. We will
        keep the indices around for when we have to filter them out. */
        let mut indexes_of_nonsparse_masters: Vec<usize> = vec![];
        let mut nonsparse_masters: Vec<&UnconvertedMaster<'a>> = vec![];
        let mut index_of_default_master_in_nonsparse_list: Option<usize> = None;
        // Check for path count compatibility
        let default_path_count = self.default_master().babelfont_contours.len();

        for (ix, m) in self.masters.iter_mut().enumerate() {
            if let Some(m) = m {
                indexes_of_nonsparse_masters.push(ix);
                nonsparse_masters.push(m);
                result.masters.push(Some(ConvertedMaster {
                    glyf_contours: vec![],
                    components: (*m.components).to_vec(),
                    width: m.width,
                }))
            } else {
                result.masters.push(None);
            }
            if ix == self.default_master_ix {
                index_of_default_master_in_nonsparse_list =
                    Some(indexes_of_nonsparse_masters.len() - 1);
            }
        }
        let path_lengths = nonsparse_masters.iter().map(|x| x.babelfont_contours.len());
        if !is_all_the_same(path_lengths) {
            let lengths: Vec<usize> = nonsparse_masters
                .iter()
                .map(|x| x.babelfont_contours.len())
                .collect();
            log::error!(
                "Incompatible path count in glyph {:}: {:?}",
                self.glif_name,
                lengths
            );
            return self.into_nonvariable().convert();
        }

        // Convert path by path
        for index in 0..default_path_count {
            let all_contours: Vec<&babelfont::Path> = nonsparse_masters
                .iter()
                .map(|x| *x.babelfont_contours.get(index).unwrap())
                .collect();
            let all_glyf_contours = babelfont_contours_to_glyf_contours(
                index,
                all_contours,
                index_of_default_master_in_nonsparse_list.unwrap(),
                self.glif_name,
            );
            for (finished_contour, &master_id) in all_glyf_contours
                .iter()
                .zip(indexes_of_nonsparse_masters.iter())
            {
                assert!(result.masters[master_id].is_some());
                result.masters[master_id]
                    .as_mut()
                    .unwrap()
                    .glyf_contours
                    .push(finished_contour.clone());
            }
        }
        result
    }

    /// Drop all variation masters
    fn into_nonvariable(mut self) -> Self {
        let default_master: Option<UnconvertedMaster> =
            mem::replace(&mut self.masters[self.default_master_ix], None);
        GlyphForConversion {
            masters: vec![default_master],
            default_master_ix: 0,
            model: None,
            glif_name: self.glif_name,
        }
    }
}

#[derive(Default, Debug)]
struct UnconvertedMaster<'a> {
    babelfont_contours: Vec<&'a babelfont::Path>,
    components: Vec<glyf::Component>,
    width: i32,
}

#[derive(Default)]
struct GlyphReadyToGo<'a> {
    masters: Vec<Option<ConvertedMaster>>,
    default_master_ix: usize,
    model: Option<&'a VariationModel<String>>,
    glif_name: &'a str,
}

impl<'a> GlyphReadyToGo<'a> {
    fn into_glyph(mut self) -> glyf::Glyph {
        // We just build a glyph from the default master
        // There are invariants around the fact that when we built this thing we
        // have a default layer.
        let l: Option<ConvertedMaster> =
            mem::replace(&mut self.masters[self.default_master_ix], None);
        l.unwrap().into_glyph()
    }
    fn variation_data(&self) -> Option<GlyphVariationData> {
        self.model?;
        let model = self.model.as_ref().unwrap();
        let mut deltasets: Vec<DeltaSet> = vec![];
        let all_coords: Vec<Option<ndarray::Array2<f32>>> = self
            .masters
            .iter()
            .map(|o| o.as_ref().map(|m| m.gvar_coords()))
            .collect();
        // The model takes Vec<T> T:Sub, and ndarray::Array2 implements Sub,
        // so we can just send the whole vec of ndarrays to the model and get
        // back our deltas.
        if !is_all_the_same(all_coords.iter().flatten().map(|x| x.shape())) {
            log::error!(
                "Incompatible gvar shapes for glyph {:} (fonticulus bug)",
                self.glif_name
            );
            return None;
        }
        let deltas_and_supports = model.get_deltas_and_supports(&all_coords);

        for (delta, support) in deltas_and_supports.iter() {
            // log::debug!(" Delta {:#?}", delta);
            // log::debug!(" Support {:#?}", support);
            if support.is_empty() {
                continue;
            }

            // Turn the ndarray back into a vec of tuples
            let deltas: Vec<(i16, i16)> = delta
                .mapv(|x| ot_round(x) as i16)
                .outer_iter()
                .map(|x| (x[0], x[1]))
                .collect();

            // The variation model gives us the tents for each deltaset
            let tuples = model
                .axis_order
                .iter()
                .map(|ax| support.get(ax).unwrap_or(&(0.0, 0.0, 0.0)))
                .copied();
            let (start, peak, end) = tuples.into_iter().unzip_n_vec();

            // And we're done
            deltasets.push(DeltaSet {
                peak,
                start,
                end,
                deltas,
            })
        }
        // log::debug!("Deltasets {:#?}", deltasets);
        Some(GlyphVariationData { deltasets })
    }
}

#[derive(Default)]
struct ConvertedMaster {
    glyf_contours: GlyphContour,
    components: Vec<glyf::Component>,
    width: i32,
}

impl<'a> ConvertedMaster {
    fn gvar_coords(&self) -> ndarray::Array2<f32> {
        let width = self.width;
        // Flatten all points (i.e. combine all contours together) in the glyph
        // and split up X and Y into separate arrays.
        let (mut master_x_coords, mut master_y_coords): (Vec<f32>, Vec<f32>) = self
            .glyf_contours
            .iter()
            .flatten()
            .map(|pt| (pt.x as f32, pt.y as f32))
            .unzip();

        for c in &self.components {
            let [_, _, _, _, x, y] = c.transformation.as_coeffs();
            master_x_coords.push(x as f32);
            master_y_coords.push(y as f32);
        }

        // Add the phantom points
        master_x_coords.extend(vec![0_f32, width as f32, 0.0, 0.0]);
        master_y_coords.extend(vec![0.0, 0.0, 0.0, 0.0]);

        // Concat the X-coordinates/Y-coordinates in preparation for being
        // reshaped into a 2d ndarray.
        let len = master_x_coords.len();
        master_x_coords.extend(master_y_coords);
        ndarray::Array2::from_shape_vec((2, len), master_x_coords)
            .unwrap()
            .reversed_axes()
    }

    fn into_glyph(self) -> glyf::Glyph {
        glyf::Glyph {
            xMin: 0,
            xMax: 0,
            yMin: 0,
            yMax: 0,
            contours: self.glyf_contours,
            instructions: vec![],
            components: self.components,
            overlap: false,
        }
    }
}

// We are going to be converting a set of masters representing a single glyph
// at different points in the designspace into a base `glyf` table entry plus
// `gvar` table information.
//
// We are being handed:
pub fn layers_to_glyph(
    // The index of the default master (this tells us which outline goes into
    // the `glyf` table),
    default_master: usize,
    // A mapping of glyph names to glyph IDs (for resolving components)
    mapping: &BTreeMap<String, u16>,
    // The set of masters
    masters: &[Option<&babelfont::Layer>],
    // A variation model, which tells us where all the masters live in the
    // design space
    model: Option<&VariationModel<String>>,
    // and the glyph's name, for debugging purposes
    glif_name: &str,
) -> (glyf::Glyph, Option<GlyphVariationData>) {
    let mut for_conversion = GlyphForConversion {
        masters: vec![],
        default_master_ix: default_master,
        model,
        glif_name,
    };

    for maybe_layer in masters {
        if let Some(layer) = maybe_layer {
            for_conversion.masters.push(Some(UnconvertedMaster {
                babelfont_contours: layer.paths().collect(),
                components: layer
                    .components()
                    .flat_map(|component| babelfont_component_to_glyf_component(component, mapping))
                    .collect(),
                width: layer.width,
            }));
        } else {
            for_conversion.masters.push(None);
        }
    }

    let result: GlyphReadyToGo = for_conversion.convert();
    let variation_data: Option<GlyphVariationData> = result.variation_data();
    (result.into_glyph(), variation_data)
}

fn babelfont_contours_to_glyf_contours(
    // Which path this is in the glyph (for error reporting)
    path_index: usize,

    // A (non-sparse) list of contours
    paths: Vec<&babelfont::Path>,

    // The index of the default master (used as the reference for curve construction)
    default_master: usize,

    // Which glyph this is (for error reporting)
    glif_name: &str,
) -> GlyphContour {
    // Let's first get them all to kurbo elements.
    let kurbo_paths: Vec<BezPath> = paths
        .iter()
        .map(|x| x.to_kurbo().expect("Bad contour construction"))
        .collect();

    // Ensure they are all the same size
    let lengths: Vec<usize> = kurbo_paths.iter().map(|x| x.elements().len()).collect();

    if !is_all_same(&lengths) {
        log::error!(
            "Incompatible contour {:} in glyph {:}: {:?}",
            path_index,
            glif_name,
            lengths
        );
        return vec![];
    }

    // XXX ensure they are all compatible, type-wise.

    // We're going to turn the list of cubic bezpaths into Vec<Point> expected by Glyf
    let mut quadratic_paths: Vec<Vec<glyf::Point>> = paths.iter().map(|_| vec![]).collect();

    let default_elements: &[PathEl] = kurbo_paths[default_master].elements();
    for (el_ix, el) in default_elements.iter().enumerate() {
        match el {
            PathEl::CurveTo(_, _, _) => {
                // Convert all the cubics to quadratics in one go, across masters
                let all_curves: Vec<CubicBez> = kurbo_paths
                    .iter()
                    .filter_map(|x| match x.get_seg(el_ix).unwrap() {
                        PathSeg::Cubic(c) => Some(c),
                        _ => None,
                    })
                    .collect();
                if let Some(all_quadratics) = cubics_to_quadratic_splines(&all_curves, 1.0) {
                    if all_quadratics.len() != quadratic_paths.len() {
                        log::error!(
                            "Didn't get as many curves as we expected for {:} (fonticulus bug)",
                            glif_name
                        );
                        return GlyphContour::new();
                    }

                    for (c_ix, contour) in quadratic_paths.iter_mut().enumerate() {
                        let spline_points = all_quadratics[c_ix].points();
                        // Skip the spline start, because we already have a point for that
                        for pt in spline_points.iter().skip(1) {
                            contour.push(glyf::Point {
                                x: ot_round(pt.x) as i16,
                                y: ot_round(pt.y) as i16,
                                on_curve: false,
                            });
                        }
                        // Last one is on-curve
                        if let Some(last) = contour.last_mut() {
                            last.on_curve = true
                        }
                    }
                } else {
                    log::warn!("Could not compatibly interpolate {:}", glif_name)
                }
            }
            _ => {
                for (c_ix, contour) in quadratic_paths.iter_mut().enumerate() {
                    let this_path_el = kurbo_paths[c_ix].elements()[el_ix];
                    match this_path_el {
                        PathEl::MoveTo(pt) | PathEl::LineTo(pt) => contour.push(glyf::Point {
                            x: ot_round(pt.x) as i16,
                            y: ot_round(pt.y) as i16,
                            on_curve: true,
                        }),
                        PathEl::QuadTo(pt1, pt2) => {
                            // This can happen with components we already converted
                            contour.push(glyf::Point {
                                x: ot_round(pt1.x) as i16,
                                y: ot_round(pt1.y) as i16,
                                on_curve: false,
                            });
                            contour.push(glyf::Point {
                                x: ot_round(pt2.x) as i16,
                                y: ot_round(pt2.y) as i16,
                                on_curve: true,
                            });
                        }
                        PathEl::CurveTo(_, _, _) => {
                            log::error!("Why is there a cubic in {}? (fonticulus bug)", glif_name);
                            return GlyphContour::new();
                        }
                        PathEl::ClosePath => {
                            if let (Some(f), Some(l)) = (contour.first(), contour.last()) {
                                if f.x == l.x && f.y == l.y {
                                    contour.pop();
                                }
                            }
                            // TrueType curves go backwards
                            contour.reverse();
                            contour.rotate_right(1);
                        }
                    }
                }
            }
        }
    }

    quadratic_paths
}

fn babelfont_component_to_glyf_component(
    component: &babelfont::Component,
    mapping: &BTreeMap<String, u16>,
) -> Option<glyf::Component> {
    if let Some(&glyph_index) = mapping.get(&component.reference) {
        Some(glyf::Component {
            glyph_index,
            match_points: None,
            flags: glyf::ComponentFlags::empty(),
            transformation: component.transform,
        })
    } else {
        log::warn!("Couldn't find component for {:?}", component.reference);
        None
    }
}
