#![forbid(unsafe_code)]
#![allow(clippy::result_large_err)]

//! Command-line interface for inspecting, validating, decoding, and converting OFX files.
//!
//! The CLI owns filesystem I/O and presentation; parsing and conversion remain
//! in the `ofx-kit` library.

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use ofx_kit::document::OfxDocument;
use ofx_kit::header::OfxVersion;
use ofx_kit::header::parse_header;
use ofx_kit::xml::{XmlError, XmlErrorKind};
use ofx_kit::{
    ConversionOptions, DecodeWarning, ElementPath, OfxError, convert_bytes, decode,
    parse_with_options,
};
use serde::Serialize;
use time::Date;
use time::format_description::well_known::Rfc3339;

mod args;
mod commands;
mod load;
mod render;
mod report;

use args::*;
use commands::*;
use load::*;
use render::*;
use report::*;

fn main() -> ExitCode {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Inspect(args) => inspect(args, cli.no_color),
        Command::Validate(args) => validate(args, cli.no_color),
        Command::Decode(args) => decode_command(args),
        Command::Convert(args) => convert_command(args),
    };
    ExitCode::from(code)
}
