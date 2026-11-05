//! Typed OFX document parser and public parsing options.
//!
//! Parsing is a hand-written recursive descent over [`crate::xml::OfxReader`].
//! OFX 1.x bodies are normalized by [`crate::sgml`] before the same typed
//! aggregate parser is used. This module intentionally supports the documented
//! bank-statement subset plus the existing credit-card and investment subsets.

use quick_xml::events::Event;

use crate::aggregates::{
    AvailableBalance, Balance, BankAccount, CcStatementResponse, CreditCardAccount, CurrencyInfo,
    InvStatementResponse, InvestmentAccount, LedgerBalance, Payee, StatementResponse,
    StatementTransactionBuilder, Status, TransactionList, TransactionWrapper,
};
use crate::diagnostics::{ParseWarning, position_at};
use crate::document::{
    BankingMessageSet, CreditCardMessageSet, InvestmentMessageSet, OfxDocument, SignonResponse,
};
use crate::encoding::DecodeWarning;
use crate::error::OfxError;
use crate::header::{self, OfxHeader};
use crate::xml::{OfxReader, XmlError};

mod balance;
mod banking;
mod common;
mod credit_card;
mod currency;
mod investment;
mod payee;
mod signon;
mod transaction;

use balance::*;
use banking::*;
use common::*;
use credit_card::*;
use currency::*;
use investment::*;
use payee::*;
use signon::*;
use transaction::*;

/// A parsed OFX document together with the non-fatal warnings collected while
/// parsing it.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDocument {
    /// The structured OFX document.
    pub document: OfxDocument,
    /// Non-fatal conditions encountered while parsing.
    pub warnings: Vec<ParseWarning>,
    /// Warnings emitted while decoding a byte-oriented source.
    pub decode_warnings: Vec<DecodeWarning>,
}

/// Controls how the parser handles elements outside the supported subset.
#[non_exhaustive]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnknownElementPolicy {
    /// Skip unknown elements without reporting them.
    Ignore,
    /// Skip unknown elements and return a warning.
    #[default]
    Warn,
    /// Return an error when an unknown element is encountered.
    Error,
}

/// Options controlling typed OFX parsing.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseOptions {
    /// Policy applied to elements outside the supported subset.
    pub unknown_elements: UnknownElementPolicy,
    /// Turn structural SGML diagnostics into parsing errors.
    pub strict: bool,
}

impl Default for ParseOptions {
    fn default() -> Self {
        Self {
            unknown_elements: UnknownElementPolicy::Warn,
            strict: false,
        }
    }
}

impl ParseOptions {
    /// Sets the unknown-element policy.
    #[must_use]
    pub const fn with_unknown_elements(mut self, policy: UnknownElementPolicy) -> Self {
        self.unknown_elements = policy;
        self
    }

    /// Enables or disables strict structural parsing.
    #[must_use]
    pub const fn with_strict(mut self, strict: bool) -> Self {
        self.strict = strict;
        self
    }
}

/// Parses a complete OFX document from a string.
///
/// This is the primary entry point for the library. It accepts the full
/// content of an OFX file (header + XML body) and returns a structured document.
///
/// Warnings are discarded; use [`parse_with_warnings`] to receive them.
///
/// # Errors
///
/// Returns `OfxError` if the header is missing or malformed, the XML is invalid,
/// or required OFX elements are missing.
pub fn parse(input: &str) -> Result<OfxDocument, OfxError> {
    parse_with_warnings(input).map(|parsed| parsed.document)
}

/// Parses a complete OFX document and returns the non-fatal warnings it
/// generated.
///
/// # Errors
///
/// Returns `OfxError` if the header is missing or malformed, the XML is invalid,
/// or required OFX elements are missing.
pub fn parse_with_warnings(input: &str) -> Result<ParsedDocument, OfxError> {
    parse_with_options(input, &ParseOptions::default())
}

/// Parses an OFX document with explicit parser options and returns warnings.
pub fn parse_with_options(input: &str, options: &ParseOptions) -> Result<ParsedDocument, OfxError> {
    let (ofx_header, xml_body) = header::parse_header(input)?;
    // Keep the body untrimmed so positions remain relative to the complete
    // decoded source, including the header and any leading whitespace.
    let body_offset = input.len() - xml_body.len();

    // OFX 1.x uses SGML where closing tags are optional. We need to normalize
    // the body to well-formed XML before parsing.
    if ofx_header.version().major() < 2 {
        let normalized = crate::sgml::normalize_sgml_to_xml_with_warnings(xml_body);
        let mut normalized_warnings = normalized.warnings;
        remap_positions(&mut normalized_warnings, input, body_offset);
        reject_strict_warnings(&normalized_warnings, options)?;
        let (document, mut xml_warnings) = parse_ofx_body(
            &normalized.xml,
            ofx_header,
            options,
            input,
            body_offset,
            false,
        )?;
        let mut warnings = normalized_warnings;
        warnings.append(&mut xml_warnings);
        Ok(ParsedDocument {
            document,
            warnings,
            decode_warnings: Vec::new(),
        })
    } else {
        let (document, warnings) =
            parse_ofx_body(xml_body, ofx_header, options, input, body_offset, true)?;
        Ok(ParsedDocument {
            document,
            warnings,
            decode_warnings: Vec::new(),
        })
    }
}

/// Decodes and parses an OFX document from raw bytes.
///
/// Warnings are discarded; use [`parse_bytes_with_warnings`] to receive them.
pub fn parse_bytes(input: &[u8]) -> Result<OfxDocument, OfxError> {
    parse_bytes_with_warnings(input).map(|parsed| parsed.document)
}

/// Decodes and parses an OFX document from raw bytes, returning non-fatal
/// warnings encountered while parsing.
///
/// Decoding warnings are available separately through [`crate::decode`].
pub fn parse_bytes_with_warnings(input: &[u8]) -> Result<ParsedDocument, OfxError> {
    parse_bytes_with_options(input, &ParseOptions::default())
}

/// Decodes and parses an OFX document with explicit parser options.
pub fn parse_bytes_with_options(
    input: &[u8],
    options: &ParseOptions,
) -> Result<ParsedDocument, OfxError> {
    if crate::encoding::is_compressed(input) {
        return Err(OfxError::UnsupportedCompression);
    }
    let decoded = crate::encoding::decode(input);
    let (header, _) = header::parse_header(&decoded.text)?;
    if header
        .compression()
        .is_some_and(|compression| !compression.eq_ignore_ascii_case("NONE"))
    {
        return Err(OfxError::UnsupportedCompression);
    }
    let mut parsed = parse_with_options(&decoded.text, options)?;
    parsed.decode_warnings = decoded.warnings;
    Ok(parsed)
}

fn reject_strict_warnings(
    warnings: &[ParseWarning],
    options: &ParseOptions,
) -> Result<(), OfxError> {
    if !options.strict {
        return Ok(());
    }
    if let Some(warning) = warnings.iter().find(|warning| {
        matches!(
            warning.kind(),
            crate::diagnostics::ParseWarningKind::MisplacedElement { .. }
                | crate::diagnostics::ParseWarningKind::UnmatchedCloseTag { .. }
                | crate::diagnostics::ParseWarningKind::UnclosedElement { .. }
                | crate::diagnostics::ParseWarningKind::AmbiguousUnknownElement { .. }
                | crate::diagnostics::ParseWarningKind::MalformedSgml { .. }
        )
    }) {
        return Err(OfxError::StrictViolation(warning.clone()));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Tag name helpers -- compare against byte slices to avoid allocations
// ---------------------------------------------------------------------------

fn parse_ofx_body(
    xml: &str,
    header: OfxHeader,
    options: &ParseOptions,
    position_source: &str,
    position_offset: usize,
    track_positions: bool,
) -> Result<(OfxDocument, Vec<ParseWarning>), OfxError> {
    let mut reader = OfxReader::with_options_and_source(
        xml,
        options,
        position_source,
        position_offset,
        track_positions,
    );
    let mut signon: Option<SignonResponse> = None;
    let mut banking = Vec::new();
    let mut credit_card = Vec::new();
    let mut investment = Vec::new();

    // Find and enter the <OFX> root element
    loop {
        match reader.next_event()? {
            Event::Start(e) if tag_name(&e) == "OFX" => break,
            Event::Eof => {
                return Err(reader.missing_in("document", "OFX").into());
            }
            _ => {}
        }
    }

    // Parse children of <OFX>
    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "SIGNONMSGSRSV1" => {
                        signon = Some(parse_signon_message_set(&mut reader)?);
                    }
                    "BANKMSGSRSV1" => {
                        banking.push(parse_banking_message_set(&mut reader)?);
                    }
                    "CREDITCARDMSGSRSV1" => {
                        credit_card.push(parse_cc_message_set(&mut reader)?);
                    }
                    "INVSTMTMSGSRSV1" => {
                        investment.push(parse_investment_message_set(&mut reader)?);
                    }
                    _ => {
                        reader.skip_element(&name)?;
                    }
                }
            }
            Event::End(e) if end_tag_name(&e) == "OFX" => break,
            Event::Eof => break,
            _ => {}
        }
    }

    let signon = signon.ok_or_else(|| reader.missing_in("OFX", "SIGNONMSGSRSV1"))?;

    let mut doc = OfxDocument::new(header, signon);
    for b in banking {
        doc = doc.with_banking(b);
    }
    for cc in credit_card {
        doc = doc.with_credit_card(cc);
    }
    for inv in investment {
        doc = doc.with_investment(inv);
    }

    Ok((doc, reader.take_warnings()))
}

fn remap_positions(warnings: &mut [ParseWarning], source: &str, source_offset: usize) {
    for warning in warnings {
        if let Some(position) = warning.position.as_mut() {
            *position = position_at(source, source_offset + position.byte_offset);
        }
    }
}

// ---------------------------------------------------------------------------
// Signon message set
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    const SIMPLE_BANK_OFX: &str = r#"<?OFX OFXHEADER="200" VERSION="220" SECURITY="NONE" OLDFILEUID="NONE" NEWFILEUID="NONE"?>
<OFX>
<SIGNONMSGSRSV1>
<SONRS>
<STATUS>
<CODE>0</CODE>
<SEVERITY>INFO</SEVERITY>
</STATUS>
<DTSERVER>20230115120000</DTSERVER>
<LANGUAGE>ENG</LANGUAGE>
<FI>
<ORG>MyBank</ORG>
<FID>1234</FID>
</FI>
</SONRS>
</SIGNONMSGSRSV1>
<BANKMSGSRSV1>
<STMTTRNRS>
<TRNUID>1001</TRNUID>
<STATUS>
<CODE>0</CODE>
<SEVERITY>INFO</SEVERITY>
</STATUS>
<STMTRS>
<CURDEF>USD</CURDEF>
<BANKACCTFROM>
<BANKID>123456789</BANKID>
<ACCTID>987654321</ACCTID>
<ACCTTYPE>CHECKING</ACCTTYPE>
</BANKACCTFROM>
<BANKTRANLIST>
<DTSTART>20230101</DTSTART>
<DTEND>20230131</DTEND>
<STMTTRN>
<TRNTYPE>DEBIT</TRNTYPE>
<DTPOSTED>20230115</DTPOSTED>
<TRNAMT>-50.00</TRNAMT>
<FITID>20230115001</FITID>
<NAME>GROCERY STORE</NAME>
<MEMO>Weekly groceries</MEMO>
</STMTTRN>
</BANKTRANLIST>
<LEDGERBAL>
<BALAMT>1500.00</BALAMT>
<DTASOF>20230115120000</DTASOF>
</LEDGERBAL>
</STMTRS>
</STMTTRNRS>
</BANKMSGSRSV1>
</OFX>"#;

    #[test]
    fn parse_simple_bank_statement() {
        let doc = parse(SIMPLE_BANK_OFX).unwrap();

        assert_eq!(doc.header().version(), "220".parse().unwrap());
        assert!(doc.signon().status().is_success());
        assert_eq!(doc.signon().language(), "ENG");

        let banking = doc.banking().expect("banking should be present");
        assert_eq!(banking.statement_responses().len(), 1);

        let wrapper = &banking.statement_responses()[0];
        assert!(wrapper.status().is_success());
        assert_eq!(wrapper.transaction_uid(), "1001");

        let stmt = wrapper.response().expect("response should be present");
        assert_eq!(stmt.currency_default().as_str(), "USD");
        assert_eq!(stmt.bank_account().bank_id().as_str(), "123456789");
        assert_eq!(stmt.bank_account().account_id().as_str(), "987654321");

        let txn_list = stmt
            .transaction_list()
            .expect("transaction list should be present");
        assert_eq!(txn_list.len(), 1);

        let txn = &txn_list.transactions()[0];
        assert_eq!(txn.transaction_type(), crate::types::TransactionType::Debit);
        assert_eq!(txn.name(), Some("GROCERY STORE"));
        assert_eq!(txn.memo(), Some("Weekly groceries"));

        let ledger = stmt
            .ledger_balance()
            .expect("ledger balance should be present");
        assert_eq!(
            ledger.amount().as_decimal(),
            rust_decimal::Decimal::new(150000, 2)
        );
    }

    #[test]
    fn parse_multiple_transactions() {
        let input = r#"<?OFX OFXHEADER="200" VERSION="220" SECURITY="NONE" OLDFILEUID="NONE" NEWFILEUID="NONE"?>
<OFX>
<SIGNONMSGSRSV1>
<SONRS>
<STATUS><CODE>0</CODE><SEVERITY>INFO</SEVERITY></STATUS>
<DTSERVER>20230131120000</DTSERVER>
<LANGUAGE>ENG</LANGUAGE>
</SONRS>
</SIGNONMSGSRSV1>
<BANKMSGSRSV1>
<STMTTRNRS>
<TRNUID>2001</TRNUID>
<STATUS><CODE>0</CODE><SEVERITY>INFO</SEVERITY></STATUS>
<STMTRS>
<CURDEF>USD</CURDEF>
<BANKACCTFROM>
<BANKID>111222333</BANKID>
<ACCTID>444555666</ACCTID>
<ACCTTYPE>CHECKING</ACCTTYPE>
</BANKACCTFROM>
<BANKTRANLIST>
<DTSTART>20230101</DTSTART>
<DTEND>20230131</DTEND>
<STMTTRN>
<TRNTYPE>DEBIT</TRNTYPE>
<DTPOSTED>20230105</DTPOSTED>
<TRNAMT>-25.00</TRNAMT>
<FITID>TXN001</FITID>
<NAME>COFFEE SHOP</NAME>
</STMTTRN>
<STMTTRN>
<TRNTYPE>CREDIT</TRNTYPE>
<DTPOSTED>20230110</DTPOSTED>
<TRNAMT>3000.00</TRNAMT>
<FITID>TXN002</FITID>
<NAME>PAYROLL</NAME>
<MEMO>Direct deposit</MEMO>
</STMTTRN>
<STMTTRN>
<TRNTYPE>DEBIT</TRNTYPE>
<DTPOSTED>20230115</DTPOSTED>
<TRNAMT>-1200.00</TRNAMT>
<FITID>TXN003</FITID>
<NAME>RENT PAYMENT</NAME>
<CHECKNUM>1042</CHECKNUM>
</STMTTRN>
<STMTTRN>
<TRNTYPE>DEBIT</TRNTYPE>
<DTPOSTED>20230120</DTPOSTED>
<TRNAMT>-89.50</TRNAMT>
<FITID>TXN004</FITID>
<NAME>ELECTRIC COMPANY</NAME>
<MEMO>Monthly bill</MEMO>
</STMTTRN>
<STMTTRN>
<TRNTYPE>DEBIT</TRNTYPE>
<DTPOSTED>20230125</DTPOSTED>
<TRNAMT>-42.99</TRNAMT>
<FITID>TXN005</FITID>
<NAME>STREAMING SERVICE</NAME>
<SIC>7812</SIC>
</STMTTRN>
</BANKTRANLIST>
<LEDGERBAL>
<BALAMT>1642.51</BALAMT>
<DTASOF>20230131120000</DTASOF>
</LEDGERBAL>
<AVAILBAL>
<BALAMT>1542.51</BALAMT>
<DTASOF>20230131120000</DTASOF>
</AVAILBAL>
</STMTRS>
</STMTTRNRS>
</BANKMSGSRSV1>
</OFX>"#;

        let doc = parse(input).unwrap();
        let banking = doc.banking().unwrap();
        let stmt = banking.statement_responses()[0].response().unwrap();

        // Verify all 5 transactions were parsed
        let txn_list = stmt.transaction_list().unwrap();
        assert_eq!(txn_list.len(), 5);

        // Verify each transaction's unique data
        assert_eq!(txn_list.transactions()[0].fit_id().as_str(), "TXN001");
        assert_eq!(txn_list.transactions()[0].name(), Some("COFFEE SHOP"));
        assert_eq!(
            txn_list.transactions()[0].amount().as_decimal(),
            rust_decimal::Decimal::new(-2500, 2)
        );

        assert_eq!(txn_list.transactions()[1].fit_id().as_str(), "TXN002");
        assert_eq!(
            txn_list.transactions()[1].transaction_type(),
            crate::types::TransactionType::Credit
        );
        assert_eq!(txn_list.transactions()[1].memo(), Some("Direct deposit"));

        assert_eq!(txn_list.transactions()[2].fit_id().as_str(), "TXN003");
        assert_eq!(
            txn_list.transactions()[2].check_number().unwrap().as_str(),
            "1042"
        );

        assert_eq!(txn_list.transactions()[3].fit_id().as_str(), "TXN004");
        assert_eq!(txn_list.transactions()[3].memo(), Some("Monthly bill"));

        assert_eq!(txn_list.transactions()[4].fit_id().as_str(), "TXN005");
        assert_eq!(txn_list.transactions()[4].sic(), Some(7812));

        // Verify both balances
        let ledger = stmt.ledger_balance().unwrap();
        assert_eq!(
            ledger.amount().as_decimal(),
            rust_decimal::Decimal::new(164251, 2)
        );

        let avail = stmt.available_balance().unwrap();
        assert_eq!(
            avail.amount().as_decimal(),
            rust_decimal::Decimal::new(154251, 2)
        );
    }

    #[test]
    fn parse_missing_header_returns_error() {
        let result = parse("<OFX></OFX>");
        assert!(result.is_err());
    }

    #[test]
    fn parse_missing_signon_returns_error() {
        let input = r#"<?OFX OFXHEADER="200" VERSION="220" SECURITY="NONE" OLDFILEUID="NONE" NEWFILEUID="NONE"?><OFX></OFX>"#;
        let result = parse(input);
        assert!(result.is_err());
    }
}
