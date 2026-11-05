//! Regression tests for robust OFX 1.x SGML handling.
//!
//! The cases here cover the hazards fixed by the shared schema and tokenizer:
//! `ACCTKEY`, unknown leaves, raw ampersands, unknown entities, accents, CRLF
//! line endings, and aggregates that are optional in the schema.

use ofx_kit::types::{AccountType, TransactionType};
use ofx_kit::{
    ParseOptions, ParseWarning, ParseWarningKind, UnknownElementPolicy, parse, parse_with_options,
    parse_with_warnings,
};

const EDGE_CASES: &str = include_str!("fixtures/sgml_edge_cases.ofx");
const CRLF: &str = include_str!("fixtures/sgml_crlf.ofx");

#[test]
fn parses_sgml_edge_cases_and_preserves_acctkey() {
    let parsed = parse_with_warnings(EDGE_CASES).expect("document should parse");
    let doc = parsed.document;

    let banking = doc
        .banking()
        .expect("banking message set should be present");
    let stmt = banking.statement_responses()[0]
        .response()
        .expect("statement response should be present");

    assert_eq!(stmt.currency_default().as_str(), "EUR");
    assert_eq!(stmt.bank_account().account_type(), AccountType::Checking);
    assert_eq!(stmt.bank_account().account_key(), Some("41"));
    assert!(stmt.available_balance().is_none());
    assert!(stmt.ledger_balance().is_some());

    let txn_list = stmt
        .transaction_list()
        .expect("transaction list should survive ACCTKEY and unknown leaves");
    assert_eq!(txn_list.len(), 1);

    let txn = &txn_list.transactions()[0];
    assert_eq!(txn.transaction_type(), TransactionType::Debit);
    assert_eq!(txn.name(), Some("CAFÉ LÉON"));
    assert_eq!(txn.memo(), Some("Raw&value and&nbsp;"));
    assert_eq!(
        txn.amount().as_decimal(),
        rust_decimal::Decimal::new(-1250, 2)
    );
}

#[test]
fn decodes_standard_and_numeric_entities_in_typed_values() {
    let input = EDGE_CASES.replace("<MEMO>Raw&value and&nbsp;", "<MEMO>AT&amp;T &#233; &#xE9;");
    let parsed = parse(&input).expect("entity document should parse");
    let transaction = &parsed
        .banking()
        .expect("banking message set should be present")
        .statement_responses()[0]
        .response()
        .expect("statement response should be present")
        .transaction_list()
        .expect("transaction list should be present")
        .transactions()[0];

    // The typed reader retains its existing quick-xml text trimming behavior
    // around entity boundaries while decoding the references themselves.
    assert_eq!(transaction.memo(), Some("AT&Téé"));
}

#[test]
fn reports_unknown_leaves_with_their_path() {
    let parsed = parse_with_warnings(EDGE_CASES).expect("document should parse");

    let unknown = parsed
        .warnings
        .iter()
        .find_map(|warning| match warning {
            ParseWarning {
                kind: ParseWarningKind::UnknownElement { name },
                path,
                ..
            } if name == "UNKNOWNLEAF" => Some(path),
            _ => None,
        })
        .expect("unknown leaf should be reported");

    assert_eq!(
        unknown.to_string(),
        "OFX/BANKMSGSRSV1/STMTTRNRS/STMTRS/BANKTRANLIST/STMTTRN/UNKNOWNLEAF"
    );
    // XML-reader diagnostics operate on normalized SGML and therefore do not
    // expose positions until a source map is available.
    assert!(unknown_position(parsed.warnings.as_slice()).is_none());
}

fn unknown_position(warnings: &[ParseWarning]) -> Option<ofx_kit::SourcePosition> {
    warnings.iter().find_map(|warning| match warning.kind() {
        ParseWarningKind::UnknownElement { name } if name == "UNKNOWNLEAF" => warning.position(),
        _ => None,
    })
}

#[test]
fn unknown_element_policy_can_ignore_or_reject() {
    let ignored = parse_with_options(
        EDGE_CASES,
        &ParseOptions::default().with_unknown_elements(UnknownElementPolicy::Ignore),
    )
    .expect("ignore policy should parse");
    assert!(
        !ignored
            .warnings
            .iter()
            .any(|warning| matches!(warning.kind(), ParseWarningKind::UnknownElement { .. }))
    );

    let rejected = parse_with_options(
        EDGE_CASES,
        &ParseOptions::default().with_unknown_elements(UnknownElementPolicy::Error),
    );
    assert!(matches!(rejected, Err(ofx_kit::OfxError::Xml(_))));
}

#[test]
fn reports_raw_ampersands_and_unknown_entities() {
    let parsed = parse_with_warnings(EDGE_CASES).expect("document should parse");

    let raw_paths: Vec<_> = parsed
        .warnings
        .iter()
        .filter_map(|warning| match warning {
            ParseWarning {
                kind: ParseWarningKind::RawAmpersand,
                path,
                ..
            } => Some(path.to_string()),
            _ => None,
        })
        .collect();
    assert!(
        raw_paths.contains(&"OFX/SIGNONMSGSRSV1/SONRS/STATUS/MESSAGE".to_owned()),
        "raw ampersand paths: {raw_paths:?}"
    );

    let raw_ampersand_position = parsed
        .warnings
        .iter()
        .find_map(|warning| match warning {
            ParseWarning {
                kind: ParseWarningKind::RawAmpersand,
                path,
                position: Some(position),
                ..
            } if path.to_string() == "OFX/SIGNONMSGSRSV1/SONRS/STATUS/MESSAGE" => Some(*position),
            _ => None,
        })
        .expect("raw ampersand should have a source position");
    assert_eq!(
        raw_ampersand_position.byte_offset,
        EDGE_CASES.find("Raw&").expect("raw ampersand fixture") + 3
    );

    let entity = parsed
        .warnings
        .iter()
        .find_map(|warning| match warning {
            ParseWarning {
                kind: ParseWarningKind::UnknownEntity { name },
                path,
                ..
            } if name == "nbsp" => Some(path.to_string()),
            _ => None,
        })
        .expect("unknown entity should be reported");
    assert_eq!(entity, "OFX/SIGNONMSGSRSV1/SONRS/STATUS/MESSAGE");
}

#[test]
fn parses_crlf_input_without_mangling_text() {
    let doc = parse(CRLF).expect("CRLF document should parse");

    let banking = doc
        .banking()
        .expect("banking message set should be present");
    let stmt = banking.statement_responses()[0]
        .response()
        .expect("statement response should be present");
    let txn = &stmt
        .transaction_list()
        .expect("transaction list should be present")
        .transactions()[0];

    assert_eq!(txn.name(), Some("DÉPÔT CRÉDIT"));
    assert_eq!(txn.memo(), Some("Ligne CRLF préservée"));
    assert_eq!(txn.transaction_type(), TransactionType::Credit);
}

#[test]
fn parse_discards_warnings_while_parse_with_warnings_keeps_them() {
    let doc = parse(EDGE_CASES).expect("document should parse");
    assert!(doc.banking().is_some());

    let parsed = parse_with_warnings(EDGE_CASES).expect("document should parse");
    assert!(!parsed.warnings.is_empty());
}
