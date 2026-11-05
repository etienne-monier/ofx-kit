---
title: Reference
description: API and format reference for ofx-kit.
---

Use the [supported OFX subset](./supported-ofx-subset/) to check whether a
document is within the typed model, and the [CLI reference](./cli/) for
commands, filters, JSON, and exit codes. The complete Rust API reference is
available on [docs.rs](https://docs.rs/ofx-kit).

See the [library API guide](./library-api/) for choosing an entry point and
the [data model guide](./data-model/) for the structure returned by the parser.

## Main library API

| Need | API |
| --- | --- |
| Parse UTF-8 text | [`parse`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.parse.html) |
| Parse raw file bytes | [`parse_bytes`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.parse_bytes.html) |
| Keep parser and decoding warnings | [`parse_bytes_with_warnings`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.parse_bytes_with_warnings.html) |
| Configure unknown-element and strict handling | [`ParseOptions`](https://docs.rs/ofx-kit/latest/ofx_kit/struct.ParseOptions.html) |
| Decode without parsing | [`decode`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.decode.html) |
| Convert structurally to XML | [`convert_bytes`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.convert_bytes.html) |

Parsing returns an [`OfxDocument`](https://docs.rs/ofx-kit/latest/ofx_kit/struct.OfxDocument.html):

```text
OfxDocument
├── header()
├── signon()
├── banking() / bankings()
│   └── statement_responses()
│       └── StatementResponse
│           ├── bank_account()
│           ├── transaction_list()
│           └── balances
├── credit_card() / credit_cards()
└── investment() / investments()
```

The singular accessors return the first message set for convenience. Use the
plural accessors when a document can contain repeated message sets.
