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
    let mut payee = Payee::new(name);
    if let Some(value) = addr1 {
        payee = payee.with_address1(value);
    }
    if let Some(a2) = addr2 {
        payee = payee.with_address2(a2);
    }
    if let Some(a3) = addr3 {
        payee = payee.with_address3(a3);
    }
    if let Some(c) = country {
        payee = payee.with_country(c);
    }
    if let Some(value) = city {
        payee = payee.with_city(value);
    }
    if let Some(value) = state {
        payee = payee.with_state(value);
    }
    if let Some(value) = postalcode {
        payee = payee.with_postal_code(value);
    }
    if let Some(value) = phone {
        payee = payee.with_phone(value);
    }
    Ok(payee)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_payee_with_only_name() {
        let mut reader = OfxReader::new("<PAYEE><NAME>ACME</NAME></PAYEE>");
        let _ = reader.next_event().expect("PAYEE start should parse");

        let payee = parse_payee(&mut reader).expect("minimal PAYEE should parse");

        assert_eq!(payee.name(), "ACME");
        assert_eq!(payee.address1(), None);
        assert_eq!(payee.city(), None);
        assert_eq!(payee.phone(), None);
    }

    #[test]
    fn requires_payee_name() {
        let mut reader = OfxReader::new("<PAYEE><CITY>Paris</CITY></PAYEE>");
        let _ = reader.next_event().expect("PAYEE start should parse");

        let error = parse_payee(&mut reader).expect_err("PAYEE without NAME must fail");

        assert!(
            error
                .to_string()
                .contains("missing required element <NAME> in <PAYEE>")
        );
    }
}

// ---------------------------------------------------------------------------
// Currency aggregate
// ---------------------------------------------------------------------------
