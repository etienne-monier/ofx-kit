//! OFX parser support for the currency domain.
//!
//! This module contains the hand-written aggregate readers for its scope.

use super::*;

pub(super) fn parse_currency(
    reader: &mut OfxReader<'_>,
    closing_tag: &str,
) -> Result<CurrencyInfo, OfxError> {
    let mut currate: Option<crate::types::OfxAmount> = None;
    let mut cursym: Option<crate::types::CurrencyCode> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "CURRATE" => {
                        let text = read_text(reader, "CURRATE")?;
                        currate = Some(parse_text_as(reader, &text, "CURRATE")?);
                    }
                    "CURSYM" => {
                        let text = read_text(reader, "CURSYM")?;
                        cursym = Some(parse_text_as(reader, &text, "CURSYM")?);
                    }
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

    let currate = currate.ok_or_else(|| reader.missing_in(closing_tag, "CURRATE"))?;
    let cursym = cursym.ok_or_else(|| reader.missing_in(closing_tag, "CURSYM"))?;

    Ok(match closing_tag {
        "ORIGCURRENCY" => CurrencyInfo::OrigCurrency {
            code: cursym,
            rate: currate,
        },
        _ => CurrencyInfo::Currency {
            code: cursym,
            rate: currate,
        },
    })
}

// ---------------------------------------------------------------------------
// Credit card message set
// ---------------------------------------------------------------------------
