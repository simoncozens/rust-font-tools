/*
Improve the appearance of an unhinted font on Win platforms by:
    - Add a new GASP table which enables all RangeGaspBehaviorFlags
      for all sizes.
    - Add a new prep table which is optimized for unhinted fonts.
*/
use fonttools::font::Table;
use fonttools::gasp;
use fonttools_cli::{open_font, read_args, save_font};

fn main() {
    let matches = read_args(
        "ttf-fix-non-hinted",
        "Adds a gasp and prep table which is set to smooth for all sizes",
    );
    let mut infont = open_font(&matches);

    if !infont.tables.contains_key(b"gasp") {
        let gasp_table = gasp::gasp {
            version: 1,
            gaspRanges: vec![gasp::GaspRecord {
                rangeMaxPPEM: 65535,
                // Strangely, all four flags should be enabled according to Greg H
                // from Microsoft:
                // https://github.com/googlefonts/fontbakery/issues/2672#issuecomment-722027792
                rangeGaspBehavior: gasp::RangeGaspBehaviorFlags::GASP_SYMMETRIC_SMOOTHING
                    | gasp::RangeGaspBehaviorFlags::GASP_DOGRAY
                    | gasp::RangeGaspBehaviorFlags::GASP_GRIDFIT
                    | gasp::RangeGaspBehaviorFlags::GASP_SYMMETRIC_GRIDFIT
            }],
        };
        infont.tables.insert(
            *b"gasp",
            Table::Gasp(gasp_table),
        );
    }
    let prep_table = Table::Unknown(vec![0xb8, 0x01, 0xff, 0x85, 0xb0, 0x04, 0x8d]);
    infont.tables.insert(
        *b"prep",
        prep_table
    );
    save_font(infont, &matches);
}