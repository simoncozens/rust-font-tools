use paste::paste;

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

#[macro_export]
macro_rules! format_switching_lookup {
    ($lookup:ty  { $($variant:ident),* }) => {
        paste::paste! {

            #[derive(Debug, Clone, PartialEq)]
            pub enum [<$lookup Internal>] {
                $(
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
