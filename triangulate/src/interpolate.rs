use nalgebra::DVector;
use otmath::{Location, VariationModel};

fn interpolate<T: Into<f64> + Copy>(
    numbers: &[Option<ndarray::Array1<T>>],
    model: &VariationModel<String>,
    location: &Location<String>,
) -> ndarray::Array1<f32> {
    // log::debug!("Interpolating {:?} at {:?}", numbers, location);

    let locations = &model.original_locations;
    let mut vals: Vec<DVector<f32>> = vec![];
    let mut master_locations = vec![];
    for (maybe_number, master_location) in numbers.iter().zip(locations.iter()) {
        if let Some(number) = maybe_number {
            let this_val_vec: Vec<f32> =
                number.to_vec().iter().map(|x| (*x).into() as f32).collect();
            let this_val = DVector::from_vec(this_val_vec);
            vals.push(this_val);
            master_locations.push(master_location.clone());
        }
    }
    if master_locations.len() != locations.len() {
        VariationModel::new(master_locations, model.axis_order.clone())
            .interpolate_from_deltas(location, &vals)
    } else {
        model.interpolate_from_deltas(location, &vals)
    }
    .unwrap()
    .as_slice()
    .to_owned()
    .into()

    // let scatter = Scatter::create(centers, vals, rbf_interp::Basis::PolyHarmonic(1), 2);

    // let coords = DVector::from_fn(axis_count, |i, _| {
    //     let axis = model
    //         .axis_order
    //         .get(i)
    //         .expect("Location had wrong axis count?");
    //     let val = location.get(axis).expect("Axis not found?!");
    //     *val as f64
    // });
    // let deltas: Vec<f64> = scatter.eval(coords).as_slice().to_owned();
    // deltas.into()
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
    let deltas: ndarray::Array1<f64> = interpolate(&deltas, model, location).map(|&x| f64::from(x));
    let new_values: ndarray::Array1<f64> = default_numbers + deltas;
    // default_numbers + values
    setter(object, new_values.as_slice().expect("Couldn't get slice"))
}
