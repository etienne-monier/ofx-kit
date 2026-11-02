//! Detection and decoding of encodings commonly used by OFX providers.

use core::fmt;

/// The character encoding selected for an OFX source.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DetectedEncoding {
    Utf8,
    Utf16Le,
    Utf16Be,
    Windows1252,
    Big5,
    Gb18030,
    ShiftJis,
    EucKr,
    Other(String),
}

/// A non-fatal condition encountered while decoding an OFX source.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum DecodeWarning {
    FallbackToWindows1252,
    DeclaredEncoding {
        declared: String,
        detected: DetectedEncoding,
    },
    UnsupportedDeclaredEncoding(String),
    CompressedInput,
}

/// The decoded OFX source and the information used to decode it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedSource {
    pub text: String,
    pub encoding: DetectedEncoding,
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
pub fn decode(bytes: &[u8]) -> DecodedSource {
    let mut warnings = Vec::new();
    if is_compressed(bytes) {
        warnings.push(DecodeWarning::CompressedInput);
    }
    let (encoding, skip) = detect(bytes, &mut warnings);
    let text = decode_with(bytes.get(skip..).unwrap_or_default(), &encoding);
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
        return (DetectedEncoding::Utf8, 3);
    }
    if bytes.starts_with(&[0xff, 0xfe]) {
        return (DetectedEncoding::Utf16Le, 2);
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
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

fn declaration(bytes: &[u8]) -> Option<String> {
    let prefix = core::str::from_utf8(bytes.get(..bytes.len().min(1024))?).ok()?;
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
    let prefix = core::str::from_utf8(bytes.get(..bytes.len().min(1024))?).ok()?;
    for line in prefix.lines() {
        if let Some((key, value)) = line.trim().split_once(':')
            && (key.eq_ignore_ascii_case("ENCODING") || key.eq_ignore_ascii_case("CHARSET"))
            && !value.trim().eq_ignore_ascii_case("NONE")
        {
            return Some(value.trim().to_owned());
        }
    }
    None
}

fn encoding_name(name: &str) -> Option<DetectedEncoding> {
    let normalized = name.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    match normalized.as_str() {
        "utf-8" | "utf8" => Some(DetectedEncoding::Utf8),
        "utf-16" | "utf-16le" | "unicode" => Some(DetectedEncoding::Utf16Le),
        "utf-16be" => Some(DetectedEncoding::Utf16Be),
        "windows-1252" | "cp1252" | "1252" | "iso-8859-1" => Some(DetectedEncoding::Windows1252),
        "big5" | "cp950" => Some(DetectedEncoding::Big5),
        "gb18030" | "gbk" | "cp936" => Some(DetectedEncoding::Gb18030),
        "shift-jis" | "sjis" | "cp932" => Some(DetectedEncoding::ShiftJis),
        "euc-kr" | "cp949" => Some(DetectedEncoding::EucKr),
        _ => None,
    }
}

fn decode_with(bytes: &[u8], encoding: &DetectedEncoding) -> String {
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
        codec.decode_without_bom_handling(bytes).0.into_owned()
    }
    #[cfg(not(feature = "encoding"))]
    {
        let _ = encoding;
        String::from_utf8_lossy(bytes).into_owned()
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
        assert_eq!(source.text, "“hello”");
    }
    #[test]
    fn honors_sgml_charset() {
        let source = decode(b"OFXHEADER:100\nCHARSET:1252\n\n<OFX>\x93x\x94");
        assert_eq!(source.encoding, DetectedEncoding::Windows1252);
        assert_eq!(source.text, "OFXHEADER:100\nCHARSET:1252\n\n<OFX>“x”");
    }
}
