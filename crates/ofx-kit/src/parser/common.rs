//! OFX parser support for the common domain.
//!
//! This module contains the hand-written aggregate readers for its scope.

use super::*;

pub(super) fn tag_name(e: &quick_xml::events::BytesStart<'_>) -> String {
    String::from_utf8_lossy(e.name().as_ref()).into_owned()
}

pub(super) fn end_tag_name(e: &quick_xml::events::BytesEnd<'_>) -> String {
    String::from_utf8_lossy(e.name().as_ref()).into_owned()
}

/// Read the text content of the current element (after Start has been consumed).
pub(super) fn read_text(reader: &mut OfxReader<'_>, tag: &str) -> Result<String, XmlError> {
    reader.read_text(tag)
}

/// Read a required text element by name inside an aggregate being walked.
/// This is for use when we've already received a Start event for `tag`.
pub(super) fn parse_text_as<T: core::str::FromStr>(
    reader: &OfxReader<'_>,
    text: &str,
    element: &str,
) -> Result<T, XmlError> {
    text.parse::<T>().map_err(|_| {
        reader.invalid_content(
            element,
            text,
            format!("failed to parse as {}", core::any::type_name::<T>()),
        )
    })
}

/// Read the text of the current element and parse it into a typed value.
pub(super) fn parse_next_as<T: core::str::FromStr>(
    reader: &mut OfxReader<'_>,
    element: &str,
) -> Result<T, XmlError> {
    let text = reader.read_text(element)?;
    parse_text_as(reader, &text, element)
}

// ---------------------------------------------------------------------------
// Top-level document parsing
// ---------------------------------------------------------------------------
