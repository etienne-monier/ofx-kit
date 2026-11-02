# Release policy

## Versioning

The workspace uses one version for all packages. The version is declared once
in the root `Cargo.toml` under `[workspace.package]`; member manifests inherit
it with `version.workspace = true`.

The project follows Semantic Versioning. While the major version is `0`, a
minor release may contain breaking API changes. The library and CLI are
released in lockstep.

## Release checklist

1. Run all quality gates documented in `AGENTS.md` and `ROADMAP.md`.
2. Update the version in `[workspace.package]`.
3. Add the release date and changes to `CHANGELOG.md`.
4. Commit with `chore: release vX.Y.Z` and create tag `vX.Y.Z`.
5. Publish `ofx-kit` first, then `ofx-kit-cli`.
6. Verify docs.rs and the GitHub Pages deployment.

The old `ofx-rs` package is not replaced in place; `ofx-kit` is its successor.
