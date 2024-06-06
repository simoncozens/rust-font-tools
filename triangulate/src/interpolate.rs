use nalgebra::DVector;
use otmath::{Location, VariationModel};
use rbf_interp::Scatter;

fn interpolate<T: Into<f64> + Copy>(
    numbers: &[Option<ndarray::Array1<T>>],
    model: &VariationModel<String>,
    location: &Location<String>,
) -> ndarray::Array1<f64> {
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
            let this_val_vec: Vec<f64> = number.to_vec().iter().map(|x| (*x).into()).collect();
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
    let deltas: Vec<f64> = scatter.eval(coords).as_slice().to_owned();
    deltas.into()
}

pub(crate) fn interpolate_field<T: core::fmt::Debug>(
    object: &mut T,
    masters: &[Option<&T>],
    model: &VariationModel<String>,
    location: &Location<String>,
    gatherer: fn(&T) -> ndarray::Array1<f64>,
    setter: fn(&mut T, &[f64]),
) {
    let default_numbers: ndarray::Array1<f64> = gatherer(object);
    let deltas: Vec<Option<ndarray::Array1<f64>>> = masters
        .iter()
        .map(|m| {
            m.and_then(|g| {
                let nums: ndarray::Array1<f64> = gatherer(g);
                if nums.shape() == default_numbers.shape() {
                    Some(nums - default_numbers.clone())
                } else {
                    log::warn!("Incompatible masters in {:?}", g);
                    None
                }
            })
        })
        .collect();
    let deltas = interpolate(&deltas, model, location);
    let new_values = default_numbers + deltas;
    // default_numbers + values
    setter(object, new_values.as_slice().expect("Couldn't get slice"))
}
