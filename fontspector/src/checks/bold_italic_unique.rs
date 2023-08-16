use crate::check::{return_result, Status, StatusCode, StatusList};
use crate::constants::NameID;
use crate::{Check, FontCollection};

fn bold_italic_unique(c: &FontCollection) -> StatusList {
    let ribbi = c.ribbi_fonts();
    let mut problems = vec![];
    for font in ribbi.iter() {
        let _names_list = font.get_name_entry_strings(NameID::FontFamilyName);
        match font.get_os2_fsselection() {
            Ok(fsselection) => {
                let _bold = (fsselection & (2 << 5)) != 0;
                let _italic = (fsselection & 2) != 0;
            }
            Err(_e) => problems.push(Status {
                message: Some(format!("Font {} had no OS2 table", font.filename)),
                code: StatusCode::Error,
            }),
        }
    }
    return_result(problems)
}
pub const BOLD_ITALIC_UNIQUE_CHECK: Check = Check {
    id: "com.adobe.fonts/check/family/bold_italic_unique_for_nameid1",
    title: "Check that OS/2.fsSelection bold & italic settings are unique for each NameID1",
    rationale: None,
    proposal: Some("https://github.com/googlefonts/fontbakery/pull/2388"),
    check_all: Some(&bold_italic_unique),
    check_one: None,
};
