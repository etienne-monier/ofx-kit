//! Element paths and non-fatal diagnostics shared by the parser.

use core::fmt;

/// The path of an element in an OFX document, from the root to the element.
///
/// Paths are rendered with `/` separators, for example
/// `OFX/BANKMSGSRSV1/STMTTRNRS/STMTRS`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct ElementPath {
    segments: Vec<String>,
}

impl ElementPath {
    /// Creates an empty path.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Creates a path from string segments.
    #[must_use]
    pub fn from_segments<I, S>(segments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
        }
    }

    /// Returns a new path with `name` appended.
    #[must_use]
    pub fn child(&self, name: &str) -> Self {
        let mut path = self.clone();
        path.push(name);
        path
    }

    /// Appends an element name to the path.
    pub fn push(&mut self, name: &str) {
        self.segments.push(name.to_owned());
    }

    /// Removes and returns the last element name.
    pub fn pop(&mut self) -> Option<String> {
        self.segments.pop()
    }

    /// Returns the path segments.
    #[must_use]
    pub fn segments(&self) -> &[String] {
        &self.segments
    }

    /// Returns the last path segment, if any.
    #[must_use]
    pub fn last(&self) -> Option<&str> {
        self.segments.last().map(String::as_str)
    }

    /// Returns true when the path has no segments.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }
}

impl fmt::Display for ElementPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, segment) in self.segments.iter().enumerate() {
            if index > 0 {
                f.write_str("/")?;
            }
            f.write_str(segment)?;
        }
        Ok(())
    }
}

impl From<&[&str]> for ElementPath {
    fn from(segments: &[&str]) -> Self {
        Self::from_segments(segments.iter().copied())
    }
}

impl<const N: usize> From<[&str; N]> for ElementPath {
    fn from(segments: [&str; N]) -> Self {
        Self::from_segments(segments)
    }
}

/// A source location using a zero-based byte offset and one-based line and column.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourcePosition {
    /// Zero-based byte offset in the decoded source.
    pub byte_offset: usize,
    /// One-based line number.
    pub line: usize,
    /// One-based column number, measured in bytes.
    pub column: usize,
}

/// Computes a source position for a byte offset in UTF-8 text.
#[must_use]
pub(crate) fn position_at(source: &str, byte_offset: usize) -> SourcePosition {
    let offset = byte_offset.min(source.len());
    let prefix = &source[..offset];
    let line = prefix.bytes().filter(|byte| *byte == b'\n').count() + 1;
    let column = prefix.rsplit('\n').next().map_or(0, str::len) + 1;
    SourcePosition::new(offset, line, column)
}

impl SourcePosition {
    /// Creates a source position.
    #[must_use]
    pub const fn new(byte_offset: usize, line: usize, column: usize) -> Self {
        Self {
            byte_offset,
            line,
            column,
        }
    }
}

/// The kind of a non-fatal condition encountered while parsing an OFX document.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseWarningKind {
    /// An element that is not part of the known OFX subset was encountered.
    UnknownElement {
        /// Element name.
        name: String,
    },
    /// An element was found in a position where the schema does not allow it.
    MisplacedElement {
        /// Element name.
        name: String,
    },
    /// A closing tag did not match any open element and was dropped.
    UnmatchedCloseTag {
        /// Closing-tag name.
        name: String,
    },
    /// An element was still open at the end of the input and was closed implicitly.
    UnclosedElement {
        /// Element name.
        name: String,
    },
    /// An unknown element contains text before a nested element, making its
    /// SGML leaf/aggregate interpretation ambiguous.
    AmbiguousUnknownElement {
        /// Element name.
        name: String,
    },
    /// A raw `&` was found in text and escaped as `&amp;`.
    RawAmpersand,
    /// A named entity that is not valid XML was escaped as literal text.
    UnknownEntity {
        /// Entity name.
        name: String,
    },
    /// The SGML tokenizer found malformed or unterminated input.
    MalformedSgml {
        /// Description of the malformed input.
        message: String,
    },
}

/// A non-fatal condition encountered while parsing an OFX document.
///
/// Warnings are returned by [`crate::parse_with_warnings`] and
/// [`crate::parse_bytes_with_warnings`]. They never make parsing fail.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseWarning {
    /// The warning classification and associated data.
    pub kind: ParseWarningKind,
    /// The OFX element path at which the warning occurred.
    pub path: ElementPath,
    /// The source position, when available.
    pub position: Option<SourcePosition>,
}

impl ParseWarning {
    /// Creates a warning without source-position information.
    #[must_use]
    pub fn new(kind: ParseWarningKind, path: ElementPath) -> Self {
        Self {
            kind,
            path,
            position: None,
        }
    }

    /// Creates a warning with source-position information.
    #[must_use]
    pub fn at(kind: ParseWarningKind, path: ElementPath, position: SourcePosition) -> Self {
        Self {
            kind,
            path,
            position: Some(position),
        }
    }

    /// Returns the warning kind.
    #[must_use]
    pub const fn kind(&self) -> &ParseWarningKind {
        &self.kind
    }

    /// Returns the element path.
    #[must_use]
    pub const fn path(&self) -> &ElementPath {
        &self.path
    }

    /// Returns the source position, if known.
    #[must_use]
    pub const fn position(&self) -> Option<SourcePosition> {
        self.position
    }
}

impl fmt::Display for ParseWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ParseWarningKind::UnknownElement { name } => {
                write!(f, "unknown element <{name}> at {}", self.path)
            }
            ParseWarningKind::MisplacedElement { name } => {
                write!(f, "misplaced element <{name}> at {}", self.path)
            }
            ParseWarningKind::UnmatchedCloseTag { name } => {
                write!(f, "unmatched closing tag </{name}> at {}", self.path)
            }
            ParseWarningKind::UnclosedElement { name } => {
                write!(f, "unclosed element <{name}> at {}", self.path)
            }
            ParseWarningKind::AmbiguousUnknownElement { name } => {
                write!(f, "ambiguous unknown element <{name}> at {}", self.path)
            }
            ParseWarningKind::RawAmpersand => {
                write!(f, "raw ampersand escaped at {}", self.path)
            }
            ParseWarningKind::UnknownEntity { name } => {
                write!(
                    f,
                    "unknown entity &{name}; escaped as text at {}",
                    self.path
                )
            }
            ParseWarningKind::MalformedSgml { message } => {
                write!(f, "malformed SGML: {message} at {}", self.path)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renders_paths() {
        let path = ElementPath::from(["OFX", "BANKMSGSRSV1"]).child("STMTTRNRS");
        assert_eq!(path.to_string(), "OFX/BANKMSGSRSV1/STMTTRNRS");
        assert_eq!(path.last(), Some("STMTTRNRS"));
    }

    #[test]
    fn warning_messages_include_paths() {
        let warning = ParseWarning::new(
            ParseWarningKind::UnknownElement {
                name: "FOO".to_owned(),
            },
            ElementPath::from(["OFX", "STMTRS"]),
        );
        assert_eq!(warning.to_string(), "unknown element <FOO> at OFX/STMTRS");
    }
}
