//! OFX parser support for the transaction domain.
//!
//! This module contains the hand-written aggregate readers for its scope.

use super::*;

pub(super) fn parse_bank_transaction_list(
    reader: &mut OfxReader<'_>,
) -> Result<TransactionList, OfxError> {
    let mut dtstart: Option<crate::types::OfxDateTime> = None;
    let mut dtend: Option<crate::types::OfxDateTime> = None;
    let mut transactions = Vec::new();

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "DTSTART" => {
                        let text = read_text(reader, "DTSTART")?;
                        dtstart = Some(parse_text_as(reader, &text, "DTSTART")?);
                    }
                    "DTEND" => {
                        let text = read_text(reader, "DTEND")?;
                        dtend = Some(parse_text_as(reader, &text, "DTEND")?);
                    }
                    "STMTTRN" => {
                        transactions.push(parse_statement_transaction(reader)?);
                    }
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "BANKTRANLIST" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in BANKTRANLIST".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let dtstart = dtstart.ok_or_else(|| reader.missing_in("BANKTRANLIST", "DTSTART"))?;
    let dtend = dtend.ok_or_else(|| reader.missing_in("BANKTRANLIST", "DTEND"))?;

    Ok(TransactionList::new(dtstart, dtend, transactions))
}

// ---------------------------------------------------------------------------
// Statement transaction (STMTTRN) -- full hierarchical parsing
// ---------------------------------------------------------------------------

pub(super) fn apply_stmttrn_field(
    builder: StatementTransactionBuilder,
    reader: &mut OfxReader<'_>,
    name: &str,
) -> Result<StatementTransactionBuilder, OfxError> {
    Ok(match name {
        "TRNTYPE" => builder.transaction_type(parse_next_as(reader, name)?),
        "DTPOSTED" => builder.date_posted(parse_next_as(reader, name)?),
        "DTUSER" => builder.date_user(parse_next_as(reader, name)?),
        "DTAVAIL" => builder.date_available(parse_next_as(reader, name)?),
        "TRNAMT" => builder.amount(parse_next_as(reader, name)?),
        "FITID" => builder.fit_id(parse_next_as(reader, name)?),
        "CORRECTFITID" => builder.correction_id(parse_next_as(reader, name)?),
        "CORRECTACTION" => builder.correction_action(parse_next_as(reader, name)?),
        "SRVRTID" => builder.server_transaction_id(parse_next_as(reader, name)?),
        "CHECKNUM" => builder.check_number(parse_next_as(reader, name)?),
        "REFNUM" => builder.reference_number(read_text(reader, name)?),
        "SIC" => builder.sic(parse_next_as(reader, name)?),
        "PAYEEID" => builder.payee_id(read_text(reader, name)?),
        "NAME" => builder.name(read_text(reader, name)?),
        "PAYEE" => builder.payee(parse_payee(reader)?),
        "BANKACCTTO" => builder.bank_account_to(parse_bank_account(reader, name)?),
        "CCACCTTO" => builder.cc_account_to(parse_cc_account(reader, name)?),
        "MEMO" => builder.memo(read_text(reader, name)?),
        "CURRENCY" | "ORIGCURRENCY" => builder.currency(parse_currency(reader, name)?),
        "INV401KSOURCE" => builder.inv401k_source(parse_next_as(reader, name)?),
        _ => {
            reader.skip_element(name)?;
            builder
        }
    })
}

pub(super) fn parse_statement_transaction(
    reader: &mut OfxReader<'_>,
) -> Result<crate::aggregates::StatementTransaction, OfxError> {
    let mut builder = StatementTransactionBuilder::new();

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                builder = apply_stmttrn_field(builder, reader, &name)?;
            }
            Event::End(e) if end_tag_name(&e) == "STMTTRN" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in STMTTRN".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    builder
        .build()
        .map_err(|msg| OfxError::Xml(reader.malformed(format!("failed to build STMTTRN: {msg}"))))
}

// ---------------------------------------------------------------------------
// Balance aggregates
// ---------------------------------------------------------------------------

pub(super) fn parse_inv_transaction_list(
    reader: &mut OfxReader<'_>,
) -> Result<TransactionList, OfxError> {
    let mut dtstart: Option<crate::types::OfxDateTime> = None;
    let mut dtend: Option<crate::types::OfxDateTime> = None;
    let mut transactions = Vec::new();

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "DTSTART" => {
                        let text = read_text(reader, "DTSTART")?;
                        dtstart = Some(parse_text_as(reader, &text, "DTSTART")?);
                    }
                    "DTEND" => {
                        let text = read_text(reader, "DTEND")?;
                        dtend = Some(parse_text_as(reader, &text, "DTEND")?);
                    }
                    "INVBANKTRAN" => {
                        if let Some(txn) = parse_invbanktran(reader)? {
                            transactions.push(txn);
                        }
                    }
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "INVTRANLIST" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in INVTRANLIST".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let dtstart = dtstart.ok_or_else(|| reader.missing_in("INVTRANLIST", "DTSTART"))?;
    let dtend = dtend.ok_or_else(|| reader.missing_in("INVTRANLIST", "DTEND"))?;

    Ok(TransactionList::new(dtstart, dtend, transactions))
}

/// Parses an INVBANKTRAN element, extracting the STMTTRN inside it.
/// Returns `None` if no STMTTRN is found (defensive, but shouldn't happen
/// in well-formed OFX).
pub(super) fn parse_invbanktran(
    reader: &mut OfxReader<'_>,
) -> Result<Option<crate::aggregates::StatementTransaction>, OfxError> {
    let mut transaction = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                if name == "STMTTRN" {
                    transaction = Some(parse_statement_transaction(reader)?);
                } else {
                    reader.skip_element(&name)?;
                }
            }
            Event::End(e) if end_tag_name(&e) == "INVBANKTRAN" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in INVBANKTRAN".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    Ok(transaction)
}
