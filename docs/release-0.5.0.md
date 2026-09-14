# 0.5.0 release-candidate audit — 2026-09-14

This checkpoint covers the local `release/v0.5.0` worktree based on revision
`04b4e2ae1d8de8eeb8ecbccb5159c87ea775173c`. It is evidence for a release
candidate, not evidence that GitHub controls, cross-platform CI, or production
signing have completed.

## Behavior covered

- Exact explicit project scope, including nested directories, path escapes, and
  rejection of a symlink supplied as the project directory.
- Git-compatible project and nested `.gitignore` parsing for recursive patterns,
  classes, escaped leading characters, and negation.
- Advisory process locks and atomic replacement for mutable configuration,
  history, knowledge, and package operations.
- Staged package publication, generated `SHA256SUMS`, tamper detection, legacy
  package compatibility, cleanup after validation failure, and two concurrent
  installers racing for the same package name.
- Self-override and override-cycle rejection alongside existing duplicate and
  same-priority conflict behavior.
- Dependency policy, SBOM generation/validation, artifact provenance, and
  detached OpenPGP signing/verification scripts.

## Local validation

- `cargo fmt --check`: passed.
- `cargo check --locked`: passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passed.
- `cargo test --locked`: passed with 146 tests (83 library, 51 CLI,
  6 filesystem, 6 privacy), plus zero doc-test failures.
- `cargo test --locked --test ci_config`: 1 passed after adding the CI syntax and
  release-dependency assertion, for 147 passing tests across the candidate.
- `cargo build --locked --release`: passed; the binary reports `lbc 0.5.0`.
- Release-binary CLI suite: 51 passed with
  `LBC_TEST_BINARY=/home/tsuna/project/libraryCU/target/release/lbc`.
- `cargo audit` 0.22.2: passed after scanning 223 locked dependencies against
  1,245 loaded RustSec advisories.
- `cargo deny check` 0.20.2: advisories, bans, licenses, and sources passed. It
  reported allowed duplicate-version warnings for `core-foundation`, `hashbrown`,
  `syn`, and `windows-sys`; `deny.toml` keeps duplicates visible as warnings.
- `cargo cyclonedx` 0.5.9: generated a valid JSON SBOM with 178 components; all
  178 had names, versions, licenses, and package hashes.
- Provenance generator: passed against temporary archive/SBOM fixtures and
  recorded the source commit and SHA-256 subjects.
- Release signing: passed end to end with a disposable GPG key created in `/tmp`;
  every fixture signature was verified using the exported public key. The test
  key and temporary artifacts were removed.
- `bash -n` for all CircleCI shell scripts and `git diff --check`: passed.

The first release-binary run was inside a socket-restricted sandbox: 36 tests
passed and 15 loopback-dependent tests failed with `Operation not permitted`.
The complete 51-test suite was rerun with loopback permission and passed; the
environmental failure was not treated as a product regression or skipped result.

## Gates that remain open

- Apply and test the hosted GitHub ruleset described in
  [repository-security.md](repository-security.md): required pull request,
  review, required checks, resolved conversations, and blocked deletion/force push.
- Enable GitHub private vulnerability reporting.
- Provision the restricted CircleCI release context with
  `LBC_RELEASE_SIGNING_KEY_BASE64` and `LBC_RELEASE_SIGNING_FINGERPRINT`; publish
  the fingerprint through an independent trusted channel.
- Run the candidate revision in CircleCI on Linux, universal macOS, and Windows.
- Confirm the tag workflow produces all archives, checksums, SBOM, provenance,
  public key, and signatures, then verify them before publishing `v0.5.0`.
- Native Windows Authenticode, macOS Developer ID/notarization, stronger hostile
  directory-race protection, authenticated recovery records, and SLSA Level 2
  provenance remain roadmap work and are not claimed by this candidate.

Do not create or push the `v0.5.0` tag until these release gates are complete.
