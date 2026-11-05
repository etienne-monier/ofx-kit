//! Errors produced while parsing OFX protocol headers.

use super::ofx_version::InvalidOfxVersion;
use super::security_level::InvalidSecurityLevel;

/// Error returned when parsing an OFX header.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum HeaderError {
    /// No OFX header was found in the input.
    #[error("no OFX header found")]
    Missing,
    /// The OFX processing instruction was malformed.
    #[error("malformed OFX processing instruction: {detail}")]
    MalformedProcessingInstruction {
        /// Description of the malformed processing instruction.
        detail: String,
    },
    /// A required attribute was missing from the header.
    #[error("missing required OFX header attribute: {attribute}")]
    MissingAttribute {
        /// Name of the missing attribute.
        attribute: &'static str,
    },
    /// The VERSION attribute had an invalid value.
    #[error("invalid OFX version in header: '{value}'")]
    InvalidVersion {
        /// Rejected version value.
        value: String,
    },
    /// The SECURITY attribute had an invalid value.
    #[error("invalid OFX security level in header: '{value}'")]
    InvalidSecurity {
        /// Rejected security value.
        value: String,
    },
}

impl From<InvalidOfxVersion> for HeaderError {
    fn from(error: InvalidOfxVersion) -> Self {
        Self::InvalidVersion { value: error.value }
    }
}

impl From<InvalidSecurityLevel> for HeaderError {
    fn from(error: InvalidSecurityLevel) -> Self {
        Self::InvalidSecurity { value: error.value }
    }
}
