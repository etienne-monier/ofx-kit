use ofx_kit::{ConversionOptions, ConversionWarning, convert_bytes, convert_str, parse};
use proptest::prelude::*;

const SGML: &str = include_str!("fixtures/sgml_header_statement.ofx");
const GOLDEN: &str = include_str!("fixtures/conversion/sgml_header_statement.xml");

#[test]
fn conversion_matches_sgml_golden_file() {
    let converted =
        convert_str(SGML, &ConversionOptions::default()).expect("SGML conversion should succeed");
    assert_eq!(converted.xml, GOLDEN);
}

#[test]
fn converted_document_can_be_reparsed() {
    let converted =
        convert_str(SGML, &ConversionOptions::default()).expect("SGML conversion should succeed");
    let document = parse(&converted.xml).expect("generated XML should be parseable");
    let statement = document
        .banking()
        .expect("banking message set should exist")
        .statement_responses()[0]
        .response()
        .expect("statement response should exist");
    assert_eq!(statement.bank_account().account_id().as_str(), "5555555555");
    assert_eq!(statement.transaction_list().expect("transactions").len(), 1);
}

#[test]
fn compact_conversion_is_idempotent() {
    let first =
        convert_str(SGML, &ConversionOptions::default()).expect("first conversion should succeed");
    let second = convert_str(&first.xml, &ConversionOptions::default())
        .expect("second conversion should succeed");
    assert_eq!(second.xml, first.xml);
}

#[test]
fn byte_conversion_preserves_decoded_text() {
    let source = include_bytes!("fixtures/encoding/utf8_bom.ofx");
    let converted = convert_bytes(source, &ConversionOptions::default())
        .expect("byte conversion should succeed");
    assert!(converted.xml.contains("<OFX>"));
    assert_eq!(converted.encoding.to_string(), "UTF-8");
}

#[test]
fn pretty_conversion_remains_well_formed() {
    let converted = convert_str(
        SGML,
        &ConversionOptions {
            pretty: true,
            target_version: None,
        },
    )
    .expect("pretty conversion should succeed");
    assert!(converted.xml.contains("\n  <SIGNONMSGSRSV1>"));
    parse(&converted.xml).expect("pretty output should be parseable");
}

#[test]
fn conversion_reports_source_header_fields_not_copied_to_xml() {
    let converted =
        convert_str(SGML, &ConversionOptions::default()).expect("SGML conversion should succeed");
    assert!(converted.warnings.iter().any(|warning| matches!(
        warning,
        ConversionWarning::DroppedHeaderField { name, value }
            if name == "ENCODING" && value == "USASCII"
    )));
    assert!(converted.warnings.iter().any(|warning| matches!(
        warning,
        ConversionWarning::DroppedHeaderField { name, value }
            if name == "CHARSET" && value == "1252"
    )));
}

#[test]
fn conversion_reports_ambiguous_unknown_elements() {
    let source = "OFXHEADER:100\nVERSION:102\nSECURITY:NONE\n\n<OFX><UNKNOWN>before<CHILD>value</CHILD>after</UNKNOWN>";
    let converted = convert_str(source, &ConversionOptions::default())
        .expect("ambiguous SGML should still convert tolerantly");
    assert!(converted.warnings.iter().any(|warning| matches!(
        warning,
        ConversionWarning::Parse(ofx_kit::ParseWarning::AmbiguousUnknownElement { name, .. })
            if name == "UNKNOWN"
    )));
}

proptest! {
    #[test]
    fn arbitrary_leaf_text_does_not_break_conversion(
        text in "[A-Za-z0-9 &<>]{0,48}"
    ) {
        let source = format!(
            "OFXHEADER:100\nVERSION:102\nSECURITY:NONE\n\n<OFX><UNKNOWN>{text}</UNKNOWN>"
        );
        let converted = convert_str(&source, &ConversionOptions::default());
        prop_assert!(converted.is_ok());
    }
}
