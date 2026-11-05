---
title: Library API guide
description: Choose the right ofx-kit API for parsing, diagnostics, and conversion.
---

## Choose an entry point

| Situation | Recommended API | Result |
| --- | --- | --- |
| You already have UTF-8 text | [`parse`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.parse.html) | Typed `OfxDocument` |
| You have file or network bytes | [`parse_bytes`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.parse_bytes.html) | Typed `OfxDocument` with encoding detection |
| You need diagnostics | [`parse_with_warnings`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.parse_with_warnings.html) or [`parse_bytes_with_warnings`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.parse_bytes_with_warnings.html) | Document plus parser and, for bytes, decoding warnings |
| You need strict import rules | `parse_with_options` or `parse_bytes_with_options` | Document plus configurable policies |
| You only need encoding detection | [`decode`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.decode.html) | UTF-8 text, detected encoding, and decoding warnings |
| You need structural XML conversion | [`convert_str`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.convert_str.html) or [`convert_bytes`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.convert_bytes.html) | UTF-8 XML, preserving fields outside the typed model |

The parser and converter are separate paths. Parsing produces the typed model
and intentionally skips unsupported elements. Conversion preserves unknown
elements and does not round-trip through `OfxDocument`.

## Recommended import paths

The crate re-exports the most common parsing APIs at the root:

```rust
use ofx_kit::{parse_bytes, OfxDocument, OfxError};
```

Domain types are grouped by role:

| Module | Examples |
| --- | --- |
| `ofx_kit::document` | `OfxDocument`, `BankingMessageSet`, `SignonResponse` |
| `ofx_kit::aggregates` | `StatementResponse`, `StatementTransaction`, `Balance` |
| `ofx_kit::types` | `OfxAmount`, `OfxDateTime`, `CurrencyCode`, `TransactionType` |
| `ofx_kit::header` | `OfxHeader`, `OfxVersion`, `SecurityLevel` |
| `ofx_kit::diagnostics` | `ElementPath`, `ParseWarning`, `SourcePosition` |

## Error handling

All parsing errors are returned as [`OfxError`](https://docs.rs/ofx-kit/latest/ofx_kit/enum.OfxError.html).
Its variants identify the failing boundary:

- `Header`: the OFX header is missing or invalid;
- `Xml`: the document structure or content is invalid;
- `Aggregate`: a required typed field is missing or invalid;
- `UnsupportedCompression`: the input is compressed;
- `StrictViolation`: strict mode rejected a structural warning.

Error enums are non-exhaustive. Match a known variant and keep a wildcard arm
so applications remain compatible with future releases.

## Financial and date values

Use the domain types rather than parsing raw strings again:

```rust
use ofx_kit::types::{OfxAmount, OfxDateTime};

let amount: OfxAmount = "-50.00".parse()?;
assert!(amount.is_negative());

let posted: OfxDateTime = "20230115".parse()?;
println!("{posted} — {amount}");
# Ok::<(), Box<dyn std::error::Error>>(())
```

`OfxAmount` uses exact decimal arithmetic. `OfxDateTime` accepts OFX's
right-truncated date/time syntax and optional timezone offsets.
