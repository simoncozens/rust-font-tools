use otspec::types::Tuple;
/// Structs to store locations (user and normalized)
///
/// Most of these have now been moved to the otmath crate.

/// A location in the internal -1 <= 0 => 1 representation
#[derive(Debug)]
pub struct NormalizedLocation(pub Tuple);
