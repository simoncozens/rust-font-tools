pub fn adjust_offset<T>(offset: T, angle: f64) -> i32
where
    f64: From<T>,
{
    if angle == 0.0 {
        return 0;
    }
    (f64::from(offset) * (-angle).to_radians().tan()).round() as i32
}

pub fn is_all_same<T: std::cmp::PartialEq + Copy>(arr: &[T]) -> bool {
    if arr.is_empty() {
        return true;
    }
    let first = arr[0];
    arr.iter().all(|&item| item == first)
}
