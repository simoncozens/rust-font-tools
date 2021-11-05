use bitflags::bitflags;
use otspec::types::*;
use otspec::Deserializer;
use otspec_macros::{tables, Deserialize, Serialize};

/// The 'gasp' OpenType tag.
pub const TAG: Tag = crate::tag!("gasp");

tables!(
GaspRecord {
    uint16 rangeMaxPPEM
    RangeGaspBehaviorFlags rangeGaspBehavior
}

gasp {
    uint16 version
    Counted(GaspRecord) gaspRanges
}
);

bitflags! {
    #[derive(Serialize, Deserialize)]
    /// Flags which determine how grid-fitting should be carried out
    pub struct RangeGaspBehaviorFlags: u16 {
        /// Use gridfitting
        const GASP_GRIDFIT = 0x0001;
        /// Use grayscale rendering
        const GASP_DOGRAY = 0x0002;
        /// Use gridfitting with ClearType symmetric smoothing
        const GASP_SYMMETRIC_GRIDFIT = 0x0004;
        /// Use smoothing along multiple axes with ClearType®
        const GASP_SYMMETRIC_SMOOTHING = 0x0008;
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn gasp_serde() {
        let binary_gasp = vec![
            0x00, 0x00, 0x00, 0x02, 0x00, 0x08, 0x00, 0x02, 0xff, 0xff, 0x00, 0x03,
        ];
        let fgasp: super::gasp = otspec::de::from_bytes(&binary_gasp).unwrap();
        let expected = super::gasp {
            version: 0,
            gaspRanges: vec![
                super::GaspRecord {
                    rangeMaxPPEM: 8,
                    rangeGaspBehavior: super::RangeGaspBehaviorFlags::GASP_DOGRAY,
                },
                super::GaspRecord {
                    rangeMaxPPEM: 65535,
                    rangeGaspBehavior: super::RangeGaspBehaviorFlags::GASP_GRIDFIT
                        | super::RangeGaspBehaviorFlags::GASP_DOGRAY,
                },
            ],
        };
        assert_eq!(fgasp, expected);
        let serialized = otspec::ser::to_bytes(&fgasp).unwrap();
        assert_eq!(serialized, binary_gasp);
    }
}
