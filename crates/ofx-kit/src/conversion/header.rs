//! OFX conversion header support.
//!
//! This private module contains the structural conversion helpers for its scope.

use super::*;

pub(super) fn header_warnings(header: &crate::header::OfxHeader) -> Vec<ConversionWarning> {
    let mut warnings = Vec::new();
    if let Some(value) = header.encoding() {
        warnings.push(ConversionWarning::DroppedHeaderField {
            name: "ENCODING".to_owned(),
            value: value.to_owned(),
        });
    }
    if let Some(value) = header.charset() {
        warnings.push(ConversionWarning::DroppedHeaderField {
            name: "CHARSET".to_owned(),
            value: value.to_owned(),
        });
    }
    for (name, value) in header.unknown_fields() {
        if !matches!(name.as_str(), "DATA" | "OFXHEADER") {
            warnings.push(ConversionWarning::DroppedHeaderField {
                name: name.clone(),
                value: value.clone(),
            });
        }
    }
    warnings
}

pub(super) fn xml_header(header: &crate::header::OfxHeader, version: OfxVersion) -> String {
    format!(
        "<?OFX OFXHEADER=\"200\" VERSION=\"{version}\" SECURITY=\"{}\" OLDFILEUID=\"{}\" NEWFILEUID=\"{}\"?>",
        header.security(),
        escape_attribute(header.old_file_uid()),
        escape_attribute(header.new_file_uid()),
    )
}

pub(super) fn escape_attribute(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;")
}

pub(super) fn strip_xml_declaration(input: &str) -> &str {
    let input = input.trim_start();
    if !input.starts_with("<?xml") {
        return input;
    }
    input.find("?>").map_or(input, |end| &input[end + 2..])
}
