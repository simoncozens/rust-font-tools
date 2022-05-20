/// The `GDEF` (Glyph definition) table
#[allow(non_snake_case)]
pub mod GDEF;
/// The `GPOS` (Glyph positioning) table
#[allow(non_snake_case)]
pub mod GPOS;
/// The `GSUB` (Glyph substitution) table
#[allow(non_snake_case)]
pub mod GSUB;
/// The `MATH` (Mathematical typesetting) table
#[allow(non_snake_case)]
pub mod MATH;
/// The `STAT` (Style attributes) table
#[allow(non_snake_case)]
pub mod STAT;
/// The `avar` (Axis variations) table
pub mod avar;
/// The `cmap` (Character To Glyph Index Mapping) table
pub mod cmap;
/// The `cvt ` (Control Value) table
pub mod cvt;
/// The `fpgm` (Font program) table
pub mod fpgm;
/// The `fvar` (Font variations) table
pub mod fvar;
/// The `gasp` (Grid-fitting and Scan-conversion Procedure) table
pub mod gasp;
/// The `glyf` (Glyf data) table
pub mod glyf;
/// The `gvar` (Glyph variations) table
pub mod gvar;
/// The `head` (Header) table
pub mod head;
/// The `hhea` (Horizontal header) table
pub mod hhea;
/// The `hmtx` (Horizontal metrics) table
pub mod hmtx;
/// The 'loca' (Index to Location) table
pub mod loca;
/// The `maxp` (Maximum profile) table
pub mod maxp;
/// The `name` (Naming) table
pub mod name;
/// The `OS/2` (OS/2 and Windows Metrics) table
pub mod os2;
/// The `post` (PostScript) table
pub mod post;
/// The `prep` (Control Value Program) table
pub mod prep;

#[macro_export]
/// A macro that allows a high-level table structure to delegate serialization and
/// deserialization to a lower level structure.
// I'm sure there is a clever way to do this with types and trait bounds but I
// am not that clever
macro_rules! table_delegate {
    ($ours:ty, $theirs: ty) => {
        impl otspec::Serialize for $ours {
            fn to_bytes(&self, data: &mut Vec<u8>) -> Result<(), otspec::SerializationError> {
                let out: $theirs = self.into();
                out.to_bytes(data)
            }
        }

        impl otspec::Deserialize for $ours {
            fn from_bytes(
                c: &mut otspec::ReaderContext,
            ) -> Result<Self, otspec::DeserializationError>
            where
                Self: std::marker::Sized,
            {
                let incoming: $theirs = c.de()?;
                Ok(incoming.into())
            }
        }
    };
}
