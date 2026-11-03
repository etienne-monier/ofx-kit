//! OFX parser support for the payee domain.
//!
//! This module contains the hand-written aggregate readers for its scope.

use super::*;

pub(super) fn parse_payee(reader: &mut OfxReader<'_>) -> Result<Payee, OfxError> {
    let mut name: Option<String> = None;
    let mut addr1: Option<String> = None;
    let mut addr2: Option<String> = None;
    let mut addr3: Option<String> = None;
    let mut city: Option<String> = None;
    let mut state: Option<String> = None;
    let mut postalcode: Option<String> = None;
    let mut country: Option<String> = None;
    let mut phone: Option<String> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let tag = tag_name(&e);
                match tag.as_str() {
                    "NAME" => name = Some(read_text(reader, "NAME")?),
                    "ADDR1" => addr1 = Some(read_text(reader, "ADDR1")?),
                    "ADDR2" => addr2 = Some(read_text(reader, "ADDR2")?),
                    "ADDR3" => addr3 = Some(read_text(reader, "ADDR3")?),
                    "CITY" => city = Some(read_text(reader, "CITY")?),
                    "STATE" => state = Some(read_text(reader, "STATE")?),
                    "POSTALCODE" => postalcode = Some(read_text(reader, "POSTALCODE")?),
                    "COUNTRY" => country = Some(read_text(reader, "COUNTRY")?),
                    "PHONE" => phone = Some(read_text(reader, "PHONE")?),
                    _ => reader.skip_element(&tag)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "PAYEE" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in PAYEE".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let name = name.ok_or_else(|| reader.missing_in("PAYEE", "NAME"))?;
    let addr1 = addr1.ok_or_else(|| reader.missing_in("PAYEE", "ADDR1"))?;
    let city = city.ok_or_else(|| reader.missing_in("PAYEE", "CITY"))?;
    let state = state.ok_or_else(|| reader.missing_in("PAYEE", "STATE"))?;
    let postalcode = postalcode.ok_or_else(|| reader.missing_in("PAYEE", "POSTALCODE"))?;
    let phone = phone.ok_or_else(|| reader.missing_in("PAYEE", "PHONE"))?;

    let mut payee = Payee::new(name, addr1, city, state, postalcode, phone);
    if let Some(a2) = addr2 {
        payee = payee.with_address2(a2);
    }
    if let Some(a3) = addr3 {
        payee = payee.with_address3(a3);
    }
    if let Some(c) = country {
        payee = payee.with_country(c);
    }
    Ok(payee)
}

// ---------------------------------------------------------------------------
// Currency aggregate
// ---------------------------------------------------------------------------
