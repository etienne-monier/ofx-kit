---
title: Handle parser warnings
description: Inspect non-fatal parsing and decoding diagnostics.
---

The short `parse` and `parse_bytes` functions discard non-fatal warnings. Use
the warning-aware APIs when an application needs to report or audit tolerant
parsing decisions.

## Collect warnings

```rust
use std::{error::Error, fs};

use ofx_kit::parse_bytes_with_warnings;

fn main() -> Result<(), Box<dyn Error>> {
    let bytes = fs::read("statement.ofx")?;
    let parsed = parse_bytes_with_warnings(&bytes)?;

    for warning in &parsed.decode_warnings {
        eprintln!("decoding warning: {warning:?}");
    }
    for warning in &parsed.warnings {
        eprintln!("parser warning: {warning}");
    }

    let document = parsed.document;
    println!("OFX version: {}", document.header().version());
    Ok(())
}
```

Parser warnings can describe unknown or misplaced elements, unmatched or
unclosed SGML tags, repaired entities, and malformed SGML input. They include
an element path and a source position whenever the location is known.

## Change the unknown-element policy

By default, unknown elements are skipped and reported as warnings. Applications
can ignore them or turn them into errors:

```rust
use std::{error::Error, fs};

use ofx_kit::{ParseOptions, UnknownElementPolicy, parse_with_options};

fn main() -> Result<(), Box<dyn Error>> {
    let input = fs::read_to_string("statement.ofx")?;
    let options = ParseOptions::default()
        .with_unknown_elements(UnknownElementPolicy::Error)
        .with_strict(true);

    let _parsed = parse_with_options(&input, &options)?;
    Ok(())
}
```

`strict` upgrades structural SGML diagnostics to errors. Text-escaping
warnings remain warnings because the resulting text is still made well-formed.
Use strict mode when malformed structure should reject an import; use the
default warning mode when real-world provider files need tolerant handling.
