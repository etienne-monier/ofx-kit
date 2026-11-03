//! OFX parser support for the credit card domain.
//!
//! This module contains the hand-written aggregate readers for its scope.

use super::*;

pub(super) fn parse_cc_message_set(
    reader: &mut OfxReader<'_>,
) -> Result<CreditCardMessageSet, OfxError> {
    let mut responses = Vec::new();

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                if name == "CCSTMTTRNRS" {
                    responses.push(parse_ccstmttrnrs(reader)?);
                } else {
                    reader.skip_element(&name)?;
                }
            }
            Event::End(e) if end_tag_name(&e) == "CREDITCARDMSGSRSV1" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in CREDITCARDMSGSRSV1".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    Ok(CreditCardMessageSet::new(responses))
}

pub(super) fn parse_ccstmttrnrs(
    reader: &mut OfxReader<'_>,
) -> Result<TransactionWrapper<CcStatementResponse>, OfxError> {
    let mut trnuid: Option<String> = None;
    let mut status: Option<Status> = None;
    let mut client_cookie: Option<String> = None;
    let mut ccstmtrs: Option<CcStatementResponse> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "TRNUID" => trnuid = Some(read_text(reader, "TRNUID")?),
                    "STATUS" => status = Some(parse_status(reader)?),
                    "CLTCOOKIE" => client_cookie = Some(read_text(reader, "CLTCOOKIE")?),
                    "CCSTMTRS" => ccstmtrs = Some(parse_ccstmtrs(reader)?),
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "CCSTMTTRNRS" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in CCSTMTTRNRS".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let trnuid = trnuid.ok_or_else(|| reader.missing_in("CCSTMTTRNRS", "TRNUID"))?;
    let status = status.ok_or_else(|| reader.missing_in("CCSTMTTRNRS", "STATUS"))?;

    let mut wrapper = TransactionWrapper::new(trnuid, status, ccstmtrs);
    if let Some(cookie) = client_cookie {
        wrapper = wrapper.with_client_cookie(cookie);
    }
    Ok(wrapper)
}

pub(super) fn parse_ccstmtrs(reader: &mut OfxReader<'_>) -> Result<CcStatementResponse, OfxError> {
    let mut curdef: Option<crate::types::CurrencyCode> = None;
    let mut cc_acct: Option<CreditCardAccount> = None;
    let mut transaction_list: Option<TransactionList> = None;
    let mut ledger_balance: Option<LedgerBalance> = None;
    let mut available_balance: Option<AvailableBalance> = None;
    let mut balance_list: Vec<Balance> = Vec::new();
    let mut marketing_info: Option<String> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "CURDEF" => {
                        let text = read_text(reader, "CURDEF")?;
                        curdef = Some(parse_text_as(reader, &text, "CURDEF")?);
                    }
                    "CCACCTFROM" => cc_acct = Some(parse_cc_account(reader, "CCACCTFROM")?),
                    "BANKTRANLIST" => transaction_list = Some(parse_bank_transaction_list(reader)?),
                    "LEDGERBAL" => ledger_balance = Some(parse_ledger_balance(reader)?),
                    "AVAILBAL" => available_balance = Some(parse_available_balance(reader)?),
                    "BALLIST" => balance_list = parse_balance_list(reader)?,
                    "MKTGINFO" => marketing_info = Some(read_text(reader, "MKTGINFO")?),
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "CCSTMTRS" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in CCSTMTRS".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let curdef = curdef.ok_or_else(|| reader.missing_in("CCSTMTRS", "CURDEF"))?;
    let cc_acct = cc_acct.ok_or_else(|| reader.missing_in("CCSTMTRS", "CCACCTFROM"))?;

    let mut stmt = CcStatementResponse::new(curdef, cc_acct);
    if let Some(tl) = transaction_list {
        stmt = stmt.with_transaction_list(tl);
    }
    if let Some(lb) = ledger_balance {
        stmt = stmt.with_ledger_balance(lb);
    }
    if let Some(ab) = available_balance {
        stmt = stmt.with_available_balance(ab);
    }
    if !balance_list.is_empty() {
        stmt = stmt.with_balance_list(balance_list);
    }
    if let Some(info) = marketing_info {
        stmt = stmt.with_marketing_info(info);
    }

    Ok(stmt)
}

// ---------------------------------------------------------------------------
// Investment message set
// ---------------------------------------------------------------------------

pub(super) fn parse_cc_account(
    reader: &mut OfxReader<'_>,
    closing_tag: &str,
) -> Result<CreditCardAccount, OfxError> {
    let mut acct_id: Option<crate::types::AccountId> = None;
    let mut acct_key: Option<String> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "ACCTID" => {
                        let text = read_text(reader, "ACCTID")?;
                        acct_id = Some(parse_text_as(reader, &text, "ACCTID")?);
                    }
                    "ACCTKEY" => acct_key = Some(read_text(reader, "ACCTKEY")?),
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == closing_tag => break,
            Event::Eof => {
                return Err(reader
                    .malformed(format!("unexpected EOF in {closing_tag}"))
                    .into());
            }
            _ => {}
        }
    }

    let acct_id = acct_id.ok_or_else(|| reader.missing_in(closing_tag, "ACCTID"))?;

    Ok(CreditCardAccount::new(acct_id, acct_key))
}

// ---------------------------------------------------------------------------
// Payee aggregate
// ---------------------------------------------------------------------------
