use ofx_kit::{DecodeWarning, DetectedEncoding, OfxError, decode, parse_bytes};

#[test]
fn decodes_raw_encoding_fixtures() {
    let cases = [
        (
            "cp1252.ofx",
            include_bytes!("fixtures/encoding/cp1252.ofx").as_slice(),
            DetectedEncoding::Windows1252,
            "CaféBank",
        ),
        (
            "utf8_bom.ofx",
            include_bytes!("fixtures/encoding/utf8_bom.ofx").as_slice(),
            DetectedEncoding::Utf8,
            "CaféBank",
        ),
        (
            "utf16le_bom.ofx",
            include_bytes!("fixtures/encoding/utf16le_bom.ofx").as_slice(),
            DetectedEncoding::Utf16Le,
            "CaféBank",
        ),
        (
            "utf16be_bom.ofx",
            include_bytes!("fixtures/encoding/utf16be_bom.ofx").as_slice(),
            DetectedEncoding::Utf16Be,
            "CaféBank",
        ),
        (
            "xml_decl_cp1252.ofx",
            include_bytes!("fixtures/encoding/xml_decl_cp1252.ofx").as_slice(),
            DetectedEncoding::Windows1252,
            "CaféBank",
        ),
        (
            "cjk_shift_jis.ofx",
            include_bytes!("fixtures/encoding/cjk_shift_jis.ofx").as_slice(),
            DetectedEncoding::ShiftJis,
            "銀行",
        ),
    ];

    for (name, bytes, expected_encoding, expected_text) in cases {
        let decoded = decode(bytes);
        assert_eq!(decoded.encoding, expected_encoding, "fixture {name}");
        assert!(
            decoded
                .warnings
                .iter()
                .all(|warning| !matches!(warning, DecodeWarning::MalformedInput { .. })),
            "fixture {name} has malformed-input warnings: {:?}",
            decoded.warnings
        );
        assert!(decoded.text.contains(expected_text), "fixture {name}");
        parse_bytes(bytes).unwrap_or_else(|error| panic!("fixture {name} failed: {error}"));
    }
}

#[test]
fn rejects_compression_declared_in_sgml_header() {
    let source = include_bytes!("fixtures/encoding/sgml_encoding_cp1252.ofx");
    let text = String::from_utf8_lossy(source).replace("COMPRESSION:NONE", "COMPRESSION:ZIP");

    assert!(matches!(
        parse_bytes(text.as_bytes()),
        Err(OfxError::UnsupportedCompression)
    ));
}

#[test]
fn decodes_mislabeled_utf8_fixture_without_losing_source_bytes() {
    let source = include_bytes!("fixtures/encoding/mislabeled_utf8_as_cp1252.ofx");
    let decoded = decode(source);

    assert_eq!(decoded.encoding, DetectedEncoding::Windows1252);
    assert!(decoded.text.contains("CafÃ©Bank"));
}
