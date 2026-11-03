//! CLI commands support.
//!
//! This private module contains the commands concerns for the OFX command-line tool.

use super::*;

pub(crate) fn print_error_json(error: &OfxError) {
    println!(
        "{}",
        serde_json::to_string_pretty(&ErrorEnvelope {
            schema_version: 1,
            error: error_json(error)
        })
        .unwrap_or_else(|_| "{\"error\":{}}".to_owned())
    );
}

pub(crate) fn inspect(args: InspectArgs, no_color: bool) -> u8 {
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

pub(crate) fn validate(args: ValidateArgs, no_color: bool) -> u8 {
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
                .bankings()
                .iter()
                .map(|v| v.statement_responses().len())
                .sum::<usize>()
                + document
                    .credit_cards()
                    .iter()
                    .map(|v| v.statement_responses().len())
                    .sum::<usize>()
                + document
                    .investments()
                    .iter()
                    .map(|v| v.statement_responses().len())
                    .sum::<usize>();
            let transactions = document
                .bankings()
                .iter()
                .flat_map(|v| v.statement_responses())
                .filter_map(|v| v.response()?.transaction_list())
                .map(|v| v.len())
                .sum::<usize>()
                + document
                    .credit_cards()
                    .iter()
                    .flat_map(|v| v.statement_responses())
                    .filter_map(|v| v.response()?.transaction_list())
                    .map(|v| v.len())
                    .sum::<usize>()
                + document
                    .investments()
                    .iter()
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

pub(crate) fn decode_command(args: InputArgs) -> u8 {
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

pub(crate) fn convert_command(args: ConvertArgs) -> u8 {
    let bytes = match std::fs::read(&args.file) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{}: {error}", args.file.display());
            return 1;
        }
    };
    let result = match convert_bytes(
        &bytes,
        &ConversionOptions {
            target_version: args.to,
            pretty: args.pretty,
        },
    ) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("{}: {error}", args.file.display());
            return 1;
        }
    };

    for warning in &result.decode_warnings {
        eprintln!("warning: {}", warning_json(warning).message);
    }
    for warning in &result.warnings {
        eprintln!("warning: {warning}");
    }
    if args.check {
        return 0;
    }

    if let Some(output) = args.output {
        if let Err(error) = std::fs::write(&output, result.xml.as_bytes()) {
            eprintln!("{}: {error}", output.display());
            return 1;
        }
        return 0;
    }

    if let Err(error) = io::stdout().write_all(result.xml.as_bytes())
        && error.kind() != io::ErrorKind::BrokenPipe
    {
        eprintln!("{error}");
        return 1;
    }
    0
}
