//! CLI args support.
//!
//! This private module contains the args concerns for the OFX command-line tool.

use super::*;

#[derive(Debug, Parser)]
#[command(
    name = "ofx",
    version,
    about = "Inspect and validate Open Financial Exchange files",
    after_help = "Exit codes:\n  0  success\n  1  invalid data or I/O error\n  2  usage error\n\nUse NO_COLOR or --no-color to disable ANSI colors."
)]
pub(crate) struct Cli {
    #[arg(
        long,
        global = true,
        help = "Disable ANSI colors in human-readable output"
    )]
    pub(crate) no_color: bool,
    #[command(subcommand)]
    pub(crate) command: Command,
}

#[derive(Debug, Subcommand)]
pub(crate) enum Command {
    /// Display OFX metadata, accounts, balances, and transactions.
    #[command(visible_alias = "show")]
    Inspect(InspectArgs),
    /// Validate the structure of an OFX file without checking business status codes.
    Validate(ValidateArgs),
    /// Decode an OFX file to UTF-8 and write it to standard output.
    Decode(InputArgs),
    /// Convert an OFX 1.x SGML file to XML.
    Convert(ConvertArgs),
}

#[derive(Debug, Args)]
pub(crate) struct InputArgs {
    /// Path to the OFX file to decode.
    #[arg(value_name = "FILE")]
    pub(crate) file: PathBuf,
}

#[derive(Debug, Args)]
pub(crate) struct ConvertArgs {
    /// Path to the OFX file to convert.
    #[arg(value_name = "FILE")]
    pub(crate) file: PathBuf,
    /// Write the converted XML to this file instead of standard output.
    #[arg(short, long, value_name = "FILE", conflicts_with = "check")]
    pub(crate) output: Option<PathBuf>,
    /// Replace only the VERSION metadata in the generated header.
    #[arg(long, value_name = "VERSION", value_parser = parse_version)]
    pub(crate) to: Option<OfxVersion>,
    /// Indent the generated XML.
    #[arg(long)]
    pub(crate) pretty: bool,
    /// Validate the conversion without writing output.
    #[arg(long)]
    pub(crate) check: bool,
}

#[derive(Debug, Args)]
pub(crate) struct InspectArgs {
    /// Path to the OFX file to inspect.
    #[arg(value_name = "FILE")]
    pub(crate) file: PathBuf,
    #[arg(
        long,
        help = "Emit the stable JSON report instead of human-readable text"
    )]
    pub(crate) json: bool,
    #[arg(
        long = "account",
        value_name = "ACCTID",
        help = "Keep statements whose account ID matches exactly; repeatable"
    )]
    pub(crate) accounts: Vec<String>,
    #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_date, help = "Keep transactions posted on or after this date")]
    pub(crate) from: Option<Date>,
    #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_date, help = "Keep transactions posted on or before this date")]
    pub(crate) to: Option<Date>,
    #[arg(
        long,
        value_name = "N",
        help = "Display at most N transactions per statement"
    )]
    pub(crate) limit: Option<usize>,
}

#[derive(Debug, Args)]
pub(crate) struct ValidateArgs {
    /// Path to the OFX file to validate.
    #[arg(value_name = "FILE")]
    pub(crate) file: PathBuf,
    #[arg(long, help = "Emit a machine-readable validation result")]
    pub(crate) json: bool,
}

pub(crate) fn parse_date(value: &str) -> Result<Date, String> {
    Date::parse(
        value,
        &time::format_description::well_known::Iso8601::DEFAULT,
    )
    .map_err(|_| format!("invalid date '{value}', expected YYYY-MM-DD"))
}

pub(crate) fn parse_version(value: &str) -> Result<OfxVersion, String> {
    value
        .parse()
        .map_err(|error| format!("invalid OFX version '{value}': {error}"))
}
