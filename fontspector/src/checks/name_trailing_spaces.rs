use crate::check::{return_result, Status, StatusList};
use crate::{Check, TestFont};

fn name_trailing_spaces(f: &TestFont) -> StatusList {
    let mut problems: Vec<Status> = vec![];

    if let Ok(Some(name_table)) = f.font.tables.name() {
        for name_record in &name_table.records {
            if name_record.string.trim_end() != name_record.string {
                problems.push(Status::fail(&format!(
                    "Name table record {:}/{:}/{:}/{:} has trailing spaces that must be removed:\n`{:}`",
                    name_record.platformID,
                    name_record.encodingID,
                    name_record.languageID,
                    name_record.nameID,
                    name_record.string
                )))
            }
        }
    }
    return_result(problems)
}

pub const NAME_TRAILING_SPACES_CHECK: Check = Check {
    id: "com.google.fonts/check/name/trailing_spaces",
    title: "Name table records must not have trailing spaces.",
    rationale: None,
    proposal: Some("https://github.com/googlefonts/fontbakery/issues/2417"),
    check_one: Some(&name_trailing_spaces),
    check_all: None,
};
