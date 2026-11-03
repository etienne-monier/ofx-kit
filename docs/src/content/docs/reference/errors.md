---
title: Errors and warnings
description: Structured parsing errors and non-fatal diagnostics.
---

## XML errors

`XmlError` contains an `XmlErrorKind`, an `ElementPath`, and an optional
`SourcePosition`. Use `kind()`, `path()`, and `position()` to inspect errors
without parsing their display text. The error kinds include malformed XML,
unexpected elements, unknown elements, missing required elements, and invalid
content.

Element paths use `/` separators, for example:

```text
OFX/BANKMSGSRSV1/STMTTRNRS/STMTRS/BANKTRANLIST/STMTTRN
```

## Non-fatal warnings

`parse_with_warnings` and `parse_bytes_with_warnings` return a `ParsedDocument`
containing the typed document, parse warnings, and (for byte input) decode
warnings. Each `ParseWarning` has a `ParseWarningKind`, an `ElementPath`, and
an optional `SourcePosition`. Warnings include:

- unknown or misplaced elements;
- unmatched closing tags and elements closed at EOF;
- raw ampersands and unknown entities repaired during SGML normalization; and
- malformed SGML tokenizer input.

Warnings include an `ElementPath` and a source position whenever the location
can be determined. They do not make an otherwise parseable document fail. Use
`ParseOptions` to select `Ignore`, `Warn`, or `Error` for unknown elements. Its
strict mode upgrades structural SGML warnings to errors while retaining text
escaping warnings. Use `parse` when diagnostics are not needed.
