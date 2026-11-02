//! Regression test: `ACCTKEY` inside `BANKACCTFROM` (single-line SGML).
//!
//! `ACCTKEY` is a valid leaf of `BANKACCTFROM`. Before it was recognized as
//! such, the normalizer closed `BANKACCTFROM` before `<ACCTKEY>` was opened,
//! and the explicit `</BANKACCTFROM>` that followed then unwound the whole
//! tag stack up to `</OFX>` — dropping `BANKTRANLIST` outside the document,
//! so the statement parsed successfully but with no transaction list.

use ofx_kit::parse;
use ofx_kit::types::{AccountType, TransactionType};

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
<BANKMSGSRSV1><STMTTRNRS><TRNUID>1234<STATUS><CODE>0<SEVERITY>INFO</STATUS><CLTCOOKIE>123456<STMTRS><CURDEF>EUR<BANKACCTFROM><BANKID>0000<BRANCHID>00001<ACCTID>1231<ACCTTYPE>CHECKING<ACCTKEY>41</BANKACCTFROM><BANKTRANLIST><DTSTART>20260918<DTEND>20261016<STMTTRN><TRNTYPE>DIRECTDEP<DTPOSTED>20261015<TRNAMT>-10.00<FITID>1234sen<NAME>VIREMENT PERMANENT POUR</STMTTRN></BANKTRANLIST><LEDGERBAL><BALAMT>100.00<DTASOF>20261016</LEDGERBAL></STMTRS></STMTTRNRS></BANKMSGSRSV1>
</OFX>
";

#[test]
fn parses_sgml_statement_with_acctkey_in_bank_account() {
    let doc = parse(INPUT).expect("document should parse");

    let banking = doc
        .banking()
        .expect("banking message set should be present");
    let wrapper = &banking.statement_responses()[0];
    assert!(wrapper.status().is_success());

    let stmt = wrapper
        .response()
        .expect("statement response should be present");
    assert_eq!(stmt.currency_default().as_str(), "EUR");
    assert_eq!(stmt.bank_account().account_type(), AccountType::Checking);

    let txn_list = stmt
        .transaction_list()
        .expect("transaction list should be present when ACCTKEY follows ACCTTYPE");
    assert_eq!(txn_list.len(), 1);

    let txn = &txn_list.transactions()[0];
    assert_eq!(txn.transaction_type(), TransactionType::DirectDeposit);
    assert_eq!(txn.fit_id().as_str(), "1234sen");
    assert_eq!(txn.name(), Some("VIREMENT PERMANENT POUR"));
    assert!(txn.amount().is_negative());

    let ledger = stmt
        .ledger_balance()
        .expect("ledger balance should be present");
    assert_eq!(
        ledger.amount().as_decimal(),
        rust_decimal::Decimal::new(10000, 2)
    );
}
