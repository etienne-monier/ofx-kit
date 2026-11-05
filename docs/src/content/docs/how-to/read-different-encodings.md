---
title: Read OFX files with different encodings
description: Decode and parse OFX files when their source encoding is unknown.
---

## Parse raw bytes

Use `parse_bytes` when the input comes from a file, network response, or other
byte source. The library detects common OFX encodings before parsing:

```rust
use std::{error::Error, fs};

use ofx_kit::parse_bytes;

fn main() -> Result<(), Box<dyn Error>> {
    let bytes = fs::read("statement.ofx")?;
    let document = parse_bytes(&bytes)?;

    println!("OFX version: {}", document.header().version());
    Ok(())
}
```

Supported inputs include UTF-8, UTF-16LE, UTF-16BE, Windows-1252, Big5,
GB18030, Shift_JIS, and EUC-KR. UTF-8 text that is already decoded can be
parsed with [`parse`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.parse.html).

## Inspect decoding warnings

Use `decode` when you need the detected encoding without parsing the document:

```rust
use std::{error::Error, fs};

use ofx_kit::decode;

fn main() -> Result<(), Box<dyn Error>> {
    let bytes = fs::read("statement.ofx")?;
    let decoded = decode(&bytes);

    println!("Detected encoding: {}", decoded.encoding);
    for warning in decoded.warnings {
        eprintln!("decoding warning: {warning:?}");
    }

    Ok(())
}
```

`parse_bytes` intentionally returns only the typed document. Use
[`parse_bytes_with_warnings`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.parse_bytes_with_warnings.html)
when you need both the parsed document and decoding warnings.

Compressed input is reported as unsupported rather than being silently
decompressed. Applications that accept uploads should handle
`OfxError::UnsupportedCompression` explicitly.
