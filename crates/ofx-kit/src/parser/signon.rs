//! OFX parser support for the signon domain.
//!
//! This module contains the hand-written aggregate readers for its scope.

use super::*;

pub(super) fn parse_signon_message_set(
    reader: &mut OfxReader<'_>,
) -> Result<SignonResponse, OfxError> {
    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                if name == "SONRS" {
                    let sonrs = parse_sonrs(reader)?;
                    reader.skip_to_end("SIGNONMSGSRSV1")?;
                    return Ok(sonrs);
                }
                reader.skip_element(&name)?;
            }
            Event::End(e) if end_tag_name(&e) == "SIGNONMSGSRSV1" => {
                return Err(reader.missing_in("SIGNONMSGSRSV1", "SONRS").into());
            }
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in SIGNONMSGSRSV1".to_owned())
                    .into());
            }
            _ => {}
        }
    }
}

pub(super) fn parse_sonrs(reader: &mut OfxReader<'_>) -> Result<SignonResponse, OfxError> {
    let mut status: Option<Status> = None;
    let mut dtserver: Option<crate::types::OfxDateTime> = None;
    let mut language: Option<String> = None;
    let mut fi_org: Option<String> = None;
    let mut fi_id: Option<String> = None;
    let mut session_cookie: Option<String> = None;
    let mut access_key: Option<String> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "STATUS" => status = Some(parse_status(reader)?),
                    "DTSERVER" => {
                        let text = read_text(reader, "DTSERVER")?;
                        dtserver = Some(parse_text_as(reader, &text, "DTSERVER")?);
                    }
                    "LANGUAGE" => language = Some(read_text(reader, "LANGUAGE")?),
                    "FI" => {
                        let (org, id) = parse_fi(reader)?;
                        fi_org = org;
                        fi_id = id;
                    }
                    "SESSCOOKIE" => session_cookie = Some(read_text(reader, "SESSCOOKIE")?),
                    "ACCESSKEY" => access_key = Some(read_text(reader, "ACCESSKEY")?),
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "SONRS" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in SONRS".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let status = status.ok_or_else(|| reader.missing_in("SONRS", "STATUS"))?;
    let dtserver = dtserver.ok_or_else(|| reader.missing_in("SONRS", "DTSERVER"))?;
    let language = language.unwrap_or_else(|| "ENG".to_owned());

    let mut sonrs = SignonResponse::new(status, dtserver, language);
    if let Some(org) = fi_org {
        sonrs = sonrs.with_fi_org(org);
    }
    if let Some(id) = fi_id {
        sonrs = sonrs.with_fi_id(id);
    }
    if let Some(cookie) = session_cookie {
        sonrs = sonrs.with_session_cookie(cookie);
    }
    if let Some(key) = access_key {
        sonrs = sonrs.with_access_key(key);
    }
    Ok(sonrs)
}

pub(super) fn parse_fi(
    reader: &mut OfxReader<'_>,
) -> Result<(Option<String>, Option<String>), OfxError> {
    let mut org: Option<String> = None;
    let mut fid: Option<String> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "ORG" => org = Some(read_text(reader, "ORG")?),
                    "FID" => fid = Some(read_text(reader, "FID")?),
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "FI" => break,
            Event::Eof => {
                return Err(reader.malformed("unexpected EOF in FI".to_owned()).into());
            }
            _ => {}
        }
    }
    Ok((org, fid))
}

// ---------------------------------------------------------------------------
// STATUS aggregate (used everywhere)
// ---------------------------------------------------------------------------

pub(super) fn parse_status(reader: &mut OfxReader<'_>) -> Result<Status, OfxError> {
    let mut code: Option<u32> = None;
    let mut severity: Option<crate::types::Severity> = None;
    let mut message: Option<String> = None;

    loop {
        match reader.next_event()? {
            Event::Start(e) => {
                let name = tag_name(&e);
                match name.as_str() {
                    "CODE" => {
                        let text = read_text(reader, "CODE")?;
                        code = Some(parse_text_as(reader, &text, "CODE")?);
                    }
                    "SEVERITY" => {
                        let text = read_text(reader, "SEVERITY")?;
                        severity = Some(parse_text_as(reader, &text, "SEVERITY")?);
                    }
                    "MESSAGE" => message = Some(read_text(reader, "MESSAGE")?),
                    _ => reader.skip_element(&name)?,
                }
            }
            Event::End(e) if end_tag_name(&e) == "STATUS" => break,
            Event::Eof => {
                return Err(reader
                    .malformed("unexpected EOF in STATUS".to_owned())
                    .into());
            }
            _ => {}
        }
    }

    let code = code.ok_or_else(|| reader.missing_in("STATUS", "CODE"))?;
    let severity = severity.ok_or_else(|| reader.missing_in("STATUS", "SEVERITY"))?;

    Ok(Status::new(code, severity, message))
}

// ---------------------------------------------------------------------------
// Banking message set
// ---------------------------------------------------------------------------
