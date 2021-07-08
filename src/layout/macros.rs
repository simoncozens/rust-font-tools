#[macro_export]
macro_rules! deserialize_lookup_match {
    ($ty_in: expr, $c:expr, $( ($lookup_type:expr, $rule:ty, $variant:path) ),* $(,)*) => {
        match $ty_in {
            $(
                $lookup_type => {
                    let stuff: Counted<Offset16<$rule>> = $c.de()?;
                    $variant(stuff.try_into()?)

                }
            )*,
            _ => panic!("Bad lookup type: {}", $ty_in)
        }
    };
}
