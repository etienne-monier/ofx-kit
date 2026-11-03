use crate::aggregates::error::AggregateError;
use crate::diagnostics::ParseWarning;
use crate::header::HeaderError;
use crate::xml::XmlError;

/// Top-level error type for OFX parsing operations.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum OfxError {
    /// The input uses a compression format that the library does not decode.
    #[error("compressed OFX input is not supported")]
    UnsupportedCompression,
    /// Strict parsing rejected a structural warning.
    #[error("strict parsing failed: {0}")]
    StrictViolation(ParseWarning),
    /// An error occurred while parsing the OFX header.
    #[error("header error: {0}")]
    Header(#[from] HeaderError),
    /// An error occurred while parsing XML structure.
    #[error("XML error: {0}")]
    Xml(#[from] XmlError),
    /// An error occurred while parsing an OFX aggregate.
    #[error("aggregate error: {0}")]
    Aggregate(#[from] AggregateError),
}
