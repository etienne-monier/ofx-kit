---
title: How the parser works
description: How ofx-kit handles OFX XML and legacy SGML documents.
---

`ofx-kit` parses OFX 2.x as XML. OFX 1.x files use an SGML dialect in which
closing tags are often omitted, so the library first tokenizes and normalizes
the body into well-formed XML before applying the typed parser.

## Conservative SGML normalization

The normalizer uses the same internal schema as the typed parser. It infers a
closing tag only when the schema proves that the next element cannot be a child
of the current element. This is important for fields such as `ACCTKEY`, which
are leaves inside `BANKACCTFROM` but must not close that aggregate.

Unknown elements are retained in the normalized XML. Unknown leaves are closed
when the next element begins; unknown aggregates are closed only by an explicit
closing tag or at end of input.

Text is copied as UTF-8 without trimming during normalization. Raw ampersands
and non-XML named entities are escaped so the resulting document remains
well-formed, and are reported as warnings by the warning-aware API.

## Warnings

Use `parse_with_warnings` when callers need to inspect unknown elements,
unmatched or unclosed tags, entity repairs, or misplaced elements:

```rust
use ofx_kit::parse_with_warnings;

let parsed = parse_with_warnings(input)?;
for warning in parsed.warnings {
    eprintln!("{warning}");
}
```

The regular `parse` API remains available and discards non-fatal warnings.
