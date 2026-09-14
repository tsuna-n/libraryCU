# Changelog

## 0.5.0 — 2026-09-14

- Added pinned `cargo audit` and `cargo deny` policy checks to a dedicated
  dependency-security CI job.
- Added release CycloneDX SBOM generation, artifact digest provenance, detached
  OpenPGP signatures, and signature verification before publication.
- Added the security policy, vulnerability response targets, repository ruleset
  checklist, and signed-release operating procedure.
- Project commands now use the exact supplied directory instead of discovering
  and ascending to a parent root. Git ignore matching supports Git-compatible
  recursive patterns, character classes, escapes, negation, nested files, global
  excludes, repository excludes, and ignored-parent behavior.
- Knowledge package installs now stage and publish atomically under an advisory
  process lock. New installs include SHA-256 integrity manifests; corrupted
  packages are excluded while legacy packages remain readable as unverified.
- Added advisory locking and atomic replacement for mutable configuration,
  history, knowledge, and package stores, plus concurrent package-install tests.
- Hardened Unix atomic replacement against parent-directory symlink swaps and
  added failure-injection coverage for file and package publication.
- Recovery records now use HMAC-SHA256 authentication with a private per-project
  key and prune to a maximum of 100 records and 30 days.
- Override validation now rejects self-overrides and cycles while preserving the
  existing deterministic same-priority conflict checks.

## 0.4.0 — 2026-09-07

- Added `fix --ai --apply --verify "COMMAND"`, with explicit executable/arguments,
  a configurable deadline, bounded/redacted output, exit status, and honest
  verification reporting. No implicit shell or provider-selected commands.
- Every CLI apply saves a private local recovery record. Verification failure
  attempts rollback; `lbc rollback ID --project PATH` restores a prior application
  only when its file still matches, preserving later edits.
- Added TypeScript, Node.js, and Go diagnostic parsing, selected offline rules,
  Windows paths, CRLF/ANSI handling, and malformed/mixed-log regressions.
- Added GitHub Actions for Linux, macOS, and Windows: formatting, locked check,
  strict Clippy, full tests, release build, release-binary CLI tests, and artifacts.
- Added portable verification process fixtures and rollback/privacy/filesystem
  regressions. See the [0.4.0 audit](docs/release-0.4.0.md) for actual validation.

## 0.3.4 — 2026-09-06

### Added

- `lbc fix`: offline guidance, an AI-generated single-file replacement with
  `--ai`, and explicit application with `--ai --apply`. JSON output and
  English/Thai presentation are supported. No repair or verification commands
  are executed, and applied patches remain `unverified`.
- Patch validation for exact unique before text, diagnostic-line intersection,
  source/excerpt/response byte limits, recognizable secrets, path containment,
  symlinks, special files, and detected source changes. Publication is atomic
  and retains file permissions.
- Parsing for ordinary Python tracebacks and syntax-error frames, including
  chained exceptions, ANSI input, and mixed Rust/Python diagnostics.
- An optional local Python/FastAPI knowledge package with 40 bilingual notes.
  Install from the checkout with
  `lbc knowledge install ./packages/python-fastapi-basics`.
- Regression coverage for version output, fix preview/application, provider
  failure, no implicit network access, privacy, file safety, and Python workflows.

### Changed

- Unknown Python failures may retrieve a generic traceback playbook as
  `general_guidance`. The answer explicitly states that evidence for a specific
  fix is insufficient, and this fallback cannot authorize a patch.
- Exact-code/title matches retain priority. Python fallback signals avoid
  accidental substring matches, and equal retrieval scores/titles use source
  identity to break ties deterministically.
- Cargo package and lockfile version updated together; CLI version reporting
  continues to use Cargo metadata.
- README, usage examples, and AGENT.md now describe the explicit fix workflow,
  JSON statuses, limits, and verification semantics.

See the [release audit](docs/release-0.3.4.md) for exact validation results,
installation status, and remaining limitations. Structural patch validation
does not establish that the proposed code is correct.
