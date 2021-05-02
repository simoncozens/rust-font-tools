use otspec::types::Tuple;

/// Structs to store locations (user and normalized)

/// A location in the user's coordinate space (e.g. wdth=200,wght=15)
pub struct UserLocation(pub Vec<Tuple>);

/// A location in the internal -1 <= 0 => 1 representation
pub struct NormalizedLocation(pub Vec<Tuple>);
