#![forbid(unsafe_code)]

use std::io::{self, IsTerminal, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use ofx_kit::document::OfxDocument;
use ofx_kit::header::parse_header;
use ofx_kit::xml::{XmlError, XmlErrorKind};
use ofx_kit::{DecodeWarning, ElementPath, OfxError, decode, parse_bytes};
use serde::Serialize;
use time::Date;
use time::format_description::well_known::Rfc3339;

#[derive(Debug, Parser)]
#[command(
    name = "ofx",
    version,
    about = "Inspect and validate Open Financial Exchange files",
    after_help = "Exit codes:\n  0  success\n  1  invalid data or I/O error\n  2  usage error\n\nUse NO_COLOR or --no-color to disable ANSI colors."
)]
struct Cli {
    #[arg(
        long,
        global = true,
        help = "Disable ANSI colors in human-readable output"
    )]
    no_color: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Display OFX metadata, accounts, balances, and transactions.
    #[command(visible_alias = "show")]
    Inspect(InspectArgs),
    /// Validate the structure of an OFX file without checking business status codes.
    Validate(ValidateArgs),
    /// Decode an OFX file to UTF-8 and write it to standard output.
    Decode(InputArgs),
}

#[derive(Debug, Args)]
struct InputArgs {
    /// Path to the OFX file to decode.
    #[arg(value_name = "FILE")]
    file: PathBuf,
}

#[derive(Debug, Args)]
struct InspectArgs {
    /// Path to the OFX file to inspect.
    #[arg(value_name = "FILE")]
    file: PathBuf,
    #[arg(
        long,
        help = "Emit the stable JSON report instead of human-readable text"
    )]
    json: bool,
    #[arg(
        long = "account",
        value_name = "ACCTID",
        help = "Keep statements whose account ID matches exactly; repeatable"
    )]
    accounts: Vec<String>,
    #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_date, help = "Keep transactions posted on or after this date")]
    from: Option<Date>,
    #[arg(long, value_name = "YYYY-MM-DD", value_parser = parse_date, help = "Keep transactions posted on or before this date")]
    to: Option<Date>,
    #[arg(
        long,
        value_name = "N",
        help = "Display at most N transactions per statement"
    )]
    limit: Option<usize>,
}

#[derive(Debug, Args)]
struct ValidateArgs {
    /// Path to the OFX file to validate.
    #[arg(value_name = "FILE")]
    file: PathBuf,
    #[arg(long, help = "Emit a machine-readable validation result")]
    json: bool,
}

fn parse_date(value: &str) -> Result<Date, String> {
    Date::parse(
        value,
        &time::format_description::well_known::Iso8601::DEFAULT,
    )
    .map_err(|_| format!("invalid date '{value}', expected YYYY-MM-DD"))
}

struct Loaded {
    decoded: ofx_kit::DecodedSource,
    document: Result<OfxDocument, OfxError>,
}

fn load(path: &PathBuf) -> Result<Loaded, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let decoded = decode(&bytes);
    let document = parse_bytes(&bytes);
    Ok(Loaded { decoded, document })
}

#[derive(Debug, Serialize)]
struct WarningJson {
    code: String,
    message: String,
}

fn warning_json(warning: &DecodeWarning) -> WarningJson {
    let (code, message) = match warning {
        DecodeWarning::FallbackToWindows1252 => (
            "fallback_to_windows_1252",
            "fell back to Windows-1252".to_owned(),
        ),
        DecodeWarning::DeclaredEncoding { declared, detected } => (
            "declared_encoding_mismatch",
            format!("declared {declared}, detected {detected}"),
        ),
        DecodeWarning::UnsupportedDeclaredEncoding(value) => (
            "unsupported_declared_encoding",
            format!("unsupported declared encoding: {value}"),
        ),
        DecodeWarning::CompressedInput => (
            "compressed_input",
            "compressed input is not supported".to_owned(),
        ),
        DecodeWarning::MalformedInput { encoding } => {
            ("malformed_input", format!("malformed input for {encoding}"))
        }
        _ => ("unknown", "unknown decoding warning".to_owned()),
    };
    WarningJson {
        code: code.to_owned(),
        message,
    }
}

#[derive(Debug, Serialize)]
struct SourceJson {
    encoding: String,
    declared_encoding: Option<String>,
    declared_charset: Option<String>,
    warnings: Vec<WarningJson>,
}

fn source_json(loaded: &Loaded) -> SourceJson {
    let (declared_encoding, declared_charset) = parse_header(&loaded.decoded.text)
        .ok()
        .map(|(h, _)| {
            (
                h.encoding().map(str::to_owned),
                h.charset().map(str::to_owned),
            )
        })
        .unwrap_or((None, None));
    SourceJson {
        encoding: loaded.decoded.encoding.to_string(),
        declared_encoding,
        declared_charset,
        warnings: loaded.decoded.warnings.iter().map(warning_json).collect(),
    }
}

#[derive(Debug, Serialize)]
struct ErrorJson {
    kind: String,
    message: String,
    causes: Vec<String>,
}

fn error_json(error: &OfxError) -> ErrorJson {
    let kind = match error {
        OfxError::UnsupportedCompression => "compression",
        OfxError::Header(_) => "header",
        OfxError::Xml(_) => "xml",
        OfxError::Aggregate(_) => "aggregate",
        _ => "unknown",
    };
    let mut causes = Vec::new();
    let mut current = std::error::Error::source(error);
    while let Some(cause) = current {
        causes.push(cause.to_string());
        current = cause.source();
    }
    ErrorJson {
        kind: kind.to_owned(),
        message: error.to_string(),
        causes,
    }
}

#[derive(Debug, Serialize)]
struct ErrorEnvelope {
    schema_version: u8,
    error: ErrorJson,
}

fn date_string(date: &ofx_kit::types::OfxDateTime) -> String {
    date.as_offset_date_time()
        .format(&Rfc3339)
        .unwrap_or_else(|_| date.to_string())
}

#[derive(Debug, Serialize)]
struct TransactionJson {
    date_posted: String,
    r#type: String,
    amount: String,
    fit_id: String,
    name: Option<String>,
    memo: Option<String>,
    check_number: Option<String>,
    reference_number: Option<String>,
    correction_id: Option<String>,
    correction_action: Option<String>,
}

#[derive(Debug, Serialize)]
struct BalanceJson {
    amount: String,
    as_of: String,
}

#[derive(Debug, Serialize)]
struct StatementJson {
    kind: String,
    transaction_uid: String,
    status: StatusJson,
    account: AccountJson,
    currency: String,
    period: Option<PeriodJson>,
    ledger_balance: Option<BalanceJson>,
    available_balance: Option<BalanceJson>,
    transaction_count: usize,
    transactions_truncated: bool,
    transactions: Vec<TransactionJson>,
}

#[derive(Debug, Serialize)]
struct AccountJson {
    kind: String,
    account_id: String,
    bank_id: Option<String>,
    branch_id: Option<String>,
    account_type: Option<String>,
    broker_id: Option<String>,
}

#[derive(Debug, Serialize)]
struct PeriodJson {
    start: String,
    end: String,
}

#[derive(Debug, Serialize)]
struct StatusJson {
    code: u32,
    severity: String,
    message: Option<String>,
}

#[derive(Debug, Serialize)]
struct ReportJson {
    schema_version: u8,
    source: SourceJson,
    header: HeaderJson,
    signon: SignonJson,
    statements: Vec<StatementJson>,
}

#[derive(Debug, Serialize)]
struct HeaderJson {
    version: String,
    format: String,
    security: String,
    old_file_uid: String,
    new_file_uid: String,
    declared_compression: Option<String>,
}

#[derive(Debug, Serialize)]
struct SignonJson {
    status: StatusJson,
    server_time: String,
    language: String,
    institution: InstitutionJson,
}

#[derive(Debug, Serialize)]
struct InstitutionJson {
    org: Option<String>,
    fid: Option<String>,
}

fn status_json(status: &ofx_kit::aggregates::Status) -> StatusJson {
    StatusJson {
        code: status.code(),
        severity: status.severity().to_string(),
        message: status.message().map(str::to_owned),
    }
}

fn transactions_json(
    list: Option<&ofx_kit::aggregates::TransactionList>,
    from: Option<Date>,
    to: Option<Date>,
    limit: Option<usize>,
) -> (Option<PeriodJson>, usize, bool, Vec<TransactionJson>) {
    let Some(list) = list else {
        return (None, 0, false, Vec::new());
    };
    let period = Some(PeriodJson {
        start: date_string(list.start()),
        end: date_string(list.end()),
    });
    let filtered: Vec<_> = list
        .transactions()
        .iter()
        .filter(|tx| {
            let date = tx.date_posted().as_offset_date_time().date();
            from.is_none_or(|value| date >= value) && to.is_none_or(|value| date <= value)
        })
        .collect();
    let count = filtered.len();
    let shown = limit.map_or(filtered.as_slice(), |n| &filtered[..n.min(filtered.len())]);
    let truncated = shown.len() < count;
    let transactions = shown
        .iter()
        .map(|tx| TransactionJson {
            date_posted: date_string(tx.date_posted()),
            r#type: tx.transaction_type().to_string(),
            amount: tx.amount().to_string(),
            fit_id: tx.fit_id().as_str().to_owned(),
            name: tx.name().map(str::to_owned),
            memo: tx.memo().map(str::to_owned),
            check_number: tx.check_number().map(|v| v.as_str().to_owned()),
            reference_number: tx.reference_number().map(str::to_owned),
            correction_id: tx.correction_id().map(|v| v.as_str().to_owned()),
            correction_action: tx.correction_action().map(|v| v.to_string()),
        })
        .collect();
    (period, count, truncated, transactions)
}

fn report(loaded: &Loaded, args: &InspectArgs) -> Result<ReportJson, OfxError> {
    let document = loaded.document.as_ref().map_err(Clone::clone)?;
    if let (Some(from), Some(to)) = (args.from, args.to)
        && from > to
    {
        return Err(OfxError::Xml(XmlError::new(
            XmlErrorKind::MalformedXml {
                message: "--from must not be later than --to".to_owned(),
            },
            ElementPath::new(),
        )));
    }
    let header = document.header();
    let mut statements = document
        .banking()
        .into_iter()
        .flat_map(|set| set.statement_responses())
        .filter_map(|wrapper| {
            let response = wrapper.response()?;
            let id = response.bank_account().account_id().as_str();
            if !args.accounts.is_empty() && !args.accounts.iter().any(|account| account == id) {
                return None;
            }
            let (period, count, truncated, transactions) =
                transactions_json(response.transaction_list(), args.from, args.to, args.limit);
            Some(StatementJson {
                kind: "bank".to_owned(),
                transaction_uid: wrapper.transaction_uid().to_owned(),
                status: status_json(wrapper.status()),
                account: AccountJson {
                    kind: "bank".to_owned(),
                    account_id: id.to_owned(),
                    bank_id: Some(response.bank_account().bank_id().as_str().to_owned()),
                    branch_id: response
                        .bank_account()
                        .branch_id()
                        .map(|v| v.as_str().to_owned()),
                    account_type: Some(response.bank_account().account_type().to_string()),
                    broker_id: None,
                },
                currency: response.currency_default().to_string(),
                period,
                ledger_balance: response.ledger_balance().map(|v| BalanceJson {
                    amount: v.amount().to_string(),
                    as_of: date_string(v.as_of()),
                }),
                available_balance: response.available_balance().map(|v| BalanceJson {
                    amount: v.amount().to_string(),
                    as_of: date_string(v.as_of()),
                }),
                transaction_count: count,
                transactions_truncated: truncated,
                transactions,
            })
        })
        .collect::<Vec<_>>();
    statements.extend(
        document
            .credit_card()
            .into_iter()
            .flat_map(|set| set.statement_responses())
            .filter_map(|wrapper| {
                let response = wrapper.response()?;
                let id = response.credit_card_account().account_id().as_str();
                if !args.accounts.is_empty() && !args.accounts.iter().any(|account| account == id) {
                    return None;
                }
                let (period, count, truncated, transactions) =
                    transactions_json(response.transaction_list(), args.from, args.to, args.limit);
                Some(StatementJson {
                    kind: "credit_card".to_owned(),
                    transaction_uid: wrapper.transaction_uid().to_owned(),
                    status: status_json(wrapper.status()),
                    account: AccountJson {
                        kind: "credit_card".to_owned(),
                        account_id: id.to_owned(),
                        bank_id: None,
                        branch_id: None,
                        account_type: None,
                        broker_id: None,
                    },
                    currency: response.currency_default().to_string(),
                    period,
                    ledger_balance: response.ledger_balance().map(|v| BalanceJson {
                        amount: v.amount().to_string(),
                        as_of: date_string(v.as_of()),
                    }),
                    available_balance: response.available_balance().map(|v| BalanceJson {
                        amount: v.amount().to_string(),
                        as_of: date_string(v.as_of()),
                    }),
                    transaction_count: count,
                    transactions_truncated: truncated,
                    transactions,
                })
            }),
    );
    statements.extend(
        document
            .investment()
            .into_iter()
            .flat_map(|set| set.statement_responses())
            .filter_map(|wrapper| {
                let response = wrapper.response()?;
                let id = response.investment_account().account_id().as_str();
                if !args.accounts.is_empty() && !args.accounts.iter().any(|account| account == id) {
                    return None;
                }
                let (period, count, truncated, transactions) =
                    transactions_json(response.transaction_list(), args.from, args.to, args.limit);
                Some(StatementJson {
                    kind: "investment".to_owned(),
                    transaction_uid: wrapper.transaction_uid().to_owned(),
                    status: status_json(wrapper.status()),
                    account: AccountJson {
                        kind: "investment".to_owned(),
                        account_id: id.to_owned(),
                        bank_id: None,
                        branch_id: None,
                        account_type: None,
                        broker_id: Some(response.investment_account().broker_id().to_owned()),
                    },
                    currency: response.currency_default().to_string(),
                    period,
                    ledger_balance: None,
                    available_balance: None,
                    transaction_count: count,
                    transactions_truncated: truncated,
                    transactions,
                })
            }),
    );
    Ok(ReportJson {
        schema_version: 1,
        source: source_json(loaded),
        header: HeaderJson {
            version: header.version().to_string(),
            format: if header.version().major() < 2 {
                "sgml"
            } else {
                "xml"
            }
            .to_owned(),
            security: header.security().to_string(),
            old_file_uid: header.old_file_uid().to_owned(),
            new_file_uid: header.new_file_uid().to_owned(),
            declared_compression: header.compression().map(str::to_owned),
        },
        signon: SignonJson {
            status: status_json(document.signon().status()),
            server_time: date_string(document.signon().date_time_server()),
            language: document.signon().language().to_owned(),
            institution: InstitutionJson {
                org: document.signon().fi_org().map(str::to_owned),
                fid: document.signon().fi_id().map(str::to_owned),
            },
        },
        statements,
    })
}

fn print_error_json(error: &OfxError) {
    println!(
        "{}",
        serde_json::to_string_pretty(&ErrorEnvelope {
            schema_version: 1,
            error: error_json(error)
        })
        .unwrap_or_else(|_| "{\"error\":{}}".to_owned())
    );
}

fn inspect(args: InspectArgs, no_color: bool) -> u8 {
    if let (Some(from), Some(to)) = (args.from, args.to)
        && from > to
    {
        eprintln!("--from must not be later than --to");
        return 2;
    }
    let loaded = match load(&args.file) {
        Ok(value) => value,
        Err(error) => {
            if args.json {
                println!(
                    "{{\"schema_version\":1,\"error\":{{\"kind\":\"io\",\"message\":{}}}}}",
                    serde_json::to_string(&error).unwrap_or_default()
                );
            } else {
                eprintln!("{error}");
            }
            return 1;
        }
    };
    let report = match report(&loaded, &args) {
        Ok(value) => value,
        Err(error) => {
            if args.json {
                print_error_json(&error);
            } else {
                eprintln!("{error}");
            }
            return if error.to_string().contains("--from") {
                2
            } else {
                1
            };
        }
    };
    if args.json {
        println!(
            "{}",
            serde_json::to_string_pretty(&report).unwrap_or_default()
        );
    } else {
        render_text(&report, no_color);
    }
    0
}

struct Style {
    enabled: bool,
}

impl Style {
    fn new(no_color: bool) -> Self {
        let no_color_env = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        Self {
            enabled: !no_color && !no_color_env && io::stdout().is_terminal(),
        }
    }

    fn paint(&self, code: &str, text: impl std::fmt::Display) -> String {
        if self.enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

fn render_text(report: &ReportJson, no_color: bool) {
    let style = Style::new(no_color);
    println!("{}", style.paint("1;36", "OFX inspection"));
    println!(
        "Format: {} {} | Encoding: {}",
        report.header.format, report.header.version, report.source.encoding
    );
    println!(
        "Signon: {} ({})",
        if report.signon.status.code == 0 {
            style.paint("32", report.signon.status.code)
        } else {
            style.paint("31", report.signon.status.code)
        },
        if report.signon.status.code == 0 {
            style.paint("32", &report.signon.status.severity)
        } else {
            style.paint("31", &report.signon.status.severity)
        }
    );
    if let Some(org) = &report.signon.institution.org {
        println!("Institution: {org}");
    }
    for statement in &report.statements {
        println!(
            "\n{} account {} ({})",
            style.paint("1;34", &statement.kind),
            statement.account.account_id,
            statement.currency
        );
        if let Some(period) = &statement.period {
            println!("Period: {} to {}", period.start, period.end);
        }
        if let Some(balance) = &statement.ledger_balance {
            println!("Ledger balance: {}", balance.amount);
        }
        println!(
            "Transactions: {}{}",
            statement.transaction_count,
            if statement.transactions_truncated {
                " (limited)"
            } else {
                ""
            }
        );
        for transaction in &statement.transactions {
            println!(
                "  {} {:>12} {}",
                transaction.date_posted,
                if transaction.amount.starts_with('-') {
                    style.paint("31", &transaction.amount)
                } else {
                    style.paint("32", &transaction.amount)
                },
                transaction
                    .name
                    .as_deref()
                    .or(transaction.memo.as_deref())
                    .unwrap_or("(unnamed)")
            );
        }
    }
    for warning in &report.source.warnings {
        eprintln!("{} {}", style.paint("33", "warning:"), warning.message);
    }
}

fn validate(args: ValidateArgs, no_color: bool) -> u8 {
    let style = Style::new(no_color);
    let loaded = match load(&args.file) {
        Ok(value) => value,
        Err(error) => {
            if args.json {
                println!(
                    "{{\"schema_version\":1,\"valid\":false,\"error\":{{\"kind\":\"io\",\"message\":{}}}}}",
                    serde_json::to_string(&error).unwrap_or_default()
                );
            } else {
                eprintln!("{error}");
            }
            return 1;
        }
    };
    match &loaded.document {
        Ok(document) => {
            let statements = document
                .banking()
                .map_or(0, |v| v.statement_responses().len())
                + document
                    .credit_card()
                    .map_or(0, |v| v.statement_responses().len())
                + document
                    .investment()
                    .map_or(0, |v| v.statement_responses().len());
            let transactions = document
                .banking()
                .into_iter()
                .flat_map(|v| v.statement_responses())
                .filter_map(|v| v.response()?.transaction_list())
                .map(|v| v.len())
                .sum::<usize>()
                + document
                    .credit_card()
                    .into_iter()
                    .flat_map(|v| v.statement_responses())
                    .filter_map(|v| v.response()?.transaction_list())
                    .map(|v| v.len())
                    .sum::<usize>()
                + document
                    .investment()
                    .into_iter()
                    .flat_map(|v| v.statement_responses())
                    .filter_map(|v| v.response()?.transaction_list())
                    .map(|v| v.len())
                    .sum::<usize>();
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({"schema_version":1,"valid":true,"source":source_json(&loaded),"header":{"version":document.header().version().to_string(),"format":if document.header().version().major()<2 {"sgml"} else {"xml"},"security":document.header().security().to_string()},"summary":{"statements":statements,"transactions":transactions},"error":null})
                );
            } else {
                println!(
                    "{}: {} ({} statements, {} transactions)",
                    args.file.display(),
                    style.paint("32", "valid"),
                    statements,
                    transactions
                );
            }
            0
        }
        Err(error) => {
            if args.json {
                println!(
                    "{}",
                    serde_json::json!({"schema_version":1,"valid":false,"source":source_json(&loaded),"header":null,"summary":null,"error":error_json(error)})
                );
            } else {
                eprintln!(
                    "{}: {}: {error}",
                    args.file.display(),
                    style.paint("31", "invalid")
                );
            }
            1
        }
    }
}

fn decode_command(args: InputArgs) -> u8 {
    let bytes = match std::fs::read(&args.file) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{}: {error}", args.file.display());
            return 1;
        }
    };
    let decoded = decode(&bytes);
    if decoded
        .warnings
        .iter()
        .any(|warning| matches!(warning, DecodeWarning::CompressedInput))
    {
        eprintln!("compressed OFX input is not supported");
        return 1;
    }
    if let Err(error) = io::stdout().write_all(decoded.text.as_bytes())
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        eprintln!("{error}");
        return 1;
    }
    for warning in decoded.warnings {
        eprintln!("warning: {}", warning_json(&warning).message);
    }
    0
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let code = match cli.command {
        Command::Inspect(args) => inspect(args, cli.no_color),
        Command::Validate(args) => validate(args, cli.no_color),
        Command::Decode(args) => decode_command(args),
    };
    ExitCode::from(code)
}
