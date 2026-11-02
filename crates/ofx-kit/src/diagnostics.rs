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

/// A non-fatal condition encountered while parsing an OFX document.
///
/// Warnings are returned by [`crate::parse_with_warnings`] and
/// [`crate::parse_bytes_with_warnings`]. They never make parsing fail.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseWarning {
    /// An element that is not part of the known OFX subset was encountered.
    UnknownElement { name: String, path: ElementPath },
    /// An element was found in a position where the schema does not allow it.
    MisplacedElement { name: String, path: ElementPath },
    /// A closing tag did not match any open element and was dropped.
    UnmatchedCloseTag { name: String, path: ElementPath },
    /// An element was still open at the end of the input and was closed implicitly.
    UnclosedElement { name: String, path: ElementPath },
    /// A raw `&` was found in text and escaped as `&amp;`.
    RawAmpersand { path: ElementPath },
    /// A named entity that is not valid XML was escaped as literal text.
    UnknownEntity { name: String, path: ElementPath },
    /// The SGML tokenizer found malformed or unterminated input.
    MalformedSgml { message: String, path: ElementPath },
}

impl fmt::Display for ParseWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownElement { name, path } => {
                write!(f, "unknown element <{name}> at {path}")
            }
            Self::MisplacedElement { name, path } => {
                write!(f, "misplaced element <{name}> at {path}")
            }
            Self::UnmatchedCloseTag { name, path } => {
                write!(f, "unmatched closing tag </{name}> at {path}")
            }
            Self::UnclosedElement { name, path } => {
                write!(f, "unclosed element <{name}> at {path}")
            }
            Self::RawAmpersand { path } => {
                write!(f, "raw ampersand escaped at {path}")
            }
            Self::UnknownEntity { name, path } => {
                write!(f, "unknown entity &{name}; escaped as text at {path}")
            }
            Self::MalformedSgml { message, path } => {
                write!(f, "malformed SGML: {message} at {path}")
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
        let warning = ParseWarning::UnknownElement {
            name: "FOO".to_owned(),
            path: ElementPath::from(["OFX", "STMTRS"]),
        };
        assert_eq!(warning.to_string(), "unknown element <FOO> at OFX/STMTRS");
    }
}
