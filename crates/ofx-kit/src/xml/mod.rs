//! XML parsing infrastructure for OFX documents.

pub mod helpers;
pub mod reader;

use core::fmt;

use crate::diagnostics::{ElementPath, SourcePosition};

pub use reader::OfxReader;

/// The reason an XML-level parsing operation failed.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum XmlErrorKind {
    /// The XML document was malformed.
    MalformedXml { message: String },
    /// An unexpected element was encountered.
    UnexpectedElement { expected: String, found: String },
    /// A required element was missing from its parent.
    MissingElement { parent: String, element: String },
    /// An element outside the supported OFX subset was encountered.
    UnknownElement { name: String },
    /// An element contained invalid text content.
    InvalidContent {
        element: String,
        value: String,
        reason: String,
    },
}

/// Error type for XML-level parsing failures.
///
/// Every error carries the [`ElementPath`] at which it occurred.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XmlError {
    kind: XmlErrorKind,
    path: ElementPath,
    position: Option<SourcePosition>,
}

impl XmlError {
    /// Creates an XML error with its location.
    #[must_use]
    pub fn new(kind: XmlErrorKind, path: ElementPath) -> Self {
        Self {
            kind,
            path,
            position: None,
        }
    }

    /// Returns the error kind.
    #[must_use]
    pub fn kind(&self) -> &XmlErrorKind {
        &self.kind
    }

    /// Returns the path at which the error occurred.
    #[must_use]
    pub fn path(&self) -> &ElementPath {
        &self.path
    }

    /// Returns the source position, if known.
    #[must_use]
    pub const fn position(&self) -> Option<SourcePosition> {
        self.position
    }

    /// Adds source-position information to this error.
    #[must_use]
    pub fn with_position(mut self, position: SourcePosition) -> Self {
        self.position = Some(position);
        self
    }

    pub(crate) fn malformed(path: ElementPath, message: impl Into<String>) -> Self {
        Self::new(
            XmlErrorKind::MalformedXml {
                message: message.into(),
            },
            path,
        )
    }

    pub(crate) fn unexpected(
        path: ElementPath,
        expected: impl Into<String>,
        found: impl Into<String>,
    ) -> Self {
        Self::new(
            XmlErrorKind::UnexpectedElement {
                expected: expected.into(),
                found: found.into(),
            },
            path,
        )
    }

    pub(crate) fn missing(
        path: ElementPath,
        parent: impl Into<String>,
        element: impl Into<String>,
    ) -> Self {
        Self::new(
            XmlErrorKind::MissingElement {
                parent: parent.into(),
                element: element.into(),
            },
            path,
        )
    }

    pub(crate) fn invalid_content(
        path: ElementPath,
        element: impl Into<String>,
        value: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self::new(
            XmlErrorKind::InvalidContent {
                element: element.into(),
                value: value.into(),
                reason: reason.into(),
            },
            path,
        )
    }
}

impl fmt::Display for XmlErrorKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MalformedXml { message } => {
                write!(f, "malformed XML: {message}")
            }
            Self::UnexpectedElement { expected, found } => {
                write!(f, "expected element <{expected}>, found <{found}>")
            }
            Self::MissingElement { parent, element } => {
                write!(f, "missing required element <{element}> in <{parent}>")
            }
            Self::UnknownElement { name } => write!(f, "unknown element <{name}>"),
            Self::InvalidContent {
                element,
                value,
                reason,
            } => {
                write!(f, "invalid content in <{element}>: '{value}' ({reason})")
            }
        }
    }
}

impl fmt::Display for XmlError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.path.is_empty() {
            write!(f, "{}", self.kind)
        } else {
            write!(f, "{} (at {})", self.kind, self.path)
        }
    }
}

impl std::error::Error for XmlError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_includes_path() {
        let error = XmlError::invalid_content(
            ElementPath::from(["OFX", "STMTRS", "CURDEF"]),
            "CURDEF",
            "EURO",
            "unsupported currency",
        );
        assert_eq!(
            error.to_string(),
            "invalid content in <CURDEF>: 'EURO' (unsupported currency) \
             (at OFX/STMTRS/CURDEF)"
        );
    }

    #[test]
    fn error_without_path_has_no_suffix() {
        let error = XmlError::malformed(ElementPath::new(), "boom");
        assert_eq!(error.to_string(), "malformed XML: boom");
    }
}
