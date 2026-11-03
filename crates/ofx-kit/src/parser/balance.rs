//! OFX parser support for the balance domain.
//!
//! This module contains the hand-written aggregate readers for its scope.

use super::*;

pub(super) fn parse_ledger_balance(reader: &mut OfxReader<'_>) -> Result<LedgerBalance, OfxError> {
    let mut balamt: Option<crate::types::OfxAmount> = None;
    let mut dtasof: Option<crate::types::OfxDateTime> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "BALAMT" => {
                        let text = read_text(reader, "BALAMT")?;
                        balamt = Some(parse_text_as(reader, &text, "BALAMT")?);
                    }
                    "DTASOF" => {
                        let text = read_text(reader, "DTASOF")?;
                        dtasof = Some(parse_text_as(reader, &text, "DTASOF")?);
                    }
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "LEDGERBAL" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in LEDGERBAL".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let balamt = balamt.ok_or_else(|| reader.missing_in("LEDGERBAL", "BALAMT"))?;
    let dtasof = dtasof.ok_or_else(|| reader.missing_in("LEDGERBAL", "DTASOF"))?;

    Ok(LedgerBalance::new(balamt, dtasof))
}

pub(super) fn parse_available_balance(
    reader: &mut OfxReader<'_>,
) -> Result<AvailableBalance, OfxError> {
    let mut balamt: Option<crate::types::OfxAmount> = None;
    let mut dtasof: Option<crate::types::OfxDateTime> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "BALAMT" => {
                        let text = read_text(reader, "BALAMT")?;
                        balamt = Some(parse_text_as(reader, &text, "BALAMT")?);
                    }
                    "DTASOF" => {
                        let text = read_text(reader, "DTASOF")?;
                        dtasof = Some(parse_text_as(reader, &text, "DTASOF")?);
                    }
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "AVAILBAL" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in AVAILBAL".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let balamt = balamt.ok_or_else(|| reader.missing_in("AVAILBAL", "BALAMT"))?;
    let dtasof = dtasof.ok_or_else(|| reader.missing_in("AVAILBAL", "DTASOF"))?;

    Ok(AvailableBalance::new(balamt, dtasof))
}

pub(super) fn parse_balance_list(reader: &mut OfxReader<'_>) -> Result<Vec<Balance>, OfxError> {
    let mut balances = Vec::new();

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                if name == "BAL" {
                    balances.push(parse_bal(reader)?);
                } else {
                    reader.skip_element(&name)?;
                }
            }
            Event::End(e) if end_tag_name(&e) == "BALLIST" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in BALLIST".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    Ok(balances)
}

pub(super) fn parse_bal(reader: &mut OfxReader<'_>) -> Result<Balance, OfxError> {
    let mut name: Option<String> = None;
    let mut desc: Option<String> = None;
    let mut bal_type: Option<crate::types::BalanceType> = None;
    let mut value: Option<crate::types::OfxAmount> = None;
    let mut dtasof: Option<crate::types::OfxDateTime> = None;
    let mut currency: Option<crate::types::CurrencyCode> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let tag = tag_name(&e);
                match tag.as_str() {
                    "NAME" => name = Some(read_text(reader, "NAME")?),
                    "DESC" => desc = Some(read_text(reader, "DESC")?),
                    "BALTYPE" => {
                        let text = read_text(reader, "BALTYPE")?;
                        bal_type = Some(parse_text_as(reader, &text, "BALTYPE")?);
                    }
                    "VALUE" => {
                        let text = read_text(reader, "VALUE")?;
                        value = Some(parse_text_as(reader, &text, "VALUE")?);
                    }
                    "DTASOF" => {
                        let text = read_text(reader, "DTASOF")?;
                        dtasof = Some(parse_text_as(reader, &text, "DTASOF")?);
                    }
                    "CURRENCY" => {
                        let text = read_text(reader, "CURRENCY")?;
                        currency = Some(parse_text_as(reader, &text, "CURRENCY")?);
                    }
                    _ => reader.skip_element(&tag)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "BAL" => break,
            Event::Eof => {
                return Err(reader.malformed("unexpected EOF in BAL".to_owned()).into());
            }
            _ => {}
        }
    }

    let name = name.ok_or_else(|| reader.missing_in("BAL", "NAME"))?;
    let desc = desc.ok_or_else(|| reader.missing_in("BAL", "DESC"))?;
    let bal_type = bal_type.ok_or_else(|| reader.missing_in("BAL", "BALTYPE"))?;
    let value = value.ok_or_else(|| reader.missing_in("BAL", "VALUE"))?;

    Ok(Balance::new(name, desc, bal_type, value, dtasof, currency))
}

// ---------------------------------------------------------------------------
// Account aggregates
// ---------------------------------------------------------------------------
