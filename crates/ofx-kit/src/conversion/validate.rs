//! OFX conversion validate support.
//!
//! This private module contains the structural conversion helpers for its scope.

use super::*;

pub(super) fn validate_xml(xml: &str) -> Result<(), ConversionError> {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => return Ok(()),
            Ok(_) => {}
            Err(error) => return Err(ConversionError::InvalidXml(error.to_string())),
        }
    }
}
