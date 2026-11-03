//! Errors produced while parsing OFX protocol headers.

use core::fmt;

/// Error returned when parsing an OFX header.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeaderError {
    /// No OFX header was found in the input.
    Missing,
    /// The OFX processing instruction was malformed.
    MalformedProcessingInstruction {
        /// Description of the malformed processing instruction.
        detail: String,
    },
    /// A required attribute was missing from the header.
    MissingAttribute {
        /// Name of the missing attribute.
        attribute: &'static str,
    },
    /// The VERSION attribute had an invalid value.
    InvalidVersion {
        /// Rejected version value.
        value: String,
    },
    /// The SECURITY attribute had an invalid value.
    InvalidSecurity {
        /// Rejected security value.
        value: String,
    },
}

impl fmt::Display for HeaderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Missing => f.write_str("no OFX header found"),
            Self::MalformedProcessingInstruction { detail } => {
                write!(f, "malformed OFX processing instruction: {detail}")
            }
            Self::MissingAttribute { attribute } => {
                write!(f, "missing required OFX header attribute: {attribute}")
            }
            Self::InvalidVersion { value } => {
                write!(f, "invalid OFX version in header: '{value}'")
            }
            Self::InvalidSecurity { value } => {
                write!(f, "invalid OFX security level in header: '{value}'")
            }
        }
    }
}

impl std::error::Error for HeaderError {}
