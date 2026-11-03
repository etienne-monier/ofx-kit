#![forbid(unsafe_code)]
#![deny(clippy::unwrap_used, clippy::expect_used)]
#![warn(missing_docs)]
// Structured parse errors intentionally carry their diagnostic context.
#![allow(clippy::result_large_err)]
#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]

//! `ofx-kit` -- A Rust library for parsing Open Financial Exchange (OFX) documents.
//!
//! This library provides a pure functional parser for OFX 1.x and 2.x documents.
//! It accepts decoded strings or raw bytes and produces typed data structures
//! with no I/O.
//!
//! # Usage
//!
//! ```
//! use ofx_kit::parse;
//!
//! let ofx_content = r#"<?OFX OFXHEADER="200" VERSION="220" SECURITY="NONE" OLDFILEUID="NONE" NEWFILEUID="NONE"?>
//! <OFX>
//! <SIGNONMSGSRSV1>
//! <SONRS>
//! <STATUS><CODE>0</CODE><SEVERITY>INFO</SEVERITY></STATUS>
//! <DTSERVER>20230115120000</DTSERVER>
//! <LANGUAGE>ENG</LANGUAGE>
//! </SONRS>
//! </SIGNONMSGSRSV1>
//! </OFX>"#;
//!
//! let doc = parse(ofx_content).unwrap();
//! assert!(doc.signon().status().is_success());
//! ```

pub mod aggregates;
pub mod conversion;
pub mod diagnostics;
pub mod document;
pub mod encoding;
pub mod error;
pub mod header;
mod parser;
mod schema;
mod sgml;
pub mod types;
pub mod xml;

pub use conversion::{
    ConversionError, ConversionOptions, ConversionWarning, ConvertedSource, convert_bytes,
    convert_str,
};
pub use diagnostics::{ElementPath, ParseWarning, ParseWarningKind, SourcePosition};
pub use document::OfxDocument;
pub use encoding::{DecodeWarning, DecodedSource, DetectedEncoding, decode};
pub use error::OfxError;
pub use parser::{
    ParseOptions, ParsedDocument, UnknownElementPolicy, parse, parse_bytes,
    parse_bytes_with_options, parse_bytes_with_warnings, parse_with_options, parse_with_warnings,
};
