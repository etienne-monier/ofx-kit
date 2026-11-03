---
title: Supported OFX subset
description: The OFX formats and message sets supported by ofx-kit.
---

`ofx-kit` intentionally supports the statement-oriented part of OFX rather
than the complete protocol. Unknown fields can be preserved by structural
conversion, but are not exposed by the typed document model.

## File formats and encodings

| Area | Support |
| --- | --- |
| OFX 1.x | SGML with optional closing tags; normalized before typed parsing |
| OFX 2.x | XML parsed directly |
| UTF-8 | Yes, including UTF-8 BOM |
| UTF-16 | UTF-16LE and UTF-16BE, with BOM; SGML `ENCODING:UNICODE` is UTF-16LE |
| Windows-1252 | Yes, including tolerant fallback with a warning |
| Common CJK encodings | Big5, GB18030, Shift_JIS, and EUC-KR |
| Compression | Not supported; reported explicitly |

Use [`parse_bytes`](https://docs.rs/ofx-kit/latest/ofx_kit/fn.parse_bytes.html)
for files whose encoding is not known in advance. Use `convert_bytes` when a
lossless structural SGML-to-XML transformation is required.

## Message sets

| Message set | Status | Scope |
| --- | --- | --- |
| `SIGNONMSGSRSV1` | Supported | Sign-on status, server time, language, and institution |
| `BANKMSGSRSV1` | Supported | Bank accounts, balances, periods, and banking transactions |
| `CREDITCARDMSGSRSV1` | Supported | Credit-card accounts, balances, periods, and transactions |
| `INVSTMTMSGSRSV1` | Partial | Investment account metadata and banking transactions from `INVBANKTRAN` / `STMTTRN` |

Repeated message sets are preserved and are available through the plural
accessors `bankings()`, `credit_cards()`, and `investments()`. The singular
accessors return the first set for compatibility.

## Statement data

The typed model covers the following common statement data:

- response status and transaction identifiers;
- account identifiers, bank/branch identifiers, account types, and broker
  identifiers where applicable;
- default currency;
- transaction date, type, amount, FITID, name, memo, check/reference number,
  and correction fields;
- statement period;
- ledger, available, and additional balances where defined by the statement
  type;
- sign-on institution information.

Financial amounts use decimal arithmetic; they are never represented as
floating-point values.

## Outside the current scope

The following OFX areas are not currently represented by the typed model:

- bill pay and payment schedules;
- wire transfer and electronic-payment message sets;
- customer/profile, email, and server-capability message sets;
- tax, loan, and full investment-trade support;
- full 401(k) and investment-position semantics;
- lossless typed round-tripping through `OfxDocument`.

The schema used by the parser and the SGML normalizer is the implementation
source of truth. Structural conversion does not use this typed subset and
preserves unknown elements instead of discarding them.

## Unknown elements and diagnostics

Parsing defaults to warning when an unknown element is skipped. Applications
can select `Ignore`, `Warn`, or `Error` with `ParseOptions`. Strict mode turns
structural SGML diagnostics into errors while retaining text-escaping warnings
as warnings. See the [error handling reference](./errors/) for details.
