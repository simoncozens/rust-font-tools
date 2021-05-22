use bitflags::bitflags;
use otspec::types::*;
use otspec_macros::tables;
use serde::{Deserialize, Serialize};

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
    use crate::gasp;

    #[test]
    fn gasp_serde() {
        let binary_gasp = vec![
            0x00, 0x00, 0x00, 0x02, 0x00, 0x08, 0x00, 0x02, 0xff, 0xff, 0x00, 0x03,
        ];
        let fgasp: gasp::gasp = otspec::de::from_bytes(&binary_gasp).unwrap();
        let expected = gasp::gasp {
            version: 0,
            gaspRanges: vec![
                gasp::GaspRecord {
                    rangeMaxPPEM: 8,
                    rangeGaspBehavior: gasp::RangeGaspBehaviorFlags::GASP_DOGRAY,
                },
                gasp::GaspRecord {
                    rangeMaxPPEM: 65535,
                    rangeGaspBehavior: gasp::RangeGaspBehaviorFlags::GASP_GRIDFIT
                        | gasp::RangeGaspBehaviorFlags::GASP_DOGRAY,
                },
            ],
        };
        assert_eq!(fgasp, expected);
        let serialized = otspec::ser::to_bytes(&fgasp).unwrap();
        assert_eq!(serialized, binary_gasp);
    }
}
