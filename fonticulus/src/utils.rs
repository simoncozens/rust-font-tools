pub fn adjust_offset<T>(offset: T, angle: f64) -> i32
where
    T: Into<f32>,
{
    if angle == 0.0 {
        return 0;
    }
    (offset.into() as f64 * (-angle).to_radians().tan()).round() as i32
}

pub fn is_all_same<T: std::cmp::PartialEq + Copy>(arr: &[T]) -> bool {
    if arr.is_empty() {
        return true;
    }
    let first = arr[0];
    arr.iter().all(|&item| item == first)
}
