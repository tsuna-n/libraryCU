# v0.6.0 — Developer experience

Release candidate prepared on 2026-11-12. This version adds compatible CLI
features, so the package, signed release tag and archive examples use `0.6.0`.
The existing `v0.5.0` tag and public release remain unchanged.

## Changes

- English/Thai errors explain the cause, remedy and next command. Parser errors
  omit private source values, and credential redaction covers encoded URL keys.
- `doctor` diagnoses configuration, knowledge directories, permissions, symlinks
  and invalid documents without creating stores or modifying user files.
  Provider connections require explicit `--connectivity`; no repair runs.
- `search/list --limit`, global `--quiet`/`--no-color` and `NO_COLOR` support
  make terminal and scripted output predictable. Default limits, JSON fields
  and runtime/usage exit codes remain compatible.
- Guided offline-first setup, verified archive installation, stable personal
  storage paths and shorter onboarding guides are included.
- Dependency security updates and signing-output path validation retain the
  existing release verification policy.

## Validation and publication

Local Linux validation uses isolated stores and loopback provider fixtures:
all checks below passed on the 0.6.0 candidate before commit:

| Check | Result |
|---|---|
| `cargo fmt --all --check` | PASS |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --locked --all-targets --all-features` | PASS: 278 tests, no failures or ignored tests |
| `cargo build --locked --release` and `target/release/lbc --version` | PASS: `lbc 0.6.0` |
| `cargo audit --no-fetch` | PASS using the existing local advisory database |
| `bash .circleci/test-release-scripts.sh` | PASS: real archive/OpenPGP fixtures and 12 mock draft API scenarios |
| Independent code review and `git diff --cached --check` | PASS |

The
release commit and annotated tag must both carry the configured maintainer
signature. CI derives archive names from the Cargo package version and rejects
tag/version mismatches.

The GitHub release starts as a draft. Production publication still requires the
existing CircleCI source identity gate, Windows Authenticode, macOS signing and
notarization, the complete signed asset manifest, and owner approval. No signing
credential or policy check is removed. Do not publish an empty public release
or label local unsigned binaries as verified production assets.

Native Windows/macOS execution and live AI-provider requests are not part of
the local Linux test evidence. The dated [onboarding report](onboarding-validation.md)
and [v0.5.0 release audit](release-readiness-2026-11-08.md) remain historical
receipts, not evidence that the v0.6.0 hosted gates have passed.
