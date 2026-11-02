//! Regression test: non-ASCII text in SGML statements.
//!
//! `normalize_sgml_to_xml` used to walk the body byte-by-byte and cast each
//! byte to `char`, so every multi-byte UTF-8 character in a leaf value came
//! out of `parse()` as mojibake (e.g. "CAFÉ LÉON" became "CAFÃ\u{89}
//! LÃ\u{89}ON"). Text runs must be copied as-is.

use ofx_kit::parse;

const INPUT: &str = "\
OFXHEADER:100
DATA:OFXSGML
VERSION:102
SECURITY:NONE
ENCODING:USASCII
CHARSET:1252
COMPRESSION:NONE
OLDFILEUID:NONE
NEWFILEUID:NONE

<OFX>
<SIGNONMSGSRSV1><SONRS><STATUS><CODE>0<SEVERITY>INFO</STATUS><DTSERVER>20261016120000<LANGUAGE>FRA</SONRS></SIGNONMSGSRSV1>
<BANKMSGSRSV1><STMTTRNRS><TRNUID>1234<STATUS><CODE>0<SEVERITY>INFO</STATUS><STMTRS><CURDEF>EUR<BANKACCTFROM><BANKID>0000<ACCTID>1231<ACCTTYPE>CHECKING</BANKACCTFROM><BANKTRANLIST><DTSTART>20261001<DTEND>20261016<STMTTRN><TRNTYPE>DEBIT<DTPOSTED>20261010<TRNAMT>-12.50<FITID>cp1252-1<NAME>CAFÉ LÉON<MEMO>Croissants à la boulangerie</STMTTRN></BANKTRANLIST></STMTRS></STMTTRNRS></BANKMSGSRSV1>
</OFX>
";

#[test]
fn preserves_non_ascii_leaf_text() {
    let doc = parse(INPUT).expect("document should parse");

    let banking = doc
        .banking()
        .expect("banking message set should be present");
    let stmt = banking.statement_responses()[0]
        .response()
        .expect("statement response should be present");
    let txn_list = stmt
        .transaction_list()
        .expect("transaction list should be present");
    assert_eq!(txn_list.len(), 1);

    let txn = &txn_list.transactions()[0];
    assert_eq!(txn.name(), Some("CAFÉ LÉON"));
    assert_eq!(txn.memo(), Some("Croissants à la boulangerie"));
    assert_eq!(txn.fit_id().as_str(), "cp1252-1");
}
