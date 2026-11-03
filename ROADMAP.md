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

- Diagnostics have element paths and positions for token-level diagnostics,
  but not every XML error or synthesized SGML close has a source position yet.
- The documentation site is only partially populated; several planned
  tutorials, how-to guides, and reference pages are still missing.

## Roadmap

### 0. Rename and foundations

**Status: completed.**

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

**Status: completed.**

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

**Status: completed.**

Implement:

```text
ofx inspect statement.ofx
ofx inspect statement.ofx --json
ofx validate statement.ofx
ofx decode statement.ofx
ofx validate statement.ofx --json
```

`inspect` should show format/version, encoding and warnings, signon status,
institution, account, currency, period, balances, and transaction rows. Add
filters such as `--account`, `--from`, `--to`, and `--limit`.

Use color only for a TTY and honor `NO_COLOR` / `--no-color`. Define exit codes:
0 for success, 1 for invalid data, and 2 for usage errors. Keep CLI I/O and
presentation dependencies out of the library package.

Add CLI integration tests and stable JSON-output tests.

The CLI also supports credit-card and investment statements, exact `--account`
filtering, date filtering, per-statement transaction limits, structural JSON
validation reports, TTY-aware color, `NO_COLOR`, and exit codes 0/1/2.

### 3. Robust SGML handling

**Status: completed.**

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

**Status: completed.**

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

**Status: in progress.**

Already implemented as part of the preceding work:

- `ParseWarning` with a separate `ParseWarningKind`, element paths, and
  optional source positions.
- `ParseOptions` with `Ignore`, `Warn`, and `Error` unknown-element policies.
- Strict mode for structural SGML diagnostics.
- Opt-in parsed documents with non-fatal parse warnings through
  `parse_with_warnings` and `parse_bytes_with_warnings`.
- Preservation of repeated top-level message sets with plural accessors.
- Fuzz targets and a scheduled CI fuzz-build job.
- An initial supported-OFX-subset reference matrix.

Remaining work:

- Complete source-position propagation for all XML errors and synthesized SGML
  closures, and expose positions in CLI diagnostics.
- Add focused regression fixtures for repeated message sets and combined byte
  decoding/parsing warnings.
- Freeze the public API for 1.0.

## Detailed remaining implementation plan

The following plan records the implementation sequence for completing point 5.
The changes already present in the working tree should be stabilized first,
then completed in the order below.

### Phase 0 — Stabilize the current implementation

- Review the current diff and split it into coherent changesets:
  diagnostics and parser options, repeated message sets, combined decoding
  warnings, partial source positions, fuzzing, and documentation.
- Correct documentation that still describes decoding and parsing warnings as
  separate APIs.
- Verify links between the error-handling and supported-subset reference pages.
- Re-run the complete validation suite before adding more public behavior.

**Exit criteria:** the current implementation is internally consistent, all
existing tests pass, and no new behavior is undocumented.

### Phase 1 — Complete source-position propagation

#### Position semantics

Document and preserve the following contract:

- `byte_offset` is zero-based in the decoded UTF-8 source;
- `line` and `column` are one-based;
- `column` is measured in bytes, consistently with `byte_offset`;
- positions returned by `parse_bytes` refer to the decoded source rather than
  the original encoded byte stream.

#### Source mapping

Introduce an internal source-map abstraction that records line starts and maps
normalized SGML output ranges back to source ranges. The mapping must cover:

- escaped text;
- rewritten entities;
- opening and explicit closing tags;
- synthesized closing tags;
- warnings and errors generated while parsing normalized XML.

The header parser must also expose or preserve the body offset so positions are
absolute relative to the complete input, not only the body slice.

#### XML reader and SGML normalizer

- Replace the current XML position heuristic with the exact event start
  offset captured before reading each event.
- Attach positions to all `XmlError` paths: malformed XML, missing elements,
  invalid content, unexpected elements, skipped-element failures, and unknown
  elements rejected by policy.
- Store the opening position in each SGML normalization frame so
  `UnclosedElement` points to the opening tag rather than end-of-input.
- Ensure every token-level warning has a position whenever the tokenizer can
  determine one.
- Keep `position: None` as the documented fallback for synthesized or
  otherwise unmappable locations.

#### Tests

Add focused tests for:

- LF and CRLF line endings;
- UTF-8 multi-byte text;
- SGML warnings mapped back to original input offsets;
- XML malformed, missing, invalid-content, unexpected, and unknown-element
  errors;
- UTF-8 BOM and UTF-16 byte parsing;
- warnings near the beginning, middle, and end of large inputs.

**Exit criteria:** all parser warnings and XML errors have a source position
when their origin is known, and SGML positions refer to the original source.

### Phase 2 — Expose parser diagnostics in the CLI

- Refactor CLI loading so decoding, parsing, and their diagnostics are
  collected once without decoding or parsing the same file twice.
- Add `--strict` and `--unknown-elements <ignore|warn|error>` to the relevant
  parsing commands.
- Print parsing warnings alongside decoding warnings for text output,
  including path and `line:column (byte N)` when available.
- Extend JSON diagnostics with an origin (`decode`, `parse`, or `conversion`),
  code, message, path, and optional source position.
- Include XML error paths and positions in structured error output.
- Update CLI integration tests for text output, JSON output, strict failures,
  and unknown-element rejection.
- Update the CLI and error-handling documentation.

The existing JSON schema should remain stable where possible. If a unified
diagnostics array requires a schema change, make it explicit and document the
migration rather than silently changing the meaning of `schema_version`.

**Exit criteria:** no parsing diagnostic is lost by the CLI, and text and JSON
outputs expose equivalent diagnostic information.

### Phase 3 — Complete regression coverage

Add tests covering:

- repeated banking, credit-card, and investment message sets;
- preservation of per-type ordering;
- singular accessors returning the first set for compatibility;
- `Ignore`, `Warn`, and `Error` unknown-element policies;
- strict conversion of structural SGML warnings into errors;
- strict mode retaining `RawAmpersand` and `UnknownEntity` as warnings;
- byte parsing that returns both decoding and parsing warnings;
- interactions between options, repeated message sets, and CLI reporting.

**Exit criteria:** every new public API and every policy decision has a
regression test.

### Phase 4 — Harden fuzzing

- Install `cargo-fuzz` and run `cargo fuzz build` locally for all targets.
- Run the `parse_bytes`, `parse_str`, and `convert_bytes` targets against the
  existing fixtures and a small seed corpus.
- Add a `fuzz/README.md` describing build, run, crash reproduction, and
  triage commands.
- Keep fuzzing out of mandatory pull-request and MSRV checks, but make the
  weekly scheduled job report failures visibly instead of silently masking
  them.
- Confirm that the fuzz package remains excluded from the workspace and does
  not affect normal workspace commands.

**Exit criteria:** fuzz targets build locally, have a documented workflow,
and the scheduled CI job provides actionable failure visibility.

### Phase 5 — Prepare release `0.5.0`

Before releasing, review and document the public API:

- `ParseWarning`, `ParseWarningKind`, and `SourcePosition`;
- `ParseOptions` and `UnknownElementPolicy`;
- `parse_with_options` and `parse_bytes_with_options`;
- `XmlError::position()`;
- plural message-set accessors and singular compatibility accessors;
- the semantics of `with_banking`, `with_credit_card`, and
  `with_investment` as additive builders.

Run compatibility and packaging checks:

- `cargo test --workspace --locked --no-default-features`;
- `cargo package --workspace` or equivalent publish dry runs;
- semver compatibility checks against the release baseline;
- MSRV, documentation, Clippy, cargo-deny, fuzz-build, and site-build checks.

Then:

- update the workspace and CLI dependency version to `0.5.0`;
- move the relevant `[Unreleased]` changelog entries into `[0.5.0]`;
- add a migration section covering warning pattern matching, plural message
  sets, parser options, and source positions.

**Exit criteria:** `0.5.0` is a documented and reproducible API stabilization
release.

### Phase 6 — Prepare release `1.0.0`

After the `0.5.0` feedback period:

- freeze the public diagnostics, options, and message-set APIs;
- remove or hide experimental APIs that should not be part of the stable
  surface;
- verify `#[non_exhaustive]` usage and public documentation;
- update the supported-subset matrix and migration guide;
- require green tests, Clippy, docs, MSRV, cargo-deny, fuzz, and site builds;
- publish `1.0.0` with an explicit compatibility policy.

After `1.0.0`, incompatible public API changes require a new major version.

### Recommended execution order

| Order | Work package | Main outcome |
| --- | --- | --- |
| 0 | Stabilization | Current implementation reviewed and documented |
| 1 | Source positions | Complete SGML-to-source and XML position mapping |
| 2 | CLI diagnostics | Strict/options support and unified diagnostic reporting |
| 3 | Regression tests | Public behavior protected by focused fixtures |
| 4 | Fuzzing | Local build, corpus, and visible scheduled CI |
| 5 | `0.5.0` | API review, semver checks, packaging, and migration guide |
| 6 | `1.0.0` | Final API freeze and compatibility commitment |

### Final validation checklist

```text
cargo fmt --all -- --check
cargo test --workspace --locked
cargo test --workspace --locked --no-default-features
cargo clippy --workspace --all-targets --locked -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked
cargo deny check
cargo package --workspace
cargo fuzz build
npm ci && npm run build   # from docs/
```

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
