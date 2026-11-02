# ofx-kit Roadmap

## Vision

Make OFX bank-statement files easy to understand and use:

1. A clear documentation site for beginners and experienced Rust users.
2. A library that handles real-world OFX encodings itself.
3. An `ofx` CLI for validation, inspection, decoding, and v1-to-XML conversion.

The project intentionally supports a useful subset of OFX rather than the
whole specification.

## Decisions

- Library package: `ofx-kit`; Rust import path: `ofx_kit`.
- CLI package: `ofx-kit-cli`; installed binary: `ofx`.
- Scope: bank statements, retaining the existing credit-card and investment
  statement support. Investment support remains limited to banking
  transactions already extracted by the parser.
- Documentation: Astro Starlight with a Diátaxis structure, published on
  GitHub Pages; API reference remains on docs.rs; README remains a concise
  entry point.
- Encoding feature: `encoding`, enabled by default.
- OFX 1.x to OFX 2.x conversion is a separate convenience feature. It is not
  required to parse a document.

## Parsing and conversion

Translation is not conceptually required for parsing. OFX 1.x needs SGML
structure handling because closing tags are optional, but the resulting XML is
currently an internal compatibility step for `quick-xml`.

The future implementation must keep two paths separate:

- **Typed parsing:** decode bytes, understand SGML/XML structure, and produce
  `OfxDocument`.
- **Conversion:** decode bytes and perform a lossless structural SGML-to-XML
  transformation. It must not round-trip through `OfxDocument`, since that
  would discard unsupported or unknown fields.

## Current gaps

- The library accepts only UTF-8 `&str`; real bank-file encoding is handled by
  consumers such as Pactole.
- OFX 1.x normalization uses hard-coded aggregate/parent tables and can drop a
  subtree when an unknown tag is encountered in the wrong place.
- There is no CLI, serializer, conversion command, or structured warning API.
- Errors have no element path or source position.
- Documentation is currently limited to the README.
- There are no raw-byte encoding fixtures, CLI integration tests, or fuzz tests.

## Roadmap

### 0. Rename and foundations

- Rename the library package and imports from `ofx-rs` / `ofx_rs` to
  `ofx-kit` / `ofx_kit`.
- Add a Cargo workspace member `crates/ofx-cli` with package `ofx-kit-cli` and
  binary `ofx`.
- Add `CHANGELOG.md` and release/versioning policy.
- Add the Starlight site skeleton and GitHub Pages workflow.
- Keep the existing attribution to `Govcraft/ofx-rs`.

**Exit criteria:** workspace tests pass, `ofx --help` works, and the docs site
builds.

### 1. Encoding in the library

Add byte-oriented APIs while retaining `parse(&str)`:

```rust
pub struct DecodedSource {
    pub text: String,
    pub encoding: DetectedEncoding,
    pub warnings: Vec<DecodeWarning>,
}

pub fn decode(bytes: &[u8]) -> DecodedSource;
pub fn parse_bytes(bytes: &[u8]) -> Result<OfxDocument, OfxError>;
```

Detection order:

1. UTF-8, UTF-16LE, and UTF-16BE BOMs.
2. XML declaration encoding.
3. SGML `ENCODING:` / `CHARSET:` declarations.
4. `ENCODING:UNICODE` with an UTF-16LE body.
5. Strict UTF-8.
6. Windows-1252 fallback with a warning.

Use `encoding_rs` behind the default `encoding` feature. Expose the detected
encoding and warnings so a tolerant fallback is never silent. Enrich the
header model with the encoding-related fields needed to explain the decision.
Reject unsupported compression explicitly.

Add raw-byte fixtures for cp1252, UTF-16, BOMs, mislabeled UTF-8/cp1252, and
CJK code pages. This should allow Pactole to remove its duplicate decoder.

### 2. CLI inspection and validation

Implement:

```text
ofx inspect statement.ofx
ofx inspect statement.ofx --json
ofx validate statement.ofx
ofx decode statement.ofx
```

`inspect` should show format/version, encoding and warnings, signon status,
institution, account, currency, period, balances, and transaction rows. Add
filters such as `--account`, `--from`, `--to`, and `--limit`.

Use color only for a TTY and honor `NO_COLOR` / `--no-color`. Define exit codes:
0 for success, 1 for invalid data, and 2 for usage errors. Keep CLI I/O and
presentation dependencies out of the library package.

Add CLI integration tests and stable JSON-output tests.

### 3. Robust SGML handling

- Extract a reusable SGML tokenizer for tags, text, entities, comments, and
  processing instructions.
- Replace scattered hard-coded relationships with a shared schema used by the
  parser and converter.
- Preserve unknown tags for conversion and report them as warnings during typed
  parsing.
- Add element paths to diagnostics.
- Cover `ACCTKEY`, unknown leaves, raw ampersands, accents, CRLF, and optional
  aggregates with regression tests.

The SGML implementation should be conservative: only infer a closing tag when
the schema justifies it.

### 4. SGML 1.x to XML 2.x conversion

Implement:

```text
ofx convert statement.ofx
ofx convert statement.ofx -o statement.xml
ofx convert statement.ofx --to 220
ofx convert statement.ofx --check
```

The pipeline should decode bytes, convert the legacy header, infer missing
closing tags, preserve unknown elements, escape text correctly, optionally
pretty-print, and validate the generated XML before writing it.

Without `--to`, conversion is structural and keeps the source semantic
version. `--to 220` updates header metadata only; it is not a full semantic
OFX migration.

Use golden files, idempotence tests, reparse/semantic-equivalence tests, and
property tests. No conversion may silently lose data.

### 5. Diagnostics and API stabilization

- Add element paths and, where practical, line/column or byte offsets.
- Add parse options for unknown-element policy (`Ignore`, `Warn`, `Error`) and
  strict/tolerant behavior.
- Return parsed documents together with warnings through an opt-in API.
- Preserve multiple message sets instead of overwriting repeated sets.
- Add fuzz targets for parsing and conversion.
- Publish an explicit supported-OFX-subset matrix.
- Freeze the public API for 1.0.

## Documentation structure

Use Starlight directories following Diátaxis:

```text
docs/src/content/docs/
├── tutorials/
│   ├── getting-started.md
│   ├── parse-a-bank-statement.md
│   └── use-the-cli.md
├── how-to/
│   ├── install-ofx-kit.md
│   ├── decode-a-non-utf8-file.md
│   ├── inspect-a-statement.md
│   ├── convert-sgml-to-xml.md
│   └── troubleshoot-mojibake.md
├── reference/
│   ├── cli.md
│   ├── supported-ofx-subset.md
│   ├── encoding.md
│   └── errors.md
└── explanation/
    ├── what-is-ofx.md
    ├── ofx-1x-vs-2x.md
    ├── how-the-parser-works.md
    └── why-encoding-is-hard.md
```

Every feature should ship with tests, a Starlight page, and a runnable
example. The README should link to the site and docs.rs instead of duplicating
the full reference.

## Release targets

| Version | Target |
|---|---|
| `0.3.0` | Rename, encoding, `parse_bytes`, CLI `decode`/`inspect` MVP, Starlight skeleton |
| `0.4.0` | Robust SGML module, lossless conversion, CLI `convert`, conversion tests |
| `0.5.0` | Diagnostics, warnings, strict/tolerant modes, fuzzing, coverage matrix |
| `1.0.0` | Stable API, complete docs, release binaries, clean changelog |

## Quality gates

Every release must pass:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings
cargo test --workspace --locked
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo deny check
npm ci && npm run build              # from docs/
```

The Rust 1.90 MSRV check must remain green. New behavior requires focused
tests and documentation; encoding and conversion changes additionally require
fixture or golden-file coverage.
