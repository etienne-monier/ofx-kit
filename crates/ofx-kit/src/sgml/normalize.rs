//! SGML-to-XML normalization for OFX 1.x documents.
//!
//! OFX 1.x uses SGML, where closing tags are optional for both leaves and
//! aggregates. For example:
//!
//! ```text
//! <STATUS>
//! <CODE>0
//! <SEVERITY>INFO
//! <DTSERVER>20240402
//! ```
//!
//! Here, `<CODE>` and `<SEVERITY>` are leaves closed implicitly, and `<STATUS>`
//! is an aggregate closed implicitly when `<DTSERVER>` (a non-child) appears.
//!
//! Normalization is deliberately conservative: a closing tag is inferred only
//! when the shared schema proves that the open element cannot contain the next
//! one. Unknown elements are preserved and, when they contain child elements,
//! are only closed by an explicit closing tag or by the end of the input.

use crate::diagnostics::{ElementPath, ParseWarning, ParseWarningKind, position_at};
use crate::schema::{NodeKind, Schema, schema};

use super::tokenizer::{EntityKind, Token, TokenizeWarning, Tokenizer};

/// Normalized XML plus the non-fatal problems found while normalizing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NormalizedSgml {
    pub(crate) xml: String,
    pub(crate) warnings: Vec<ParseWarning>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FrameKind {
    Aggregate,
    Leaf,
    /// An unknown element followed by another element.
    UnknownAggregate,
    /// An unknown element holding text or closed explicitly.
    UnknownLeaf,
}

#[derive(Debug, Clone)]
struct Frame {
    name: String,
    kind: FrameKind,
}

impl Frame {
    fn is_leaf(&self) -> bool {
        matches!(self.kind, FrameKind::Leaf | FrameKind::UnknownLeaf)
    }
}

/// Converts OFX SGML content into well-formed XML.
///
/// This wrapper is only used by tests; callers that need diagnostics use
/// [`normalize_sgml_to_xml_with_warnings`].
#[cfg(test)]
pub(crate) fn normalize_sgml_to_xml(input: &str) -> String {
    normalize_sgml_to_xml_with_warnings(input).xml
}

/// Converts OFX SGML content into well-formed XML and collects warnings.
///
/// Text is preserved verbatim. Only entity references are rewritten when XML
/// requires it.
pub(crate) fn normalize_sgml_to_xml_with_warnings(input: &str) -> NormalizedSgml {
    let mut tokenizer = Tokenizer::new(input);
    let mut tokens = Vec::new();
    let mut warning_starts = Vec::new();

    loop {
        let start = tokenizer.warnings().len();
        let Some(token) = tokenizer.next() else {
            break;
        };
        tokens.push(token);
        warning_starts.push(start);
    }

    let tokenizer_warnings = tokenizer.warnings();
    let mut output = String::with_capacity(input.len() + input.len() / 4);
    let mut stack: Vec<Frame> = Vec::new();
    let mut warnings = Vec::new();

    for (index, token) in tokens.iter().enumerate() {
        let warning_path = current_path(&stack);

        match token {
            Token::Text { text, .. } => push_escaped_text(&mut output, text),
            Token::Entity {
                raw,
                name,
                kind,
                span,
            } => match kind {
                EntityKind::Xml | EntityKind::Numeric => output.push_str(raw),
                EntityKind::Unknown => {
                    output.push_str("&amp;");
                    output.push_str(name);
                    output.push(';');
                    warnings.push(ParseWarning::at(
                        ParseWarningKind::UnknownEntity {
                            name: (*name).to_owned(),
                        },
                        warning_path.clone(),
                        position_at(input, span.start),
                    ));
                }
                EntityKind::Raw => {
                    output.push_str("&amp;");
                    warnings.push(ParseWarning::at(
                        ParseWarningKind::RawAmpersand,
                        warning_path.clone(),
                        position_at(input, span.start),
                    ));
                }
            },
            Token::Comment { raw, .. }
            | Token::ProcessingInstruction { raw, .. }
            | Token::Declaration { raw, .. } => output.push_str(raw),
            Token::Malformed { raw, .. } => push_escaped_text(&mut output, raw),
            Token::CloseTag { name, span } => {
                handle_close_tag(
                    name,
                    &mut stack,
                    &mut output,
                    &mut warnings,
                    &warning_path,
                    position_at(input, span.start),
                );
            }
            Token::OpenTag {
                name,
                raw,
                self_closing,
                span,
            } => {
                handle_open_tag(
                    name,
                    raw,
                    *self_closing,
                    &tokens,
                    index,
                    &mut stack,
                    &mut output,
                    &mut warnings,
                    &warning_path,
                    position_at(input, span.start),
                );
            }
        }

        let warning_end = warning_starts
            .get(index + 1)
            .copied()
            .unwrap_or(tokenizer_warnings.len());
        for warning in &tokenizer_warnings[warning_starts[index]..warning_end] {
            warnings.push(malformed_sgml(warning, warning_path.clone(), input));
        }
    }

    close_remaining(&mut stack, &mut output, &mut warnings);

    NormalizedSgml {
        xml: output,
        warnings,
    }
}

fn push_escaped_text(output: &mut String, text: &str) {
    let mut start = 0;
    for (index, character) in text.char_indices() {
        let replacement = match character {
            '<' => "&lt;",
            '&' => "&amp;",
            _ => continue,
        };
        output.push_str(&text[start..index]);
        output.push_str(replacement);
        start = index + character.len_utf8();
    }
    output.push_str(&text[start..]);
}

fn malformed_sgml(warning: &TokenizeWarning, path: ElementPath, input: &str) -> ParseWarning {
    let offset = match warning {
        TokenizeWarning::UnterminatedTag { span }
        | TokenizeWarning::UnterminatedComment { span }
        | TokenizeWarning::UnterminatedProcessingInstruction { span }
        | TokenizeWarning::EmptyTagName { span }
        | TokenizeWarning::InvalidTagName { span } => span.start,
    };
    ParseWarning::at(
        ParseWarningKind::MalformedSgml {
            message: warning.to_string(),
        },
        path,
        position_at(input, offset),
    )
}

fn current_path(stack: &[Frame]) -> ElementPath {
    ElementPath::from_segments(stack.iter().map(|frame| frame.name.as_str()))
}

fn emit_close(output: &mut String, name: &str) {
    output.push_str("</");
    output.push_str(name);
    output.push('>');
}

fn pop_and_close(stack: &mut Vec<Frame>, output: &mut String) {
    if let Some(frame) = stack.pop() {
        emit_close(output, &frame.name);
    }
}

fn handle_close_tag(
    name: &str,
    stack: &mut Vec<Frame>,
    output: &mut String,
    warnings: &mut Vec<ParseWarning>,
    path: &ElementPath,
    position: crate::diagnostics::SourcePosition,
) {
    let Some(position) = stack.iter().rposition(|frame| frame.name == name) else {
        warnings.push(ParseWarning::at(
            ParseWarningKind::UnmatchedCloseTag {
                name: name.to_owned(),
            },
            path.clone(),
            position,
        ));
        return;
    };

    let closed = stack.split_off(position);
    for frame in closed.iter().rev() {
        emit_close(output, &frame.name);
    }
}

#[allow(clippy::too_many_arguments)]
fn handle_open_tag(
    name: &str,
    raw: &str,
    self_closing: bool,
    tokens: &[Token<'_>],
    index: usize,
    stack: &mut Vec<Frame>,
    output: &mut String,
    warnings: &mut Vec<ParseWarning>,
    path: &ElementPath,
    position: crate::diagnostics::SourcePosition,
) {
    // A leaf cannot contain elements, so a new element always closes it.
    while stack.last().is_some_and(Frame::is_leaf) {
        pop_and_close(stack, output);
    }

    if self_closing {
        output.push_str(raw);
        return;
    }

    let parent = stack.last().map(|frame| frame.name.as_str());
    let kind = schema().kind_in(parent, name);

    match kind {
        Some(NodeKind::Aggregate) => {
            close_to_aggregate_parent(stack, output, name, warnings, path, position);
        }
        Some(NodeKind::Leaf) => {
            close_to_leaf_parent(stack, output, name, warnings, path, position);
        }
        None => {
            // Unknown elements are left where they are. If they contain child
            // elements they are only closed explicitly or at end of input.
            if unknown_is_ambiguous(tokens, index) {
                warnings.push(ParseWarning::at(
                    ParseWarningKind::AmbiguousUnknownElement {
                        name: name.to_owned(),
                    },
                    path.child(name),
                    position,
                ));
            }
        }
    }

    output.push_str(raw);
    let frame_kind = match kind {
        Some(NodeKind::Aggregate) => FrameKind::Aggregate,
        Some(NodeKind::Leaf) => FrameKind::Leaf,
        None if looks_like_leaf(tokens, index) => FrameKind::UnknownLeaf,
        None => FrameKind::UnknownAggregate,
    };
    stack.push(Frame {
        name: name.to_owned(),
        kind: frame_kind,
    });
}

/// Pops frames above the nearest aggregate that can contain `name`.
fn close_to_aggregate_parent(
    stack: &mut Vec<Frame>,
    output: &mut String,
    name: &str,
    warnings: &mut Vec<ParseWarning>,
    path: &ElementPath,
    position: crate::diagnostics::SourcePosition,
) {
    let (close_count, found) = find_aggregate_parent(stack, name, schema());
    if !found && !stack.is_empty() {
        warnings.push(ParseWarning::at(
            ParseWarningKind::MisplacedElement {
                name: name.to_owned(),
            },
            path.child(name),
            position,
        ));
    }
    for _ in 0..close_count {
        pop_and_close(stack, output);
    }
}

/// Pops frames above the nearest aggregate that accepts `leaf`.
fn close_to_leaf_parent(
    stack: &mut Vec<Frame>,
    output: &mut String,
    leaf: &str,
    warnings: &mut Vec<ParseWarning>,
    path: &ElementPath,
    position: crate::diagnostics::SourcePosition,
) {
    let (close_count, found) = find_leaf_parent(stack, leaf, schema());
    if !found && !stack.is_empty() {
        warnings.push(ParseWarning::at(
            ParseWarningKind::MisplacedElement {
                name: leaf.to_owned(),
            },
            path.child(leaf),
            position,
        ));
    }
    for _ in 0..close_count {
        pop_and_close(stack, output);
    }
}

fn find_aggregate_parent(stack: &[Frame], name: &str, schema: &Schema) -> (usize, bool) {
    if stack.is_empty() {
        return (0, true);
    }
    for (offset, frame) in stack.iter().rev().enumerate() {
        match frame.kind {
            FrameKind::Leaf | FrameKind::UnknownLeaf => continue,
            FrameKind::UnknownAggregate => return (offset, true),
            FrameKind::Aggregate => match schema.can_contain(&frame.name, name) {
                Some(true) => return (offset, true),
                Some(false) => continue,
                None => return (offset, true),
            },
        }
    }
    (0, false)
}

fn find_leaf_parent(stack: &[Frame], leaf: &str, schema: &Schema) -> (usize, bool) {
    if stack.is_empty() {
        return (0, true);
    }
    for (offset, frame) in stack.iter().rev().enumerate() {
        match frame.kind {
            FrameKind::Leaf | FrameKind::UnknownLeaf => continue,
            FrameKind::UnknownAggregate => return (offset, true),
            FrameKind::Aggregate => match schema.accepts_leaf(&frame.name, leaf) {
                Some(true) => return (offset, true),
                Some(false) => continue,
                None => return (offset, true),
            },
        }
    }
    (0, false)
}

/// Looks ahead after an unknown opening tag to decide whether it is a leaf.
///
/// An unknown element is treated as an aggregate when the next meaningful
/// token is another opening tag; otherwise it holds text and is a leaf.
fn looks_like_leaf(tokens: &[Token<'_>], index: usize) -> bool {
    for token in &tokens[index + 1..] {
        match token {
            Token::Text { text, .. } => {
                if !text.trim().is_empty() {
                    return true;
                }
            }
            Token::Entity { .. } | Token::Malformed { .. } => return true,
            Token::CloseTag { .. } => return true,
            Token::Comment { .. }
            | Token::ProcessingInstruction { .. }
            | Token::Declaration { .. } => {}
            Token::OpenTag { .. } => return false,
        }
    }
    true
}

fn unknown_is_ambiguous(tokens: &[Token<'_>], index: usize) -> bool {
    let mut text_seen = false;
    for token in &tokens[index + 1..] {
        match token {
            Token::Text { text, .. } => text_seen |= !text.trim().is_empty(),
            Token::OpenTag { .. } if text_seen => return true,
            Token::CloseTag { .. } => return false,
            Token::Entity { .. } | Token::Malformed { .. } => text_seen = true,
            Token::Comment { .. }
            | Token::ProcessingInstruction { .. }
            | Token::Declaration { .. }
            | Token::OpenTag { .. } => {}
        }
    }
    false
}

fn close_remaining(stack: &mut Vec<Frame>, output: &mut String, warnings: &mut Vec<ParseWarning>) {
    while let Some(frame) = stack.pop() {
        let mut path = current_path(stack);
        path.push(&frame.name);
        warnings.push(ParseWarning::new(
            ParseWarningKind::UnclosedElement {
                name: frame.name.clone(),
            },
            path,
        ));
        emit_close(output, &frame.name);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn normalize(input: &str) -> String {
        normalize_sgml_to_xml(input)
    }

    #[test]
    fn simple_leaf_elements() {
        let input = "<CODE>0\n<SEVERITY>INFO\n";
        let result = normalize(input);
        assert!(result.contains("<CODE>0\n</CODE>"), "got: {result}");
        assert!(
            result.contains("<SEVERITY>INFO\n</SEVERITY>"),
            "got: {result}"
        );
    }

    #[test]
    fn preserves_non_ascii_text() {
        // Text runs must pass through as UTF-8, not byte-by-byte as `char`.
        let input = "<NAME>CAFÉ LÉON\n<MEMO>Croissants à la boulangerie\n";
        let result = normalize(input);
        assert!(result.contains("<NAME>CAFÉ LÉON\n</NAME>"), "got: {result}");
        assert!(
            result.contains("<MEMO>Croissants à la boulangerie\n</MEMO>"),
            "got: {result}"
        );
    }

    #[test]
    fn preserves_crlf() {
        let input = "<NAME>CAFÉ\r\n<MEMO>two\r\n";
        let result = normalize(input);
        assert!(result.contains("<NAME>CAFÉ\r\n</NAME>"), "got: {result}");
        assert!(result.contains("<MEMO>two\r\n</MEMO>"), "got: {result}");
    }

    #[test]
    fn escapes_literal_less_than_in_text() {
        let result = normalize("<NAME>A < B and x < 10</NAME>");
        assert!(
            result.contains("<NAME>A &lt; B and x &lt; 10</NAME>"),
            "got: {result}"
        );
    }

    #[test]
    fn escapes_invalid_tag_like_text() {
        let normalized = normalize_sgml_to_xml_with_warnings("<NAME>value<123</NAME>");
        assert!(
            normalized.xml.contains("value&lt;123"),
            "got: {}",
            normalized.xml
        );
        assert!(
            normalized
                .warnings
                .iter()
                .any(|warning| matches!(warning.kind(), ParseWarningKind::MalformedSgml { .. }))
        );
    }

    #[test]
    fn aggregate_with_explicit_close() {
        let input = "<STATUS>\n<CODE>0\n<SEVERITY>INFO\n</STATUS>\n";
        let result = normalize(input);
        assert!(result.contains("<CODE>0\n</CODE>"), "got: {result}");
        assert!(
            result.contains("<SEVERITY>INFO\n</SEVERITY>"),
            "got: {result}"
        );
        assert!(result.contains("</STATUS>"), "got: {result}");
    }

    #[test]
    fn aggregate_with_implicit_close() {
        // DTSERVER is a sibling of STATUS in SONRS, so STATUS should auto-close
        let input = "<SONRS>\n<STATUS>\n<CODE>0\n<SEVERITY>INFO\n<DTSERVER>20240402\n<LANGUAGE>ENG\n</SONRS>\n";
        let result = normalize(input);
        assert!(result.contains("<CODE>0\n</CODE>"), "got: {result}");
        assert!(
            result.contains("<SEVERITY>INFO\n</SEVERITY>"),
            "got: {result}"
        );
        assert!(result.contains("</STATUS>"), "got: {result}");
        assert!(
            result.contains("<DTSERVER>20240402\n</DTSERVER>"),
            "got: {result}"
        );
        assert!(
            result.contains("<LANGUAGE>ENG\n</LANGUAGE>"),
            "got: {result}"
        );
        assert!(result.contains("</SONRS>"), "got: {result}");
    }

    #[test]
    fn already_valid_xml_passes_through() {
        let input = "<STATUS><CODE>0</CODE><SEVERITY>INFO</SEVERITY></STATUS>";
        let result = normalize(input);
        assert!(result.contains("<CODE>0</CODE>"), "got: {result}");
        assert!(
            result.contains("<SEVERITY>INFO</SEVERITY>"),
            "got: {result}"
        );
        assert!(result.contains("</STATUS>"), "got: {result}");
    }

    #[test]
    fn datetime_with_timezone_brackets() {
        let input = "<DTSERVER>20260319000000.000[+0:UTC]\n<LANGUAGE>ENG\n";
        let result = normalize(input);
        assert!(
            result.contains("<DTSERVER>20260319000000.000[+0:UTC]\n</DTSERVER>"),
            "got: {result}"
        );
        assert!(
            result.contains("<LANGUAGE>ENG\n</LANGUAGE>"),
            "got: {result}"
        );
    }

    #[test]
    fn mercury_style_signon() {
        let input = "<OFX>\n<SIGNONMSGSRSV1>\n<SONRS>\n<STATUS>\n<CODE>0\n<SEVERITY>INFO\n<DTSERVER>20260319\n<LANGUAGE>ENG\n<BANKMSGSRSV1>\n</BANKMSGSRSV1>\n</OFX>";
        let result = normalize(input);
        assert!(result.contains("</STATUS>"), "got: {result}");
        assert!(result.contains("</SONRS>"), "got: {result}");
        assert!(result.contains("</SIGNONMSGSRSV1>"), "got: {result}");
        assert!(result.contains("<BANKMSGSRSV1>"), "got: {result}");
    }

    #[test]
    fn acctkey_stays_inside_bank_account() {
        let input = "<BANKACCTFROM>\n<BANKID>001\n<ACCTID>123\n<ACCTTYPE>CHECKING\n<ACCTKEY>KEY\n</BANKACCTFROM>";
        let result = normalize(input);
        assert!(
            result.contains("<ACCTKEY>KEY\n</ACCTKEY></BANKACCTFROM>"),
            "got: {result}"
        );
        assert!(
            !result.contains("</BANKACCTFROM></ACCTKEY>"),
            "got: {result}"
        );
    }

    #[test]
    fn unknown_leaf_is_closed_before_next_element() {
        let input = "<STATUS><CODE>0<FOO>bar<SEVERITY>INFO</STATUS>";
        let result = normalize(input);
        assert!(result.contains("<FOO>bar</FOO>"), "got: {result}");
    }

    #[test]
    fn unknown_aggregate_is_only_closed_explicitly() {
        let input = "<OFX><FOO><BAR>1<BAZ>2</FOO></OFX>";
        let result = normalize(input);
        assert!(
            result.contains("<FOO><BAR>1</BAR><BAZ>2</BAZ></FOO>"),
            "got: {result}"
        );
    }

    #[test]
    fn escapes_raw_ampersands_and_unknown_entities() {
        let input = "<NAME>AT&T &nbsp; &amp;</NAME>";
        let normalized = normalize_sgml_to_xml_with_warnings(input);
        assert!(
            normalized
                .xml
                .contains("<NAME>AT&amp;T &amp;nbsp; &amp;</NAME>")
        );
        assert!(matches!(
            normalized.warnings.as_slice(),
            [first, second]
                if matches!(first.kind(), ParseWarningKind::RawAmpersand)
                    && matches!(second.kind(), ParseWarningKind::UnknownEntity { name } if name == "nbsp")
        ));
    }

    #[test]
    fn warns_about_unmatched_close_tag() {
        let normalized = normalize_sgml_to_xml_with_warnings("</STRAY><A>1");
        assert!(normalized.xml.contains("<A>1</A>"));
        assert!(matches!(
            normalized.warnings.as_slice(),
            [first, second]
                if matches!(first.kind(), ParseWarningKind::UnmatchedCloseTag { name } if name == "STRAY")
                    && matches!(second.kind(), ParseWarningKind::UnclosedElement { name } if name == "A")
        ));
    }

    #[test]
    fn warns_about_unclosed_elements_at_eof() {
        let normalized = normalize_sgml_to_xml_with_warnings("<OFX><A>1");
        assert!(normalized.xml.ends_with("</A></OFX>"));
        assert_eq!(normalized.warnings.len(), 2);
        assert!(matches!(
            &normalized.warnings[0],
            ParseWarning { kind: ParseWarningKind::UnclosedElement { name }, path, .. }
                if name == "A" && path.to_string() == "OFX/A"
        ));
        assert!(matches!(
            &normalized.warnings[1],
            ParseWarning { kind: ParseWarningKind::UnclosedElement { name }, path, .. }
                if name == "OFX" && path.to_string() == "OFX"
        ));
    }
}
