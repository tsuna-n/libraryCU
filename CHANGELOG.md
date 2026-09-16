# Changelog

## 0.5.0 — Unreleased (release-candidate checkpoint 2026-09-16)

- Implemented tag-only, separately credential-scoped Windows Authenticode and
  macOS Developer ID/hardened-runtime/notarization/stapling jobs. They sign and
  re-test existing candidate binaries, verify repackaged bytes and emit
  digest-bound native records; a signed/stapled DMG accompanies the macOS tar.
  Production native identities/execution remain external and unverified.
- Added a pinned signed-commit/annotated-tag source gate, post-native owner
  approval, and deterministic 25-asset production manifest. Provenance now binds
  exact repository origin, source, version/tag/ref, workflow and all 11 final
  build subjects; native records are not hosted attestations.
- Hardened OpenPGP against ambiguous JSON, private public-key material,
  expired/known-revoked identities, weak digests and wrong signing-subkey pins;
  protected keys read masked passphrases through a private descriptor.
- Publication freezes/re-verifies a private upload snapshot and rechecks draft
  state before every mutation. Added intervening-public-state and local-mutation
  fixtures, real disposable source/OpenPGP cases, a 19-case real ZIP/tar policy
  suite, 22 Mac and 26 Windows signing mocks, and three hosted Windows disposable
  PFX lifecycle cases.

- Added pinned `cargo audit` and `cargo deny` policy checks to a dedicated
  dependency-security CI job.
- Fixed `ask --ai` and `chat --ai` to send the documented default model for
  named providers and report the configured provider identity in JSON.
- Updated locked `rustls` from 0.23.43 to 0.23.45 to resolve
  `RUSTSEC-2026-0285` in the release-candidate dependency gate.
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
  added failure-injection coverage for file and package publication. Mutable
  store lock paths now reject FIFOs, symlinks, other non-regular files, a
  different owner, and group/other-accessible modes without blocking.
- Package staging is synced before anchored directory publication; Linux uses a
  no-clobber rename, and symlinked package content and observed existing targets
  are rejected.
- Release publication now requires the exact three-platform archive/checksum
  set and a valid SBOM, records their final digests, verifies every detached
  signature independently, and keeps the GitHub Release in draft state until
  every verified remote asset name, size, state, and SHA-256 digest matches.
- Added disposable-key and mock-GitHub-API CI regressions for missing
  credentials, fingerprint mismatch, missing assets, bad checksums, tampered
  release bytes, interrupted and resumed uploads, API and response failures,
  unexpected/duplicate assets, remote digest mismatch, complete publication,
  and refusal to modify a public release.
- Candidate branches and pull requests now run the macOS and Windows package
  jobs as well as Linux, leaving publication restricted to matching version tags.
- Expanded package-integrity regressions for malformed, duplicate, missing,
  unexpected, symlinked, and tampered content, plus post-validation source
  mutation; recovery retention now proves expired records are actually removed.
- Recovery records now use HMAC-SHA256 authentication with a private per-project
  key and prune to a maximum of 100 records and 30 days.
- Override validation now rejects self-overrides and cycles while preserving the
  existing deterministic same-priority conflict checks.

- Added fail-closed single-owner mutable-store validation on Linux, macOS, and
  Windows: descriptor/handle ownership and ACL checks, unsafe ancestors and
  inherited grants, private key/history reads, and Windows junction rejection.
  Added platform-specific ACL and shared-store regressions; this does not enable
  shared writable collaboration or establish production signing evidence.
- Windows administrative ownership now requires an enabled effective-token SID;
  disabled/deny-only membership cannot authorize a private store. macOS atomic
  replacement refuses existing deny-only ACLs rather than stripping restrictions.
  Native Linux/macOS/Windows and dependency-security validation passed the exact
  source candidate recorded in `docs/release-0.5.0.md`; production gates stay open.

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
