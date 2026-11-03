//! Lossless structural conversion of OFX 1.x SGML to XML.

use core::fmt;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::diagnostics::ParseWarning;
use crate::encoding::{DecodeWarning, DetectedEncoding, decode};
use crate::header::{HeaderError, OfxVersion, parse_header};
use crate::sgml::normalize_sgml_to_xml_with_warnings;

mod header;
mod pretty;
mod validate;

use header::*;
use pretty::*;
use validate::*;

/// A non-fatal condition encountered while converting an OFX source.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionWarning {
    /// A source header field was not copied to the UTF-8 XML header.
    DroppedHeaderField { name: String, value: String },
    /// A warning emitted while normalizing the SGML body.
    Parse(ParseWarning),
}

impl fmt::Display for ConversionWarning {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DroppedHeaderField { name, value } => write!(
                f,
                "source header field {name}={value} was not copied to UTF-8 XML output"
            ),
            Self::Parse(warning) => warning.fmt(f),
        }
    }
}

/// Options controlling an OFX conversion.
#[derive(Debug, Clone, Default)]
pub struct ConversionOptions {
    /// Replace the source VERSION metadata with this version.
    pub target_version: Option<OfxVersion>,
    /// Indent the generated XML.
    pub pretty: bool,
}

/// The result of a structural conversion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConvertedSource {
    /// UTF-8 XML output.
    pub xml: String,
    /// Encoding detected for the input.
    pub encoding: DetectedEncoding,
    /// Warnings emitted while decoding the input.
    pub decode_warnings: Vec<DecodeWarning>,
    /// Warnings emitted while converting the source.
    pub warnings: Vec<ConversionWarning>,
}

/// An error produced while converting an OFX source.
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConversionError {
    /// The OFX header could not be read.
    Header(HeaderError),
    /// The input declares compression that this crate cannot decode.
    UnsupportedCompression,
    /// The generated document is not well-formed XML.
    InvalidXml(String),
}

impl fmt::Display for ConversionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Header(error) => write!(f, "header error: {error}"),
            Self::UnsupportedCompression => f.write_str("compressed OFX input is not supported"),
            Self::InvalidXml(error) => write!(f, "generated XML is invalid: {error}"),
        }
    }
}

impl std::error::Error for ConversionError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Header(error) => Some(error),
            Self::UnsupportedCompression | Self::InvalidXml(_) => None,
        }
    }
}

impl From<HeaderError> for ConversionError {
    fn from(error: HeaderError) -> Self {
        Self::Header(error)
    }
}

/// Convert an OFX source from bytes to UTF-8 XML.
pub fn convert_bytes(
    input: &[u8],
    options: &ConversionOptions,
) -> Result<ConvertedSource, ConversionError> {
    let decoded = decode(input);
    if decoded
        .warnings
        .iter()
        .any(|warning| matches!(warning, DecodeWarning::CompressedInput))
    {
        return Err(ConversionError::UnsupportedCompression);
    }
    convert_decoded(&decoded.text, decoded.encoding, decoded.warnings, options)
}

/// Convert an already decoded OFX source to UTF-8 XML.
pub fn convert_str(
    input: &str,
    options: &ConversionOptions,
) -> Result<ConvertedSource, ConversionError> {
    convert_decoded(input, DetectedEncoding::Utf8, Vec::new(), options)
}

fn convert_decoded(
    input: &str,
    encoding: DetectedEncoding,
    decode_warnings: Vec<DecodeWarning>,
    options: &ConversionOptions,
) -> Result<ConvertedSource, ConversionError> {
    let (header, body) = parse_header(input)?;
    if header
        .compression()
        .is_some_and(|compression| !compression.eq_ignore_ascii_case("NONE"))
    {
        return Err(ConversionError::UnsupportedCompression);
    }

    let body = strip_xml_declaration(body);
    let mut warnings = header_warnings(&header);
    let (xml, mut body_warnings) = if header.version().major() < 2 {
        let normalized = normalize_sgml_to_xml_with_warnings(body.trim());
        let version = options.target_version.unwrap_or(header.version());
        let header_xml = xml_header(&header, version);
        (
            format!("{header_xml}\n{}", normalized.xml),
            normalized.warnings,
        )
    } else {
        let version = options.target_version.unwrap_or(header.version());
        let header_xml = xml_header(&header, version);
        (format!("{header_xml}{}", body.trim_start()), Vec::new())
    };
    warnings.extend(body_warnings.drain(..).map(ConversionWarning::Parse));

    let mut xml = if options.pretty {
        pretty_xml(&xml)?
    } else {
        xml
    };
    if !xml.ends_with('\n') {
        xml.push('\n');
    }
    validate_xml(&xml)?;

    Ok(ConvertedSource {
        xml,
        encoding,
        decode_warnings,
        warnings,
    })
}
