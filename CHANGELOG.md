# Changelog

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
