//! A small, forgiving SGML tokenizer.
//!
//! The tokenizer only lexes structure; it does not interpret the OFX schema.
//! It borrows slices from the input and therefore copies no text.

/// Byte range of a token in the source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Span {
    pub start: usize,
    pub end: usize,
}

/// Classification of an entity reference.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum EntityKind {
    /// One of the five predefined XML entities.
    Xml,
    /// A decimal or hexadecimal character reference.
    Numeric,
    /// A named entity that XML does not define without a DTD.
    Unknown,
    /// A bare `&` with no terminating `;`.
    Raw,
}

/// A lexical token of an OFX SGML document.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Token<'a> {
    OpenTag {
        name: &'a str,
        raw: &'a str,
        self_closing: bool,
        span: Span,
    },
    CloseTag {
        name: &'a str,
        span: Span,
    },
    Text {
        text: &'a str,
        span: Span,
    },
    Entity {
        raw: &'a str,
        name: &'a str,
        kind: EntityKind,
        span: Span,
    },
    Comment {
        raw: &'a str,
        span: Span,
    },
    ProcessingInstruction {
        raw: &'a str,
        span: Span,
    },
    Declaration {
        raw: &'a str,
        span: Span,
    },
    /// Input that looks like a tag but cannot be interpreted. It is preserved
    /// verbatim so that no data is lost.
    Malformed {
        raw: &'a str,
        span: Span,
    },
}

/// A non-fatal tokenizer problem.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TokenizeWarning {
    UnterminatedTag { span: Span },
    UnterminatedComment { span: Span },
    UnterminatedProcessingInstruction { span: Span },
    EmptyTagName { span: Span },
    InvalidTagName { span: Span },
}

impl core::fmt::Display for TokenizeWarning {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::UnterminatedTag { span } => {
                write!(f, "unterminated tag at byte {}", span.start)
            }
            Self::UnterminatedComment { span } => {
                write!(f, "unterminated comment at byte {}", span.start)
            }
            Self::UnterminatedProcessingInstruction { span } => {
                write!(
                    f,
                    "unterminated processing instruction at byte {}",
                    span.start
                )
            }
            Self::EmptyTagName { span } => {
                write!(f, "empty tag name at byte {}", span.start)
            }
            Self::InvalidTagName { span } => {
                write!(f, "invalid tag name at byte {}", span.start)
            }
        }
    }
}

/// Streaming SGML tokenizer.
pub(crate) struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
    warnings: Vec<TokenizeWarning>,
}

impl<'a> Tokenizer<'a> {
    pub(crate) fn new(input: &'a str) -> Self {
        Self {
            input,
            pos: 0,
            warnings: Vec::new(),
        }
    }

    /// Returns the warnings collected so far.
    pub(crate) fn warnings(&self) -> &[TokenizeWarning] {
        &self.warnings
    }

    fn span(&self, start: usize, end: usize) -> Span {
        Span { start, end }
    }

    fn emit_malformed(&mut self, start: usize, end: usize, warning: TokenizeWarning) -> Token<'a> {
        self.warnings.push(warning);
        self.pos = end;
        Token::Malformed {
            raw: &self.input[start..end],
            span: self.span(start, end),
        }
    }

    fn next_text(&mut self) -> Option<Token<'a>> {
        let start = self.pos;
        let rest = &self.input[start..];
        let end = rest
            .char_indices()
            .find(|(_, ch)| *ch == '<' || *ch == '&')
            .map_or(self.input.len(), |(index, _)| start + index);

        if end == start {
            return None;
        }

        self.pos = end;
        Some(Token::Text {
            text: &self.input[start..end],
            span: self.span(start, end),
        })
    }

    fn next_literal_less_than(&mut self) -> Token<'a> {
        let start = self.pos;
        self.pos += 1;
        Token::Text {
            text: &self.input[start..self.pos],
            span: self.span(start, self.pos),
        }
    }

    fn next_entity(&mut self) -> Token<'a> {
        let start = self.pos;
        let rest = &self.input[start + 1..];
        let Some(semicolon) = find_entity_end(rest) else {
            self.pos = start + 1;
            return Token::Entity {
                raw: &self.input[start..start + 1],
                name: "",
                kind: EntityKind::Raw,
                span: self.span(start, start + 1),
            };
        };

        let name = &rest[..semicolon];
        let end = start + 1 + semicolon + 1;
        self.pos = end;
        let kind = classify_entity(name);
        Token::Entity {
            raw: &self.input[start..end],
            name,
            kind,
            span: self.span(start, end),
        }
    }
}

impl<'a> Iterator for Tokenizer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.input.len() {
            return None;
        }

        if self.input.as_bytes()[self.pos] == b'&' {
            return Some(self.next_entity());
        }

        if self.input.as_bytes()[self.pos] != b'<' {
            return self.next_text();
        }

        let start = self.pos;

        if !looks_like_markup(&self.input[start..]) {
            return Some(self.next_literal_less_than());
        }

        if self.input[start..].starts_with("<!--") {
            let Some(end_index) = self.input[start + 4..].find("-->") else {
                let end = self.input.len();
                return Some(self.emit_malformed(
                    start,
                    end,
                    TokenizeWarning::UnterminatedComment {
                        span: self.span(start, end),
                    },
                ));
            };
            let end = start + 4 + end_index + 3;
            self.pos = end;
            return Some(Token::Comment {
                raw: &self.input[start..end],
                span: self.span(start, end),
            });
        }

        if self.input[start..].starts_with("<?") {
            let Some(end_index) = self.input[start + 2..].find("?>") else {
                let end = self.input.len();
                return Some(self.emit_malformed(
                    start,
                    end,
                    TokenizeWarning::UnterminatedProcessingInstruction {
                        span: self.span(start, end),
                    },
                ));
            };
            let end = start + 2 + end_index + 2;
            self.pos = end;
            return Some(Token::ProcessingInstruction {
                raw: &self.input[start..end],
                span: self.span(start, end),
            });
        }

        let Some(tag_end) = find_tag_end(&self.input[start + 1..]) else {
            let end = self.input.len();
            return Some(self.emit_malformed(
                start,
                end,
                TokenizeWarning::UnterminatedTag {
                    span: self.span(start, end),
                },
            ));
        };

        let end = start + 1 + tag_end + 1;
        let content = &self.input[start + 1..start + 1 + tag_end];
        self.pos = end;

        if let Some(close) = content.strip_prefix('/') {
            let name = close.trim();
            if name.is_empty() {
                return Some(self.emit_malformed(
                    start,
                    end,
                    TokenizeWarning::EmptyTagName {
                        span: self.span(start, end),
                    },
                ));
            }
            if !is_valid_name(name) {
                return Some(self.emit_malformed(
                    start,
                    end,
                    TokenizeWarning::InvalidTagName {
                        span: self.span(start, end),
                    },
                ));
            }
            return Some(Token::CloseTag {
                name,
                span: self.span(start, end),
            });
        }

        if content.starts_with('!') {
            return Some(Token::Declaration {
                raw: &self.input[start..end],
                span: self.span(start, end),
            });
        }

        let trimmed = content.trim();
        let self_closing = trimmed.ends_with('/');
        let name_part = if self_closing {
            trimmed[..trimmed.len() - 1].trim_end()
        } else {
            trimmed
        };
        let name = name_part.split_whitespace().next().unwrap_or("");

        if name.is_empty() {
            return Some(self.emit_malformed(
                start,
                end,
                TokenizeWarning::EmptyTagName {
                    span: self.span(start, end),
                },
            ));
        }

        if name_part != name || !is_valid_name(name) {
            return Some(self.emit_malformed(
                start,
                end,
                TokenizeWarning::InvalidTagName {
                    span: self.span(start, end),
                },
            ));
        }

        Some(Token::OpenTag {
            name,
            raw: &self.input[start..end],
            self_closing,
            span: self.span(start, end),
        })
    }
}

fn find_entity_end(rest: &str) -> Option<usize> {
    for (index, ch) in rest.char_indices() {
        match ch {
            ';' => return Some(index),
            // Raw text such as `AT&T` must not swallow a later `;` from a
            // different entity, so only name characters are accepted.
            _ if is_entity_name_char(ch) && index < 32 => {}
            _ => return None,
        }
    }
    None
}

fn is_entity_name_char(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || matches!(ch, '#' | '_' | ':' | '-' | '.')
}

fn looks_like_markup(input: &str) -> bool {
    let Some(rest) = input.strip_prefix('<') else {
        return false;
    };
    match rest.chars().next() {
        Some('/') => rest[1..]
            .chars()
            .find(|ch| !ch.is_ascii_whitespace())
            .is_some_and(is_name_start),
        Some('?') | Some('!') => true,
        Some(ch) => is_name_start(ch) || ch.is_ascii_digit() || ch == '=',
        None => false,
    }
}

fn is_name_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || matches!(ch, '_' | ':')
}

fn is_valid_name(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(is_name_start)
        && chars.all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '_' | ':' | '-' | '.'))
}

fn find_tag_end(rest: &str) -> Option<usize> {
    let mut quote = None;
    for (index, ch) in rest.char_indices() {
        match ch {
            '"' | '\'' if quote.is_none() => quote = Some(ch),
            ch if Some(ch) == quote => quote = None,
            '>' if quote.is_none() => return Some(index),
            _ => {}
        }
    }
    None
}

fn classify_entity(name: &str) -> EntityKind {
    match name {
        "amp" | "lt" | "gt" | "quot" | "apos" => EntityKind::Xml,
        _ if name.starts_with('#') => {
            let digits = &name[1..];
            let valid = if let Some(hex) = digits
                .strip_prefix('x')
                .or_else(|| digits.strip_prefix('X'))
            {
                !hex.is_empty() && hex.chars().all(|ch| ch.is_ascii_hexdigit())
            } else {
                !digits.is_empty() && digits.chars().all(|ch| ch.is_ascii_digit())
            };
            if valid {
                EntityKind::Numeric
            } else {
                EntityKind::Unknown
            }
        }
        _ => EntityKind::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(input: &str) -> Vec<Token<'_>> {
        Tokenizer::new(input).collect()
    }

    #[test]
    fn tokenizes_tags_text_and_entities() {
        let tokens = tokens("<NAME>AT&amp;T &#233; &nbsp; </NAME>");
        assert!(matches!(tokens[0], Token::OpenTag { name: "NAME", .. }));
        assert!(matches!(tokens[1], Token::Text { text: "AT", .. }));
        assert!(matches!(
            tokens[2],
            Token::Entity {
                kind: EntityKind::Xml,
                ..
            }
        ));
        assert!(matches!(tokens[3], Token::Text { text: "T ", .. }));
        assert!(matches!(
            tokens[4],
            Token::Entity {
                kind: EntityKind::Numeric,
                ..
            }
        ));
        assert!(matches!(
            tokens[6],
            Token::Entity {
                kind: EntityKind::Unknown,
                ..
            }
        ));
        assert!(matches!(tokens[8], Token::CloseTag { name: "NAME", .. }));
    }

    #[test]
    fn tokenizes_comments_pi_and_declarations() {
        let tokens = tokens("<!-- x --><?xml version=\"1.0\"?><!DOCTYPE OFX><A/>");
        assert!(matches!(tokens[0], Token::Comment { .. }));
        assert!(matches!(tokens[1], Token::ProcessingInstruction { .. }));
        assert!(matches!(tokens[2], Token::Declaration { .. }));
        assert!(matches!(
            tokens[3],
            Token::OpenTag {
                name: "A",
                self_closing: true,
                ..
            }
        ));
    }

    #[test]
    fn preserves_unterminated_input() {
        let mut tokenizer = Tokenizer::new("<NAME>CAFÉ");
        assert!(matches!(tokenizer.next(), Some(Token::OpenTag { .. })));
        assert!(matches!(
            tokenizer.next(),
            Some(Token::Text { text: "CAFÉ", .. })
        ));
        assert_eq!(tokenizer.next(), None);

        let mut tokenizer = Tokenizer::new("<NAME");
        assert!(matches!(
            tokenizer.next(),
            Some(Token::Malformed { raw: "<NAME", .. })
        ));
        assert_eq!(tokenizer.warnings().len(), 1);
    }

    #[test]
    fn tokenizes_crlf_text() {
        let tokens = tokens("<A>one\r\n<B>two\r\n");
        assert!(matches!(
            tokens[1],
            Token::Text {
                text: "one\r\n",
                ..
            }
        ));
        assert!(matches!(
            tokens[3],
            Token::Text {
                text: "two\r\n",
                ..
            }
        ));
    }

    #[test]
    fn treats_literal_less_than_as_text() {
        let tokens = tokens("<NAME>A < B</NAME>");
        assert!(matches!(tokens[2], Token::Text { text: "<", .. }));
        assert!(matches!(tokens[3], Token::Text { text: " B", .. }));
    }

    #[test]
    fn rejects_invalid_tag_names() {
        let mut tokenizer = Tokenizer::new("<123>value");
        assert!(matches!(tokenizer.next(), Some(Token::Malformed { .. })));
        assert!(matches!(
            tokenizer.warnings().first(),
            Some(TokenizeWarning::InvalidTagName { .. })
        ));

        let mut tokenizer = Tokenizer::new("<NAME=value>");
        assert!(matches!(tokenizer.next(), Some(Token::Malformed { .. })));
    }
}
