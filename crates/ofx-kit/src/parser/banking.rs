//! OFX parser support for the banking domain.
//!
//! This module contains the hand-written aggregate readers for its scope.

use super::*;

pub(super) fn parse_banking_message_set(
    reader: &mut OfxReader<'_>,
) -> Result<BankingMessageSet, OfxError> {
    let mut responses = Vec::new();

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                if name == "STMTTRNRS" {
                    responses.push(parse_stmttrnrs(reader)?);
                } else {
                    reader.skip_element(&name)?;
                }
            }
            Event::End(e) if end_tag_name(&e) == "BANKMSGSRSV1" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in BANKMSGSRSV1".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    Ok(BankingMessageSet::new(responses))
}

pub(super) fn parse_stmttrnrs(
    reader: &mut OfxReader<'_>,
) -> Result<TransactionWrapper<StatementResponse>, OfxError> {
    let mut trnuid: Option<String> = None;
    let mut status: Option<Status> = None;
    let mut client_cookie: Option<String> = None;
    let mut stmtrs: Option<StatementResponse> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "TRNUID" => trnuid = Some(read_text(reader, "TRNUID")?),
                    "STATUS" => status = Some(parse_status(reader)?),
                    "CLTCOOKIE" => client_cookie = Some(read_text(reader, "CLTCOOKIE")?),
                    "STMTRS" => stmtrs = Some(parse_stmtrs(reader)?),
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "STMTTRNRS" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in STMTTRNRS".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let trnuid = trnuid.ok_or_else(|| reader.missing_in("STMTTRNRS", "TRNUID"))?;
    let status = status.ok_or_else(|| reader.missing_in("STMTTRNRS", "STATUS"))?;

    let mut wrapper = TransactionWrapper::new(trnuid, status, stmtrs);
    if let Some(cookie) = client_cookie {
        wrapper = wrapper.with_client_cookie(cookie);
    }
    Ok(wrapper)
}

pub(super) fn parse_stmtrs(reader: &mut OfxReader<'_>) -> Result<StatementResponse, OfxError> {
    let mut curdef: Option<crate::types::CurrencyCode> = None;
    let mut bank_acct: Option<BankAccount> = None;
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
                    "BANKACCTFROM" => bank_acct = Some(parse_bank_account(reader, "BANKACCTFROM")?),
                    "BANKTRANLIST" => transaction_list = Some(parse_bank_transaction_list(reader)?),
                    "LEDGERBAL" => ledger_balance = Some(parse_ledger_balance(reader)?),
                    "AVAILBAL" => available_balance = Some(parse_available_balance(reader)?),
                    "BALLIST" => balance_list = parse_balance_list(reader)?,
                    "MKTGINFO" => marketing_info = Some(read_text(reader, "MKTGINFO")?),
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "STMTRS" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in STMTRS".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let curdef = curdef.ok_or_else(|| reader.missing_in("STMTRS", "CURDEF"))?;
    let bank_acct = bank_acct.ok_or_else(|| reader.missing_in("STMTRS", "BANKACCTFROM"))?;

    let mut stmt = StatementResponse::new(curdef, bank_acct);
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
// Bank transaction list (BANKTRANLIST) -- contains multiple STMTTRN
// ---------------------------------------------------------------------------

pub(super) fn parse_bank_account(
    reader: &mut OfxReader<'_>,
    closing_tag: &str,
) -> Result<BankAccount, OfxError> {
    let mut bank_id: Option<crate::types::BankId> = None;
    let mut branch_id: Option<crate::types::BranchId> = None;
    let mut acct_id: Option<crate::types::AccountId> = None;
    let mut acct_type: Option<crate::types::AccountType> = None;
    let mut acct_key: Option<String> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "BANKID" => {
                        let text = read_text(reader, "BANKID")?;
                        bank_id = Some(parse_text_as(reader, &text, "BANKID")?);
                    }
                    "BRANCHID" => {
                        let text = read_text(reader, "BRANCHID")?;
                        branch_id = Some(parse_text_as(reader, &text, "BRANCHID")?);
                    }
                    "ACCTID" => {
                        let text = read_text(reader, "ACCTID")?;
                        acct_id = Some(parse_text_as(reader, &text, "ACCTID")?);
                    }
                    "ACCTTYPE" => {
                        let text = read_text(reader, "ACCTTYPE")?;
                        acct_type = Some(parse_text_as(reader, &text, "ACCTTYPE")?);
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

    let bank_id = bank_id.ok_or_else(|| reader.missing_in(closing_tag, "BANKID"))?;
    let acct_id = acct_id.ok_or_else(|| reader.missing_in(closing_tag, "ACCTID"))?;
    let acct_type = acct_type.ok_or_else(|| reader.missing_in(closing_tag, "ACCTTYPE"))?;

    Ok(BankAccount::new(
        bank_id, branch_id, acct_id, acct_type, acct_key,
    ))
}
