---
title: Convert SGML OFX to XML
description: Convert an OFX 1.x file to a well-formed XML document.
---

Use the `convert` command to turn a legacy OFX 1.x file into UTF-8 XML:

```bash
ofx convert statement.ofx -o statement.xml --pretty
```

Without `-o`, the XML is written to standard output. Use `--check` when you
only want to decode, convert, and validate the result:

```bash
ofx convert statement.ofx --check
```

The conversion is structural. Missing SGML closing tags are inferred using the
OFX schema, text is escaped for XML, and unknown elements are retained. The
conversion does not pass through `OfxDocument`, so fields outside the typed
parser subset are not discarded.

The source version is kept by default. To change only the header metadata, use
`--to`:

```bash
ofx convert statement.ofx --to 220 -o statement-220.xml
```

This is not a semantic OFX migration. The element vocabulary and values are
left unchanged. Decode and structural warnings are written to standard error.
Warnings also identify legacy header fields such as `ENCODING` and `CHARSET`
that are not copied into the UTF-8 XML header. Ambiguous unknown elements are
retained using the tolerant structural interpretation and are reported on
standard error.
