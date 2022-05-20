/// Helper macro to deserialize lookups
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
            _ => panic!("Could not deserialize lookup type: {}", $ty_in)
        }
    };
}

/// Helper macro for converting lookups from internal to external representation
#[macro_export]
macro_rules! convert_outgoing_subtables {
    ($rule:expr, $( ($lookup_type:path, $outgoing_type:ty) ),* $(,)*) => {
        match $rule {
            $(
                $lookup_type(subtables) => {
                    let mut v: Vec<Box<dyn OffsetMarkerTrait>> = vec![];
                    for sub in subtables {
                        v.push(Box::new(Offset16::to(sub)));
                    }
                    v
                }
            )*,
            _ => panic!("Could not convert lookup to binary representation")
        }
    };
}

/// Creates a ...Internal enum for serializing/deserializing lookups which have more than one format
#[macro_export]
macro_rules! format_switching_lookup {
    ($lookup:ty  { $($variant:ident),* }) => {
        paste::paste! {

            #[derive(Debug, Clone, PartialEq)]
            /// Internal representation of $lookup
            pub enum [<$lookup Internal>] {
                $(
                    /// Internal representation of $lookup $variant
                    $variant([<$lookup $variant>]),
                )*
            }

            impl Serialize for $lookup {
                fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
                    let int: [<$lookup Internal>] = self.into();
                    int.to_bytes(data)
                }
            }

            impl Serialize for [<$lookup Internal>] {
                fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), SerializationError> {
                    match self {
                        $(
                            [<$lookup Internal>] :: $variant (s) => s.to_bytes(data),
                        )*
                    }
                }
            }

        }
    };
}
