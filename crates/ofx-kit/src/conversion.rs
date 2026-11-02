//! Lossless structural conversion of OFX 1.x SGML to XML.

use core::fmt;

use quick_xml::Reader;
use quick_xml::events::Event;

use crate::diagnostics::ParseWarning;
use crate::encoding::{DecodeWarning, DetectedEncoding, decode};
use crate::header::{HeaderError, OfxVersion, parse_header};
use crate::sgml::normalize_sgml_to_xml_with_warnings;

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

fn header_warnings(header: &crate::header::OfxHeader) -> Vec<ConversionWarning> {
    let mut warnings = Vec::new();
    if let Some(value) = header.encoding() {
        warnings.push(ConversionWarning::DroppedHeaderField {
            name: "ENCODING".to_owned(),
            value: value.to_owned(),
        });
    }
    if let Some(value) = header.charset() {
        warnings.push(ConversionWarning::DroppedHeaderField {
            name: "CHARSET".to_owned(),
            value: value.to_owned(),
        });
    }
    for (name, value) in header.unknown_fields() {
        if !matches!(name.as_str(), "DATA" | "OFXHEADER") {
            warnings.push(ConversionWarning::DroppedHeaderField {
                name: name.clone(),
                value: value.clone(),
            });
        }
    }
    warnings
}

fn xml_header(header: &crate::header::OfxHeader, version: OfxVersion) -> String {
    format!(
        "<?OFX OFXHEADER=\"200\" VERSION=\"{version}\" SECURITY=\"{}\" OLDFILEUID=\"{}\" NEWFILEUID=\"{}\"?>",
        header.security(),
        escape_attribute(header.old_file_uid()),
        escape_attribute(header.new_file_uid()),
    )
}

fn escape_attribute(value: &str) -> String {
    value.replace('&', "&amp;").replace('"', "&quot;")
}

fn strip_xml_declaration(input: &str) -> &str {
    let input = input.trim_start();
    if !input.starts_with("<?xml") {
        return input;
    }
    input.find("?>").map_or(input, |end| &input[end + 2..])
}

fn validate_xml(xml: &str) -> Result<(), ConversionError> {
    let mut reader = Reader::from_str(xml);
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => return Ok(()),
            Ok(_) => {}
            Err(error) => return Err(ConversionError::InvalidXml(error.to_string())),
        }
    }
}

fn pretty_xml(xml: &str) -> Result<String, ConversionError> {
    let mut reader = Reader::from_str(xml);
    let mut output = String::new();
    let mut depth = 0usize;
    let mut line_start = true;
    let mut text_content = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Eof) => break,
            Ok(Event::Decl(event)) => {
                write_indent(&mut output, depth, &mut line_start);
                output.push_str(&String::from_utf8_lossy(event.as_ref()));
                output.push('\n');
                line_start = true;
            }
            Ok(Event::PI(event)) => {
                write_indent(&mut output, depth, &mut line_start);
                output.push_str("<?");
                output.push_str(&String::from_utf8_lossy(event.as_ref()));
                output.push_str("?>");
                output.push('\n');
                line_start = true;
            }
            Ok(Event::Start(event)) => {
                let parent_has_text = text_content.last().copied().unwrap_or(false);
                if !line_start && !parent_has_text {
                    output.push('\n');
                    line_start = true;
                }
                write_indent(&mut output, depth, &mut line_start);
                output.push('<');
                output.push_str(&String::from_utf8_lossy(event.name().as_ref()));
                for attribute in event.attributes().flatten() {
                    output.push(' ');
                    output.push_str(&String::from_utf8_lossy(attribute.key.as_ref()));
                    output.push_str("=\"");
                    output.push_str(&String::from_utf8_lossy(&attribute.value));
                    output.push('"');
                }
                output.push('>');
                depth += 1;
                text_content.push(false);
            }
            Ok(Event::End(event)) => {
                depth = depth.saturating_sub(1);
                let has_text = text_content.pop().unwrap_or(false);
                if line_start && !has_text {
                    write_indent(&mut output, depth, &mut line_start);
                }
                output.push_str("</");
                output.push_str(&String::from_utf8_lossy(event.name().as_ref()));
                output.push('>');
                if text_content.last().copied().unwrap_or(false) {
                    line_start = false;
                } else {
                    output.push('\n');
                    line_start = true;
                }
            }
            Ok(Event::Empty(event)) => {
                let parent_has_text = text_content.last().copied().unwrap_or(false);
                if !line_start && !parent_has_text {
                    output.push('\n');
                    line_start = true;
                }
                write_indent(&mut output, depth, &mut line_start);
                output.push('<');
                output.push_str(&String::from_utf8_lossy(event.name().as_ref()));
                output.push_str("/>");
                if parent_has_text {
                    line_start = false;
                } else {
                    output.push('\n');
                    line_start = true;
                }
            }
            Ok(Event::Text(event)) => {
                let text = String::from_utf8_lossy(event.as_ref());
                if let Some(has_text) = text_content.last_mut() {
                    *has_text |= !text.trim().is_empty();
                }
                write_indent(&mut output, depth, &mut line_start);
                output.push_str(&text);
                line_start = text.ends_with('\n');
            }
            Ok(Event::Comment(event)) => {
                let parent_has_text = text_content.last().copied().unwrap_or(false);
                if !line_start && !parent_has_text {
                    output.push('\n');
                    line_start = true;
                }
                write_indent(&mut output, depth, &mut line_start);
                output.push_str("<!--");
                output.push_str(&String::from_utf8_lossy(&event));
                output.push_str("-->");
                if parent_has_text {
                    line_start = false;
                } else {
                    output.push('\n');
                    line_start = true;
                }
            }
            Ok(Event::CData(event)) => {
                let parent_has_text = text_content.last().copied().unwrap_or(false);
                if !line_start && !parent_has_text {
                    output.push('\n');
                    line_start = true;
                }
                write_indent(&mut output, depth, &mut line_start);
                output.push_str("<![CDATA[");
                output.push_str(&String::from_utf8_lossy(&event));
                output.push_str("]]>");
                if let Some(has_text) = text_content.last_mut() {
                    *has_text = true;
                }
            }
            Ok(Event::GeneralRef(event)) => {
                write_indent(&mut output, depth, &mut line_start);
                output.push('&');
                output.push_str(&String::from_utf8_lossy(&event));
                output.push(';');
                if let Some(has_text) = text_content.last_mut() {
                    *has_text = true;
                }
            }
            Ok(Event::DocType(event)) => {
                if !line_start {
                    output.push('\n');
                    line_start = true;
                }
                write_indent(&mut output, depth, &mut line_start);
                output.push_str("<!DOCTYPE ");
                output.push_str(&String::from_utf8_lossy(&event));
                output.push_str(">\n");
                line_start = true;
            }
            Err(error) => return Err(ConversionError::InvalidXml(error.to_string())),
        }
    }
    Ok(output)
}

fn write_indent(output: &mut String, depth: usize, line_start: &mut bool) {
    if *line_start {
        output.push_str(&"  ".repeat(depth));
        *line_start = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_sgml_and_preserves_unknown_elements() {
        let result = convert_str(
            "OFXHEADER:100\nVERSION:102\nSECURITY:NONE\n\n<OFX><UNKNOWN>CAFÉ</UNKNOWN><CODE>0",
            &ConversionOptions::default(),
        )
        .expect("conversion should succeed");
        assert!(
            result
                .xml
                .starts_with("<?OFX OFXHEADER=\"200\" VERSION=\"102\"")
        );
        assert!(result.xml.contains("<UNKNOWN>CAFÉ</UNKNOWN>"));
        assert!(result.xml.contains("<CODE>0</CODE>"));
    }

    #[test]
    fn target_version_changes_only_header_metadata() {
        let source = "OFXHEADER:100\nVERSION:102\nSECURITY:NONE\n\n<OFX><A>1";
        let version = "220".parse().expect("valid version");
        let result = convert_str(
            source,
            &ConversionOptions {
                target_version: Some(version),
                pretty: false,
            },
        )
        .expect("conversion should succeed");
        assert!(
            result
                .xml
                .starts_with("<?OFX OFXHEADER=\"200\" VERSION=\"220\"")
        );
        assert!(result.xml.contains("<A>1</A>"));
    }

    #[test]
    fn strips_xml_declaration_from_the_body() {
        let body = "  <?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<OFX>";
        assert_eq!(strip_xml_declaration(body), "\n<OFX>");
    }

    #[test]
    fn pretty_print_preserves_unknown_elements_and_entities() {
        let source = "OFXHEADER:100\nVERSION:102\nSECURITY:NONE\n\n<OFX><UNKNOWN><CHILD>A&amp;B</CHILD></UNKNOWN>";
        let result = convert_str(
            source,
            &ConversionOptions {
                target_version: None,
                pretty: true,
            },
        )
        .expect("conversion should succeed");
        assert!(
            result
                .xml
                .contains("<UNKNOWN>\n    <CHILD>A&amp;B</CHILD>\n  </UNKNOWN>"),
            "{}",
            result.xml
        );
    }

    #[test]
    fn converts_xml_input_without_duplicate_xml_declaration() {
        let source = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<?OFX OFXHEADER=\"200\" VERSION=\"220\" SECURITY=\"NONE\" OLDFILEUID=\"NONE\" NEWFILEUID=\"NONE\"?>\n<OFX><A>1</A></OFX>";
        let result =
            convert_str(source, &ConversionOptions::default()).expect("conversion should succeed");
        assert_eq!(result.xml.matches("<?xml").count(), 0);
        assert_eq!(result.xml.matches("<?OFX").count(), 1);
    }
}
