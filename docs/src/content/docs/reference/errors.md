---
title: Errors and warnings
description: Structured parsing errors and non-fatal diagnostics.
---

## XML errors

`XmlError` contains an `XmlErrorKind` and an `ElementPath`. Use `kind()` and
`path()` to inspect errors without parsing their display text. The error kinds
include malformed XML, unexpected elements, missing required elements, and
invalid content.

Element paths use `/` separators, for example:

```text
OFX/BANKMSGSRSV1/STMTTRNRS/STMTRS/BANKTRANLIST/STMTTRN
```

## Non-fatal warnings

`parse_with_warnings` and `parse_bytes_with_warnings` return a `ParsedDocument`
containing the typed document and a `Vec<ParseWarning>`. Warnings include:

- unknown or misplaced elements;
- unmatched closing tags and elements closed at EOF;
- raw ampersands and unknown entities repaired during SGML normalization; and
- malformed SGML tokenizer input.

Warnings include an `ElementPath` whenever the location can be determined.
They do not make an otherwise parseable document fail. Use `parse` when these
diagnostics are not needed.
