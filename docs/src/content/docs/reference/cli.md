---
title: CLI reference
description: Commands and output formats for the ofx command-line tool.
---

## Commands

```text
ofx inspect statement.ofx
ofx inspect statement.ofx --json
ofx validate statement.ofx
ofx validate statement.ofx --json
ofx decode statement.ofx
ofx convert statement.ofx
ofx convert statement.ofx -o statement.xml --pretty
ofx convert statement.ofx --to 220 --check
```

`inspect` displays the format, version, encoding warnings, signon status,
accounts, balances, periods, and transactions. Use `--account`, `--from`,
`--to`, and `--limit` to filter transactions. `--limit` applies independently
to each statement and `--account` matches `ACCTID` exactly.

`validate` performs structural validation only. A non-zero OFX signon status is
reported but does not make an otherwise well-formed file invalid.

`decode` writes decoded UTF-8 content to stdout and warnings to stderr.

`convert` performs a structural SGML 1.x to XML conversion. It preserves
unknown elements and does not serialize through the typed document model. Use
`-o` to write a file, `--pretty` for indentation, and `--check` to validate
without writing output. `--to` changes only the VERSION metadata in the
generated header; it does not perform a semantic OFX migration.
Warnings for discarded legacy header metadata and ambiguous unknown elements
are written to standard error while the XML remains on standard output.

## JSON output

JSON output has `schema_version: 1`. Monetary values are strings and dates use
RFC 3339. `inspect --json` returns a report with `source`, `header`, `signon`,
and `statements`. `validate --json` returns `valid`, a source description, a
summary, and `error` (null on success).

When JSON output cannot parse the input, the command still writes a JSON error
object to stdout and exits with code 1. Usage errors are printed as text and
exit with code 2.

## Exit codes and color

| Code | Meaning |
| --- | --- |
| 0 | Success, including non-fatal decode warnings |
| 1 | I/O, compression, or invalid OFX data |
| 2 | Invalid command-line usage |

Text color is used only for a terminal. Set `NO_COLOR` or pass `--no-color` to
disable it. JSON output never contains ANSI escape sequences.
