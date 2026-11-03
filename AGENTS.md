# Agent instructions

## Project

`ofx-kit` is a pure Rust parser for OFX 1.x (SGML) and OFX 2.x (XML). The
library package is `ofx-kit` and the import path is `ofx_kit`.

The supported product scope is bank statements, plus the existing credit-card
and investment-statement support. Investment support currently extracts only
banking transactions from `INVBANKTRAN` / `STMTTRN`; this is not a full OFX
implementation.

## Commands

- `cargo test --workspace --locked` — full test suite.
- `cargo test --workspace --locked --test parse_statement_test` — one integration file.
- `cargo test --workspace --locked parse_simple_bank_statement` — one test by name.
- `cargo test --workspace --locked -- --nocapture` — show test-vector output.
- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets --locked -- -D warnings`
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked`
- `cargo deny check` — configuration is in `deny.toml`.

CI validates formatting, Clippy, tests, Rust 1.90 MSRV, public/private docs,
and cargo-deny. Run the relevant checks before claiming validation.

## Architecture

- `crates/ofx-kit/src/lib.rs` re-exports the public parse APIs from the private
  `crates/ofx-kit/src/parser.rs` module.
- Parsing flows through `header::parse_header`, then normalizes OFX 1.x with
  `crates/ofx-kit/src/sgml/{tokenizer,normalize}.rs`, and parses the resulting XML with `quick-xml` through
  `crates/ofx-kit/src/xml/reader.rs`.
- `crates/ofx-kit/src/parser.rs` is a hand-written aggregate parser. `crates/ofx-kit/src/aggregates/` holds
  composite data, `crates/ofx-kit/src/types/` holds validated primitives/enums, and
  `crates/ofx-kit/src/document/` holds top-level message sets.
- The library currently performs no I/O. `parse(&str)` accepts UTF-8 text and
  `parse_bytes(&[u8])` decodes supported source encodings; keep file I/O in the
  CLI.

## SGML and data hazards

- OFX 1.x omits closing tags. When adding an aggregate or leaf, update the
  declarative hierarchy in `crates/ofx-kit/src/schema.rs` (`AGGREGATES` and its
  child/leaf relationships). The SGML normalizer in
  `crates/ofx-kit/src/sgml/normalize.rs` uses this schema to infer closing tags.
- A wrong SGML relationship can close a parent early and drop an entire
  subtree. `ACCTKEY` inside `BANKACCTFROM` is a regression case.
- Preserve text as UTF-8 slices. Byte-by-byte `char` conversion breaks accented
  text and creates mojibake.
- Monetary values use `rust_decimal::Decimal` through `OfxAmount`; never use
  `f64` for financial data.
- Unknown elements are currently skipped. Do not claim lossless conversion by
  routing a document through `OfxDocument`.

## Code and tests

- `crates/ofx-kit/src/lib.rs` forbids unsafe code and denies `unwrap`/`expect` in production;
  use `?`, `ok_or`, and `ok_or_else`.
- Error enums are `#[non_exhaustive]`; preserve `Display`, `Error`, and `From`
  implementations when extending them.
- Unit tests live beside implementation; integration tests are in
  `crates/ofx-kit/tests/` and use fixtures from `crates/ofx-kit/tests/fixtures/`
  via `include_str!`.
- `parse_test_vectors.rs` prints a fixture report only with `--nocapture`.
- `proptest` is declared as a dev-dependency but is not currently used.
  There is no CLI, raw-byte encoding suite, or fuzzing yet.

## Repository quirks

- `.gitignore` ignores Markdown by default; `AGENTS.md`, `ROADMAP.md`, and
  `README.md` are explicitly included.
- `Cargo.lock` is ignored but tracked. Do not delete it casually.
- `spec/` contains third-party OFX PDFs and is excluded from published crates.
- The crate uses edition 2024 and requires Rust 1.90 or newer.
