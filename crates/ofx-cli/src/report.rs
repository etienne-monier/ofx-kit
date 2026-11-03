//! CLI report support.
//!
//! This private module contains the report concerns for the OFX command-line tool.

use super::*;

#[derive(Debug, Serialize)]
pub(crate) struct WarningJson {
    pub(crate) code: String,
    pub(crate) message: String,
}

pub(crate) fn warning_json(warning: &DecodeWarning) -> WarningJson {
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
pub(crate) struct SourceJson {
    pub(crate) encoding: String,
    pub(crate) declared_encoding: Option<String>,
    pub(crate) declared_charset: Option<String>,
    pub(crate) warnings: Vec<WarningJson>,
}

pub(crate) fn source_json(loaded: &Loaded) -> SourceJson {
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
pub(crate) struct ErrorJson {
    pub(crate) kind: String,
    pub(crate) message: String,
    pub(crate) causes: Vec<String>,
}

pub(crate) fn error_json(error: &OfxError) -> ErrorJson {
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
pub(crate) struct ErrorEnvelope {
    pub(crate) schema_version: u8,
    pub(crate) error: ErrorJson,
}

pub(crate) fn date_string(date: &ofx_kit::types::OfxDateTime) -> String {
    date.as_offset_date_time()
        .format(&Rfc3339)
        .unwrap_or_else(|_| date.to_string())
}

#[derive(Debug, Serialize)]
pub(crate) struct TransactionJson {
    pub(crate) date_posted: String,
    pub(crate) r#type: String,
    pub(crate) amount: String,
    pub(crate) fit_id: String,
    pub(crate) name: Option<String>,
    pub(crate) memo: Option<String>,
    pub(crate) check_number: Option<String>,
    pub(crate) reference_number: Option<String>,
    pub(crate) correction_id: Option<String>,
    pub(crate) correction_action: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct BalanceJson {
    pub(crate) amount: String,
    pub(crate) as_of: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct StatementJson {
    pub(crate) kind: String,
    pub(crate) transaction_uid: String,
    pub(crate) status: StatusJson,
    pub(crate) account: AccountJson,
    pub(crate) currency: String,
    pub(crate) period: Option<PeriodJson>,
    pub(crate) ledger_balance: Option<BalanceJson>,
    pub(crate) available_balance: Option<BalanceJson>,
    pub(crate) transaction_count: usize,
    pub(crate) transactions_truncated: bool,
    pub(crate) transactions: Vec<TransactionJson>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AccountJson {
    pub(crate) kind: String,
    pub(crate) account_id: String,
    pub(crate) bank_id: Option<String>,
    pub(crate) branch_id: Option<String>,
    pub(crate) account_type: Option<String>,
    pub(crate) broker_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct PeriodJson {
    pub(crate) start: String,
    pub(crate) end: String,
}

#[derive(Debug, Serialize)]
pub(crate) struct StatusJson {
    pub(crate) code: u32,
    pub(crate) severity: String,
    pub(crate) message: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReportJson {
    pub(crate) schema_version: u8,
    pub(crate) source: SourceJson,
    pub(crate) header: HeaderJson,
    pub(crate) signon: SignonJson,
    pub(crate) statements: Vec<StatementJson>,
}

#[derive(Debug, Serialize)]
pub(crate) struct HeaderJson {
    pub(crate) version: String,
    pub(crate) format: String,
    pub(crate) security: String,
    pub(crate) old_file_uid: String,
    pub(crate) new_file_uid: String,
    pub(crate) declared_compression: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct SignonJson {
    pub(crate) status: StatusJson,
    pub(crate) server_time: String,
    pub(crate) language: String,
    pub(crate) institution: InstitutionJson,
}

#[derive(Debug, Serialize)]
pub(crate) struct InstitutionJson {
    pub(crate) org: Option<String>,
    pub(crate) fid: Option<String>,
}

pub(crate) fn status_json(status: &ofx_kit::aggregates::Status) -> StatusJson {
    StatusJson {
        code: status.code(),
        severity: status.severity().to_string(),
        message: status.message().map(str::to_owned),
    }
}

pub(crate) fn transactions_json(
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

pub(crate) fn report(loaded: &Loaded, args: &InspectArgs) -> Result<ReportJson, OfxError> {
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
        .bankings()
        .iter()
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
            .credit_cards()
            .iter()
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
            .investments()
            .iter()
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
