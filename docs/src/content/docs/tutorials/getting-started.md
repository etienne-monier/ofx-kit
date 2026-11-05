---
title: Getting started
description: Parse your first OFX document with ofx-kit.
---

## Add the library

```sh
cargo add ofx-kit
```

The library accepts text or raw bytes and does not read files itself. Use
`parse` when your application already has UTF-8 text, or `parse_bytes` when
you are reading a file or do not know its encoding in advance.

## Parse a statement

```rust
use std::{error::Error, fs};

use ofx_kit::parse_bytes;

fn main() -> Result<(), Box<dyn Error>> {
    let content = fs::read("statement.ofx")?;
    let document = parse_bytes(&content)?;

    if !document.signon().status().is_success() {
        eprintln!(
            "OFX sign-on failed with code {}",
            document.signon().status().code()
        );
    }

    if let Some(banking) = document.banking()
        && let Some(statement) = banking.statement_responses().first()
        && let Some(response) = statement.response()
    {
        println!("Account: {}", response.bank_account().account_id().as_str());

        if let Some(transactions) = response.transaction_list() {
            for transaction in transactions.transactions() {
                println!(
                    "{} {} {}",
                    transaction.date_posted(),
                    transaction.amount(),
                    transaction.name().unwrap_or("(unnamed)")
                );
            }
        }
    }

    Ok(())
}
```

The parser accepts both OFX 1.x SGML and OFX 2.x XML through the same API. A
non-successful OFX sign-on status is part of the parsed document, not a parser
error: applications should decide whether it is actionable for their use case.

For a UTF-8 string already in memory, use `parse(&content)` instead. To retain
non-fatal parser and decoding diagnostics, use
[`parse_bytes_with_warnings`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.parse_bytes_with_warnings.html).
