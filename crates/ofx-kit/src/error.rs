use core::fmt;

use crate::aggregates::error::AggregateError;
use crate::header::HeaderError;
use crate::xml::XmlError;

/// Top-level error type for OFX parsing operations.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OfxError {
    /// The input uses a compression format that the library does not decode.
    UnsupportedCompression,
    /// An error occurred while parsing the OFX header.
    Header(HeaderError),
    /// An error occurred while parsing XML structure.
    Xml(XmlError),
    /// An error occurred while parsing an OFX aggregate.
    Aggregate(AggregateError),
}

impl fmt::Display for OfxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedCompression => f.write_str("compressed OFX input is not supported"),
            Self::Header(e) => write!(f, "header error: {e}"),
            Self::Xml(e) => write!(f, "XML error: {e}"),
            Self::Aggregate(e) => write!(f, "aggregate error: {e}"),
        }
    }
}

impl std::error::Error for OfxError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::UnsupportedCompression => None,
            Self::Header(e) => Some(e),
            Self::Xml(e) => Some(e),
            Self::Aggregate(e) => Some(e),
        }
    }
}

impl From<HeaderError> for OfxError {
    fn from(e: HeaderError) -> Self {
        Self::Header(e)
    }
}

impl From<XmlError> for OfxError {
    fn from(e: XmlError) -> Self {
        Self::Xml(e)
    }
}

impl From<AggregateError> for OfxError {
    fn from(e: AggregateError) -> Self {
        Self::Aggregate(e)
    }
}
