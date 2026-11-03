//! OFX conversion pretty support.
//!
//! This private module contains the structural conversion helpers for its scope.

use super::*;

pub(super) fn pretty_xml(xml: &str) -> Result<String, ConversionError> {
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

pub(super) fn write_indent(output: &mut String, depth: usize, line_start: &mut bool) {
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
