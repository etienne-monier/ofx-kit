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
```

`inspect` displays the format, version, encoding warnings, signon status,
accounts, balances, periods, and transactions. Use `--account`, `--from`,
`--to`, and `--limit` to filter transactions. `--limit` applies independently
to each statement and `--account` matches `ACCTID` exactly.

`validate` performs structural validation only. A non-zero OFX signon status is
reported but does not make an otherwise well-formed file invalid.

`decode` writes decoded UTF-8 content to stdout and warnings to stderr.

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
