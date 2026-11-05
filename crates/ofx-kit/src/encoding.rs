//! Detection and decoding of encodings commonly used by OFX providers.

use core::fmt;

/// The character encoding selected for an OFX source.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DetectedEncoding {
    /// UTF-8 encoding.
    Utf8,
    /// Little-endian UTF-16 encoding.
    Utf16Le,
    /// Big-endian UTF-16 encoding.
    Utf16Be,
    /// Windows-1252 encoding.
    Windows1252,
    /// Big5 encoding.
    Big5,
    /// GB18030 encoding.
    Gb18030,
    /// Shift JIS encoding.
    ShiftJis,
    /// EUC-KR encoding.
    EucKr,
    /// An encoding name not mapped to a dedicated variant.
    Other(String),
}

/// A non-fatal condition encountered while decoding an OFX source.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeWarning {
    /// Decoding fell back to Windows-1252.
    FallbackToWindows1252,
    /// The declared and detected encodings differ.
    DeclaredEncoding {
        /// Encoding declared by the source.
        declared: String,
        /// Encoding selected by detection.
        detected: DetectedEncoding,
    },
    /// The source declared an unsupported encoding name.
    UnsupportedDeclaredEncoding(String),
    /// The input appears to be compressed.
    CompressedInput,
    /// The selected decoder replaced malformed input.
    MalformedInput {
        /// Encoding used while decoding the malformed input.
        encoding: DetectedEncoding,
    },
}

/// The decoded OFX source and the information used to decode it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSource {
    /// Decoded UTF-8 source text.
    pub text: String,
    /// Encoding selected for decoding.
    pub encoding: DetectedEncoding,
    /// Non-fatal decoding warnings.
    pub warnings: Vec<DecodeWarning>,
}

impl fmt::Display for DetectedEncoding {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Utf8 => f.write_str("UTF-8"),
            Self::Utf16Le => f.write_str("UTF-16LE"),
            Self::Utf16Be => f.write_str("UTF-16BE"),
            Self::Windows1252 => f.write_str("windows-1252"),
            Self::Big5 => f.write_str("Big5"),
            Self::Gb18030 => f.write_str("GB18030"),
            Self::ShiftJis => f.write_str("Shift_JIS"),
            Self::EucKr => f.write_str("EUC-KR"),
            Self::Other(name) => f.write_str(name),
        }
    }
}

/// Decodes raw OFX bytes according to the roadmap detection order.
#[must_use]
///
/// # Example
///
/// ```
/// let decoded = ofx_kit::decode(b"OFXHEADER:100\nVERSION:102\n");
/// assert_eq!(decoded.encoding, ofx_kit::DetectedEncoding::Utf8);
/// assert!(decoded.text.starts_with("OFXHEADER"));
/// ```
pub fn decode(bytes: &[u8]) -> DecodedSource {
    let mut warnings = Vec::new();
    if is_compressed(bytes) {
        warnings.push(DecodeWarning::CompressedInput);
    }
    let (encoding, skip) = detect(bytes, &mut warnings);
    let (text, malformed) = decode_with(bytes.get(skip..).unwrap_or_default(), &encoding);
    if malformed {
        warnings.push(DecodeWarning::MalformedInput {
            encoding: encoding.clone(),
        });
    }
    DecodedSource {
        text,
        encoding,
        warnings,
    }
}

pub(crate) fn is_compressed(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"\x1f\x8b")
}

fn detect(bytes: &[u8], warnings: &mut Vec<DecodeWarning>) -> (DetectedEncoding, usize) {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        warn_about_bom_conflict(&bytes[3..], &DetectedEncoding::Utf8, warnings);
        return (DetectedEncoding::Utf8, 3);
    }
    if bytes.starts_with(&[0xff, 0xfe]) {
        warn_about_bom_conflict(&bytes[2..], &DetectedEncoding::Utf16Le, warnings);
        return (DetectedEncoding::Utf16Le, 2);
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        warn_about_bom_conflict(&bytes[2..], &DetectedEncoding::Utf16Be, warnings);
        return (DetectedEncoding::Utf16Be, 2);
    }
    if let Some(declared) = declaration(bytes) {
        if let Some(encoding) = encoding_name(&declared) {
            return (encoding, 0);
        }
        warnings.push(DecodeWarning::UnsupportedDeclaredEncoding(declared));
    }
    if let Some(declared) = sgml_declaration(bytes) {
        if declared.eq_ignore_ascii_case("UNICODE") {
            return (DetectedEncoding::Utf16Le, 0);
        }
        if let Some(encoding) = encoding_name(&declared) {
            return (encoding, 0);
        }
        warnings.push(DecodeWarning::UnsupportedDeclaredEncoding(declared));
    }
    if core::str::from_utf8(bytes).is_ok() {
        return (DetectedEncoding::Utf8, 0);
    }
    warnings.push(DecodeWarning::FallbackToWindows1252);
    (DetectedEncoding::Windows1252, 0)
}

fn warn_about_bom_conflict(
    bytes: &[u8],
    detected: &DetectedEncoding,
    warnings: &mut Vec<DecodeWarning>,
) {
    if let Some(declared) = declared_encoding(bytes)
        && let Some(declared_encoding) = encoding_name(&declared)
        && declared_encoding != detected.clone()
    {
        warnings.push(DecodeWarning::DeclaredEncoding {
            declared,
            detected: detected.clone(),
        });
    }
}

fn declared_encoding(bytes: &[u8]) -> Option<String> {
    declaration(bytes).or_else(|| sgml_declaration(bytes))
}

fn declaration(bytes: &[u8]) -> Option<String> {
    let prefix = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
    let lower = prefix.to_ascii_lowercase();
    let start = lower.find("encoding")?;
    let rest = &prefix[start + 8..];
    let value = rest.trim_start().strip_prefix('=')?.trim_start();
    let quote = value.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }
    let end = value[1..].find(quote)? + 1;
    Some(value[1..end].to_owned())
}

fn sgml_declaration(bytes: &[u8]) -> Option<String> {
    let prefix = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
    let mut encoding = None;
    let mut charset = None;
    for line in prefix.lines() {
        if let Some((key, value)) = line.trim().split_once(':')
            && !value.trim().eq_ignore_ascii_case("NONE")
        {
            if key.eq_ignore_ascii_case("ENCODING") {
                encoding = Some(value.trim().to_owned());
            } else if key.eq_ignore_ascii_case("CHARSET") {
                charset = Some(value.trim().to_owned());
            }
        }
    }
    charset.or(encoding)
}

fn encoding_name(name: &str) -> Option<DetectedEncoding> {
    let normalized = name.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    match normalized.as_str() {
        "utf-8" | "utf8" | "ascii" | "us-ascii" | "usascii" => Some(DetectedEncoding::Utf8),
        "utf-16" | "utf-16le" | "unicode" => Some(DetectedEncoding::Utf16Le),
        "utf-16be" => Some(DetectedEncoding::Utf16Be),
        "windows-1252" | "cp1252" | "1252" | "iso-8859-1" => Some(DetectedEncoding::Windows1252),
        "big5" | "cp950" => Some(DetectedEncoding::Big5),
        "gb18030" | "gbk" | "cp936" => Some(DetectedEncoding::Gb18030),
        "shift-jis" | "sjis" | "cp932" | "932" => Some(DetectedEncoding::ShiftJis),
        "euc-kr" | "cp949" => Some(DetectedEncoding::EucKr),
        _ => None,
    }
}

fn decode_with(bytes: &[u8], encoding: &DetectedEncoding) -> (String, bool) {
    #[cfg(feature = "encoding")]
    {
        let codec = match encoding {
            DetectedEncoding::Utf8 => encoding_rs::UTF_8,
            DetectedEncoding::Utf16Le => encoding_rs::UTF_16LE,
            DetectedEncoding::Utf16Be => encoding_rs::UTF_16BE,
            DetectedEncoding::Windows1252 => encoding_rs::WINDOWS_1252,
            DetectedEncoding::Big5 => encoding_rs::BIG5,
            DetectedEncoding::Gb18030 => encoding_rs::GB18030,
            DetectedEncoding::ShiftJis => encoding_rs::SHIFT_JIS,
            DetectedEncoding::EucKr => encoding_rs::EUC_KR,
            DetectedEncoding::Other(_) => encoding_rs::UTF_8,
        };
        let (text, had_errors) = codec.decode_without_bom_handling(bytes);
        (text.into_owned(), had_errors)
    }
    #[cfg(not(feature = "encoding"))]
    {
        let _ = encoding;
        let valid = core::str::from_utf8(bytes).is_ok();
        (String::from_utf8_lossy(bytes).into_owned(), !valid)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detects_boms() {
        assert_eq!(
            decode(b"\xef\xbb\xbfhello").encoding,
            DetectedEncoding::Utf8
        );
        assert_eq!(decode(b"\xff\xfeh\0").encoding, DetectedEncoding::Utf16Le);
        assert_eq!(decode(b"\xfe\xff\0h").encoding, DetectedEncoding::Utf16Be);
        assert_eq!(decode(b"\xff\xfeh\0i\0").text, "hi");
        assert_eq!(decode(b"\xfe\xff\0h\0i").text, "hi");
    }
    #[test]
    fn falls_back_with_warning() {
        let source = decode(b"\x93hello\x94");
        assert_eq!(source.encoding, DetectedEncoding::Windows1252);
        assert!(
            source
                .warnings
                .contains(&DecodeWarning::FallbackToWindows1252)
        );
        assert!(
            !source
                .warnings
                .iter()
                .any(|warning| matches!(warning, DecodeWarning::MalformedInput { .. }))
        );
        assert_eq!(source.text, "“hello”");
    }
    #[test]
    fn honors_sgml_charset() {
        let source = decode(b"OFXHEADER:100\nCHARSET:1252\n\n<OFX>\x93x\x94");
        assert_eq!(source.encoding, DetectedEncoding::Windows1252);
        assert!(
            !source
                .warnings
                .iter()
                .any(|warning| matches!(warning, DecodeWarning::UnsupportedDeclaredEncoding(_)))
        );
        assert_eq!(source.text, "OFXHEADER:100\nCHARSET:1252\n\n<OFX>“x”");
    }

    #[test]
    fn reads_sgml_charset_when_encoding_is_us_ascii() {
        let source = decode(b"OFXHEADER:100\nENCODING:USASCII\nCHARSET:1252\n\n<OFX>Caf\xe9</OFX>");
        assert_eq!(source.encoding, DetectedEncoding::Windows1252);
        assert!(source.warnings.is_empty());
        assert_eq!(
            source.text,
            "OFXHEADER:100\nENCODING:USASCII\nCHARSET:1252\n\n<OFX>Café</OFX>"
        );
    }

    #[test]
    fn warns_when_declared_encoding_has_malformed_input() {
        let source = decode(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\xff");
        assert_eq!(source.encoding, DetectedEncoding::Utf8);
        assert!(source.warnings.contains(&DecodeWarning::MalformedInput {
            encoding: DetectedEncoding::Utf8,
        }));
    }

    #[test]
    fn warns_when_bom_and_declaration_conflict() {
        let source = decode(b"\xef\xbb\xbf<?xml encoding=\"windows-1252\"?>");
        assert_eq!(source.encoding, DetectedEncoding::Utf8);
        assert!(source.warnings.contains(&DecodeWarning::DeclaredEncoding {
            declared: "windows-1252".to_owned(),
            detected: DetectedEncoding::Utf8,
        }));
    }
}
