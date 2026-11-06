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
    let plan = detect(bytes, &mut warnings);
    let (mut text, malformed) = if let Some(split_at) = plan.split_at {
        let header = String::from_utf8_lossy(bytes.get(..split_at).unwrap_or_default());
        let (body, malformed) =
            decode_with(bytes.get(split_at..).unwrap_or_default(), &plan.encoding);
        (format!("{header}{body}"), malformed)
    } else {
        decode_with(bytes.get(plan.skip..).unwrap_or_default(), &plan.encoding)
    };
    if plan.retry_windows1252 {
        let (fallback, fallback_malformed) = decode_with(bytes, &DetectedEncoding::Windows1252);
        text = fallback;
        if fallback_malformed {
            warnings.push(DecodeWarning::MalformedInput {
                encoding: DetectedEncoding::Windows1252,
            });
        }
    }
    if text.starts_with('\u{feff}') {
        text.remove(0);
    }
    if malformed {
        if plan.retry_windows1252 {
            warnings.push(DecodeWarning::DeclaredEncoding {
                declared: plan.declared.unwrap_or_else(|| "UTF-8".to_owned()),
                detected: DetectedEncoding::Windows1252,
            });
        } else {
            warnings.push(DecodeWarning::MalformedInput {
                encoding: plan.encoding.clone(),
            });
        }
    }
    DecodedSource {
        text,
        encoding: if plan.retry_windows1252 {
            DetectedEncoding::Windows1252
        } else {
            plan.encoding
        },
        warnings,
    }
}

pub(crate) fn is_compressed(bytes: &[u8]) -> bool {
    bytes.starts_with(b"PK\x03\x04") || bytes.starts_with(b"\x1f\x8b")
}

struct DecodePlan {
    encoding: DetectedEncoding,
    skip: usize,
    split_at: Option<usize>,
    retry_windows1252: bool,
    declared: Option<String>,
}

#[derive(Default)]
struct Declarations {
    xml_encoding: Option<String>,
    sgml_encoding: Option<String>,
    sgml_charset: Option<String>,
}

fn detect(bytes: &[u8], warnings: &mut Vec<DecodeWarning>) -> DecodePlan {
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        warn_about_bom_conflict(&bytes[3..], &DetectedEncoding::Utf8, warnings);
        return DecodePlan {
            encoding: DetectedEncoding::Utf8,
            skip: 3,
            split_at: None,
            retry_windows1252: false,
            declared: None,
        };
    }
    if bytes.starts_with(&[0xff, 0xfe]) {
        warn_about_bom_conflict(&bytes[2..], &DetectedEncoding::Utf16Le, warnings);
        return DecodePlan {
            encoding: DetectedEncoding::Utf16Le,
            skip: 2,
            split_at: None,
            retry_windows1252: false,
            declared: None,
        };
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        warn_about_bom_conflict(&bytes[2..], &DetectedEncoding::Utf16Be, warnings);
        return DecodePlan {
            encoding: DetectedEncoding::Utf16Be,
            skip: 2,
            split_at: None,
            retry_windows1252: false,
            declared: None,
        };
    }

    let declarations = declarations(bytes);
    if let Some(declared) = declarations.xml_encoding.as_ref() {
        return resolve_declared(bytes, declared, warnings);
    }
    if let Some(declared) = declarations.sgml_encoding.as_ref() {
        if matches!(
            declared.to_ascii_uppercase().as_str(),
            "UNICODE" | "UTF-16" | "UTF-16LE"
        ) {
            return utf16_header_plan(bytes, DetectedEncoding::Utf16Le, Some(declared.clone()));
        }
        if declared.eq_ignore_ascii_case("UTF-16BE") {
            return utf16_header_plan(bytes, DetectedEncoding::Utf16Be, Some(declared.clone()));
        }
        if is_utf8_or_ascii(declared) {
            if declared.eq_ignore_ascii_case("USASCII") || declared.eq_ignore_ascii_case("US-ASCII")
            {
                if let Some(charset) = declarations.sgml_charset.as_ref()
                    && let Some(encoding) = encoding_name(charset)
                {
                    return DecodePlan {
                        encoding,
                        skip: 0,
                        split_at: None,
                        retry_windows1252: false,
                        declared: Some(charset.clone()),
                    };
                }
                if let Some(charset) = declarations.sgml_charset.as_ref() {
                    warnings.push(DecodeWarning::UnsupportedDeclaredEncoding(charset.clone()));
                }
            }
            return utf8_declared_plan(bytes, declared);
        }
        if let Some(encoding) = encoding_name(declared) {
            return match encoding {
                DetectedEncoding::Utf16Le | DetectedEncoding::Utf16Be => {
                    utf16_header_plan(bytes, encoding, Some(declared.clone()))
                }
                encoding => DecodePlan {
                    encoding,
                    skip: 0,
                    split_at: None,
                    retry_windows1252: false,
                    declared: Some(declared.clone()),
                },
            };
        }
        warnings.push(DecodeWarning::UnsupportedDeclaredEncoding(declared.clone()));
    }

    if let Some(charset) = declarations.sgml_charset.as_ref() {
        if let Some(encoding) = encoding_name(charset) {
            return DecodePlan {
                encoding,
                skip: 0,
                split_at: None,
                retry_windows1252: false,
                declared: Some(charset.clone()),
            };
        }
        warnings.push(DecodeWarning::UnsupportedDeclaredEncoding(charset.clone()));
    }

    if core::str::from_utf8(bytes).is_ok() {
        return DecodePlan {
            encoding: DetectedEncoding::Utf8,
            skip: 0,
            split_at: None,
            retry_windows1252: false,
            declared: None,
        };
    }
    warnings.push(DecodeWarning::FallbackToWindows1252);
    DecodePlan {
        encoding: DetectedEncoding::Windows1252,
        skip: 0,
        split_at: None,
        retry_windows1252: false,
        declared: None,
    }
}

fn resolve_declared(bytes: &[u8], declared: &str, warnings: &mut Vec<DecodeWarning>) -> DecodePlan {
    if is_utf8_or_ascii(declared) {
        return utf8_declared_plan(bytes, declared);
    }
    if let Some(encoding) = encoding_name(declared) {
        return match encoding {
            DetectedEncoding::Utf16Le | DetectedEncoding::Utf16Be => {
                utf16_header_plan(bytes, encoding, Some(declared.to_owned()))
            }
            encoding => DecodePlan {
                encoding,
                skip: 0,
                split_at: None,
                retry_windows1252: false,
                declared: Some(declared.to_owned()),
            },
        };
    }
    warnings.push(DecodeWarning::UnsupportedDeclaredEncoding(
        declared.to_owned(),
    ));
    DecodePlan {
        encoding: DetectedEncoding::Utf8,
        skip: 0,
        split_at: None,
        retry_windows1252: core::str::from_utf8(bytes).is_err(),
        declared: Some(declared.to_owned()),
    }
}

fn utf8_declared_plan(bytes: &[u8], declared: &str) -> DecodePlan {
    DecodePlan {
        encoding: DetectedEncoding::Utf8,
        skip: 0,
        split_at: None,
        retry_windows1252: core::str::from_utf8(bytes).is_err(),
        declared: Some(declared.to_owned()),
    }
}

fn utf16_header_plan(
    bytes: &[u8],
    encoding: DetectedEncoding,
    declared: Option<String>,
) -> DecodePlan {
    let less_than = bytes.iter().position(|byte| *byte == b'<');
    let split_at = match encoding {
        DetectedEncoding::Utf16Be => less_than
            .filter(|position| *position > 0 && bytes[*position - 1] == 0)
            .map(|position| position - 1)
            .or(less_than),
        _ => less_than,
    };
    DecodePlan {
        encoding,
        skip: 0,
        split_at,
        retry_windows1252: false,
        declared,
    }
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
    let declarations = declarations(bytes);
    declarations
        .xml_encoding
        .or(declarations.sgml_encoding)
        .or(declarations.sgml_charset)
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

fn declarations(bytes: &[u8]) -> Declarations {
    let prefix = String::from_utf8_lossy(&bytes[..bytes.len().min(1024)]);
    let mut declarations = Declarations {
        xml_encoding: declaration(bytes),
        ..Declarations::default()
    };
    for line in prefix.lines() {
        if let Some((key, value)) = line.trim().split_once(':') {
            let value = value.trim();
            if value.is_empty() || value.eq_ignore_ascii_case("NONE") {
                continue;
            }
            if key.eq_ignore_ascii_case("ENCODING") {
                declarations.sgml_encoding = Some(value.to_owned());
            } else if key.eq_ignore_ascii_case("CHARSET") {
                declarations.sgml_charset = Some(value.to_owned());
            }
        }
    }
    declarations
}

fn is_utf8_or_ascii(name: &str) -> bool {
    matches!(
        name.trim()
            .to_ascii_lowercase()
            .replace(['_', ' '], "-")
            .as_str(),
        "utf-8" | "utf8" | "ascii" | "us-ascii" | "usascii"
    )
}

fn encoding_name(name: &str) -> Option<DetectedEncoding> {
    let normalized = name.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    match normalized.as_str() {
        "utf-8" | "utf8" | "ascii" | "us-ascii" | "usascii" => Some(DetectedEncoding::Utf8),
        "utf-16" | "utf-16le" | "unicode" => Some(DetectedEncoding::Utf16Le),
        "utf-16be" => Some(DetectedEncoding::Utf16Be),
        "windows-1252" | "cp1252" | "1252" | "iso-8859-1" | "iso8859-1" | "iso88591" => {
            Some(DetectedEncoding::Windows1252)
        }
        "windows-1250" | "cp1250" | "1250" => {
            Some(DetectedEncoding::Other("windows-1250".to_owned()))
        }
        "windows-1251" | "cp1251" | "1251" => {
            Some(DetectedEncoding::Other("windows-1251".to_owned()))
        }
        "windows-1253" | "cp1253" | "1253" => {
            Some(DetectedEncoding::Other("windows-1253".to_owned()))
        }
        "windows-1254" | "cp1254" | "1254" => {
            Some(DetectedEncoding::Other("windows-1254".to_owned()))
        }
        "windows-1255" | "cp1255" | "1255" => {
            Some(DetectedEncoding::Other("windows-1255".to_owned()))
        }
        "windows-1256" | "cp1256" | "1256" => {
            Some(DetectedEncoding::Other("windows-1256".to_owned()))
        }
        "windows-1257" | "cp1257" | "1257" => {
            Some(DetectedEncoding::Other("windows-1257".to_owned()))
        }
        "windows-1258" | "cp1258" | "1258" => {
            Some(DetectedEncoding::Other("windows-1258".to_owned()))
        }
        "windows-874" | "cp874" | "874" => Some(DetectedEncoding::Other("windows-874".to_owned())),
        "10000" | "mac" | "macintosh" => Some(DetectedEncoding::Other("macintosh".to_owned())),
        "big5" | "cp950" | "950" => Some(DetectedEncoding::Big5),
        "gb18030" | "gbk" | "cp936" | "936" => Some(DetectedEncoding::Gb18030),
        "shift-jis" | "sjis" | "cp932" | "932" => Some(DetectedEncoding::ShiftJis),
        "euc-kr" | "cp949" | "949" => Some(DetectedEncoding::EucKr),
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
            DetectedEncoding::Other(name) => encoding_rs::Encoding::for_label(name.as_bytes())
                .unwrap_or(encoding_rs::WINDOWS_1252),
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

#[cfg(all(test, feature = "encoding"))]
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
    fn falls_back_to_windows1252_for_misleading_utf8_declaration() {
        let source = decode(b"OFXHEADER:100\nENCODING:UTF-8\nCHARSET:NONE\n\n<OFX>Caf\xe9</OFX>");
        assert_eq!(source.encoding, DetectedEncoding::Windows1252);
        assert_eq!(
            source.text,
            "OFXHEADER:100\nENCODING:UTF-8\nCHARSET:NONE\n\n<OFX>Café</OFX>"
        );
        assert!(source.warnings.contains(&DecodeWarning::DeclaredEncoding {
            declared: "UTF-8".to_owned(),
            detected: DetectedEncoding::Windows1252,
        }));
        assert!(
            !source
                .warnings
                .iter()
                .any(|warning| { matches!(warning, DecodeWarning::MalformedInput { .. }) })
        );
    }

    #[test]
    fn decodes_bomless_unicode_with_ascii_header() {
        let header = b"OFXHEADER:100\nENCODING:UNICODE\nCHARSET:NONE\n\n";
        let body = "<OFX><NAME>Café</NAME></OFX>";
        let mut bytes = header.to_vec();
        for unit in body.encode_utf16() {
            bytes.extend_from_slice(&unit.to_le_bytes());
        }

        let source = decode(&bytes);
        assert_eq!(source.encoding, DetectedEncoding::Utf16Le);
        assert_eq!(
            source.text,
            format!("{}{}", String::from_utf8_lossy(header), body)
        );
        assert!(source.text.starts_with("OFXHEADER:100\nENCODING:UNICODE"));
        assert!(source.text.ends_with(body));
    }

    #[test]
    fn decodes_bomless_utf16be_with_ascii_header() {
        let header = b"OFXHEADER:100\nENCODING:UTF-16BE\nCHARSET:NONE\n\n";
        let body = "<OFX><NAME>Café</NAME></OFX>";
        let mut bytes = header.to_vec();
        for unit in body.encode_utf16() {
            bytes.extend_from_slice(&unit.to_be_bytes());
        }

        let source = decode(&bytes);
        assert_eq!(source.encoding, DetectedEncoding::Utf16Be);
        assert_eq!(
            source.text,
            format!("{}{}", String::from_utf8_lossy(header), body)
        );
    }

    #[test]
    fn utf8_declaration_does_not_use_charset_as_override() {
        let source =
            decode(b"OFXHEADER:100\nENCODING:UTF-8\nCHARSET:1252\n\n<OFX>Caf\xc3\xa9</OFX>");
        assert_eq!(source.encoding, DetectedEncoding::Utf8);
        assert_eq!(
            source.text,
            "OFXHEADER:100\nENCODING:UTF-8\nCHARSET:1252\n\n<OFX>Café</OFX>"
        );
        assert!(source.warnings.is_empty());
    }

    #[test]
    fn unknown_xml_encoding_falls_back_only_when_utf8_is_invalid() {
        let valid = decode(b"<?xml version=\"1.0\" encoding=\"unknown\"?>Caf\xc3\xa9");
        assert_eq!(valid.encoding, DetectedEncoding::Utf8);
        assert_eq!(
            valid.text,
            "<?xml version=\"1.0\" encoding=\"unknown\"?>Café"
        );
        assert!(
            valid
                .warnings
                .contains(&DecodeWarning::UnsupportedDeclaredEncoding(
                    "unknown".to_owned()
                ))
        );

        let invalid = decode(b"<?xml version=\"1.0\" encoding=\"unknown\"?>Caf\xe9");
        assert_eq!(invalid.encoding, DetectedEncoding::Windows1252);
        assert_eq!(
            invalid.text,
            "<?xml version=\"1.0\" encoding=\"unknown\"?>Café"
        );
        assert!(
            invalid
                .warnings
                .contains(&DecodeWarning::UnsupportedDeclaredEncoding(
                    "unknown".to_owned()
                ))
        );
        assert!(invalid.warnings.contains(&DecodeWarning::DeclaredEncoding {
            declared: "unknown".to_owned(),
            detected: DetectedEncoding::Windows1252,
        }));
    }

    #[test]
    fn warns_when_declared_encoding_has_malformed_input() {
        let source = decode(b"<?xml version=\"1.0\" encoding=\"UTF-8\"?>\xff");
        assert_eq!(source.encoding, DetectedEncoding::Windows1252);
        assert!(source.warnings.contains(&DecodeWarning::DeclaredEncoding {
            declared: "UTF-8".to_owned(),
            detected: DetectedEncoding::Windows1252,
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
