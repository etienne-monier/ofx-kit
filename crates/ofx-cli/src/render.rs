//! CLI render support.
//!
//! This private module contains the render concerns for the OFX command-line tool.

use super::*;

pub(crate) struct Style {
    pub(crate) enabled: bool,
}

impl Style {
    pub(crate) fn new(no_color: bool) -> Self {
        let no_color_env = std::env::var_os("NO_COLOR").is_some_and(|value| !value.is_empty());
        Self {
            enabled: !no_color && !no_color_env && io::stdout().is_terminal(),
        }
    }

    pub(crate) fn paint(&self, code: &str, text: impl std::fmt::Display) -> String {
        if self.enabled {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

pub(crate) fn render_text(report: &ReportJson, no_color: bool) {
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
