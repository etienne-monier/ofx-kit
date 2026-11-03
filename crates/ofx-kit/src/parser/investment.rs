//! OFX parser support for the investment domain.
//!
//! This module contains the hand-written aggregate readers for its scope.

use super::*;

pub(super) fn parse_investment_message_set(
    reader: &mut OfxReader<'_>,
) -> Result<InvestmentMessageSet, OfxError> {
    let mut responses = Vec::new();

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                if name == "INVSTMTTRNRS" {
                    responses.push(parse_invstmttrnrs(reader)?);
                } else {
                    reader.skip_element(&name)?;
                }
            }
            Event::End(e) if end_tag_name(&e) == "INVSTMTMSGSRSV1" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in INVSTMTMSGSRSV1".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    Ok(InvestmentMessageSet::new(responses))
}

pub(super) fn parse_invstmttrnrs(
    reader: &mut OfxReader<'_>,
) -> Result<TransactionWrapper<InvStatementResponse>, OfxError> {
    let mut trnuid: Option<String> = None;
    let mut status: Option<Status> = None;
    let mut client_cookie: Option<String> = None;
    let mut invstmtrs: Option<InvStatementResponse> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "TRNUID" => trnuid = Some(read_text(reader, "TRNUID")?),
                    "STATUS" => status = Some(parse_status(reader)?),
                    "CLTCOOKIE" => client_cookie = Some(read_text(reader, "CLTCOOKIE")?),
                    "INVSTMTRS" => invstmtrs = Some(parse_invstmtrs(reader)?),
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "INVSTMTTRNRS" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in INVSTMTTRNRS".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let trnuid = trnuid.ok_or_else(|| reader.missing_in("INVSTMTTRNRS", "TRNUID"))?;
    let status = status.ok_or_else(|| reader.missing_in("INVSTMTTRNRS", "STATUS"))?;

    let mut wrapper = TransactionWrapper::new(trnuid, status, invstmtrs);
    if let Some(cookie) = client_cookie {
        wrapper = wrapper.with_client_cookie(cookie);
    }
    Ok(wrapper)
}

pub(super) fn parse_invstmtrs(
    reader: &mut OfxReader<'_>,
) -> Result<InvStatementResponse, OfxError> {
    let mut curdef: Option<crate::types::CurrencyCode> = None;
    let mut inv_acct: Option<InvestmentAccount> = None;
    let mut transaction_list: Option<TransactionList> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "CURDEF" => {
                        let text = read_text(reader, "CURDEF")?;
                        curdef = Some(parse_text_as(reader, &text, "CURDEF")?);
                    }
                    "INVACCTFROM" => {
                        inv_acct = Some(parse_investment_account(reader)?);
                    }
                    "INVTRANLIST" => {
                        transaction_list = Some(parse_inv_transaction_list(reader)?);
                    }
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "INVSTMTRS" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in INVSTMTRS".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let curdef = curdef.ok_or_else(|| reader.missing_in("INVSTMTRS", "CURDEF"))?;
    let inv_acct = inv_acct.ok_or_else(|| reader.missing_in("INVSTMTRS", "INVACCTFROM"))?;

    let mut stmt = InvStatementResponse::new(curdef, inv_acct);
    if let Some(tl) = transaction_list {
        stmt = stmt.with_transaction_list(tl);
    }

    Ok(stmt)
}

pub(super) fn parse_investment_account(
    reader: &mut OfxReader<'_>,
) -> Result<InvestmentAccount, OfxError> {
    let mut broker_id: Option<String> = None;
    let mut acct_id: Option<crate::types::AccountId> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "BROKERID" => broker_id = Some(read_text(reader, "BROKERID")?),
                    "ACCTID" => {
                        let text = read_text(reader, "ACCTID")?;
                        acct_id = Some(parse_text_as(reader, &text, "ACCTID")?);
                    }
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "INVACCTFROM" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in INVACCTFROM".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let broker_id = broker_id.ok_or_else(|| reader.missing_in("INVACCTFROM", "BROKERID"))?;
    let acct_id = acct_id.ok_or_else(|| reader.missing_in("INVACCTFROM", "ACCTID"))?;

    Ok(InvestmentAccount::new(broker_id, acct_id))
}
