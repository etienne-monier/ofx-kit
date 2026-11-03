# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/).

## [Unreleased]

### Added

- `ofx inspect`, `ofx validate`, and `ofx decode` commands.
- Stable JSON output for inspection and validation.
- Byte-oriented decoding and parsing with encoding detection and warnings.
- Structural SGML-to-XML conversion through the library and `ofx convert`.
- A tokenizer-backed OFX 1.x SGML normalizer with shared schema relationships.
- `parse_with_warnings` and `parse_bytes_with_warnings` for non-fatal parse diagnostics.
- Configurable unknown-element policies, strict structural parsing, source
  positions, and plural message-set accessors.
- Fuzz targets and a weekly scheduled fuzz-build workflow.
- A supported OFX subset reference page.

### Changed

- XML errors now carry an element path and a structured, non-exhaustive error kind.
- SGML text is preserved verbatim during normalization, including UTF-8 and CRLF input.
- `ParsedDocument` also carries byte-decoding warnings, and repeated top-level
  message sets are preserved instead of overwritten.

## [0.3.0] - 2026-11-02

### Added

- The `ofx-kit` Cargo package and `ofx_kit` Rust import path.
- The `ofx-kit-cli` package with the `ofx` binary skeleton.
- An Astro Starlight documentation site and GitHub Pages deployment workflow.
- Workspace-level release metadata and this changelog.

### Changed

- Moved the library and its tests into the Cargo workspace.

This project remains a fork of [Govcraft/ofx-rs](https://github.com/Govcraft/ofx-rs).
