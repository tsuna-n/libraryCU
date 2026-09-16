# 0.5.0 release-candidate audit — 2026-09-16

This audit covers branch `roadmap/complete-v0.4-v0.5` through implementation
revision `8e721592197b141c374a054d5e10ac7a5c486c16`. It is repository-side,
local, and hosted candidate evidence; it is not evidence that GitHub controls,
production signing, hosted control-plane attestation, or the final release have
completed. Any later candidate commit must repeat the four hosted jobs before it
can become the release revision.

## Candidate outcome

- The shared-store continuation implements single-owner Linux/macOS/Windows
  owner/ACL validation, unsafe ancestor rejection, private key/history reads,
  and Windows junction rejection. All 180 Linux tests and 52 release-binary
  tests pass; all four native/platform and dependency-security candidate jobs
  passed the exact final-source revision below. See
  [shared-store-security.md](shared-store-security.md).

- The hosted dependency failure is fixed locally by resolving `rustls` 0.23.45
  instead of vulnerable 0.23.43.
- `ask --ai` and `chat --ai` now send the documented default model when a named
  provider omits `ai.model`, and JSON identifies that configured provider rather
  than its shared OpenAI-compatible transport.
- Explicit project boundaries, Git-compatible ignores, anchored Unix atomic
  replacement, authenticated bounded recovery, and staged checksummed package
  publication retain their regression coverage.
- Unix store locks now reject symlinks, non-regular files, another owner, and
  group/other-accessible modes.
- Package tests reject malformed, duplicate, missing, unexpected, symlinked,
  and content-tampered integrity state, and prove post-validation source changes
  cannot alter the published snapshot.
- Release inputs and all 25 published assets come from a canonical manifest.
  Provenance binds exact source/repository/version/tag/workflow metadata and 11
  final subjects; clean-keyring verification authenticates every input; draft
  publication compares every remote asset's name, size, state, and SHA-256
  digest before going public.
- Release regressions explicitly reject missing archives/checksums, malformed
  SBOMs, provenance for a different commit, and artifact changes after
  provenance generation.
- Tag-only, credential-scoped Windows Authenticode and macOS Developer ID,
  hardened-runtime, notarization and stapling jobs sign the tested candidate
  bytes and emit digest-bound records. The repository implementation is
  complete; production identity execution remains external and unverified.

## Local evidence

Formatting, locked check, strict all-target/all-feature Clippy, the 180-test
debug suite, release build, all 52 CLI tests against the release binary,
`cargo audit`, `cargo deny check`, a 179-component CycloneDX validation, shell
syntax, CI configuration regressions, and the expanded release-script fixture
all passed. Exact commands and counts are maintained in
[roadmap-progress.md](roadmap-progress.md).
The requirement-by-requirement classification is maintained in
[requirement-matrix-v0.4-v0.5.md](requirement-matrix-v0.4-v0.5.md).

## Hosted candidate evidence

Implementation revision `8e721592197b141c374a054d5e10ac7a5c486c16`
passed all four:

- [Linux 64](https://circleci.com/gh/tsuna-n/libraryCU/64): full Rust quality,
  release-binary CLI, packaging and installer gate.
- [macOS 63](https://circleci.com/gh/tsuna-n/libraryCU/63): native platform,
  release-binary CLI, packaging and ACL gate.
- [Windows 61](https://circleci.com/gh/tsuna-n/libraryCU/61): native platform,
  packaging and ACL gate plus 26 mocked signing cases and three disposable
  self-signed PFX lifecycle cases. These are not production trust evidence.
- [Dependency security 62](https://circleci.com/gh/tsuna-n/libraryCU/62): audit,
  deny, 179-component SBOM, 19 real archive-policy cases, disposable OpenPGP/
  source signatures, 22 macOS mocks and 12 draft-publication scenarios.

The following earlier results are historical, not substitutes for these passes:

- [Dependency-security build 28](https://circleci.com/gh/tsuna-n/libraryCU/28),
  [Linux build/test build 26](https://circleci.com/gh/tsuna-n/libraryCU/26),
  [macOS build 27](https://circleci.com/gh/tsuna-n/libraryCU/27), and
  [Windows build 25](https://circleci.com/gh/tsuna-n/libraryCU/25) all passed on
  exact candidate SHA `76e5df64bf1af2ab43ec5941d1595e4baaa4deb0`.
- The dependency result supersedes failed
  [build 24](https://circleci.com/gh/tsuna-n/libraryCU/24), which found
  `RUSTSEC-2026-0285` in rustls 0.23.43 at the starting checkpoint.

## Manual release gates that remain open

- Public API evidence rechecked on 2026-09-16 showed zero rulesets, private
  vulnerability reporting disabled, `8e72159` unsigned, and no `v0.5.0` tag or release. Authenticated
  branch-protection state and release-context secrets were unavailable.
- GitHub administrator: configure/export/negative-test branch/tag rules, required
  PR/review/resolved conversations and all four CI contexts, force-push/deletion
  blocks, signed maintainer controls, and private vulnerability reporting.
- Credentials/services: provision or independently verify protected source and
  OpenPGP identities/fingerprints, scoped release token, Authenticode identity/
  timestamp service and Developer ID/notarization identities. Implemented jobs
  are not proof they ran with production credentials. Hosted control-plane
  attestation and independent policy verification are required for Build Level 2.
- Production execution: gate a reviewed signed commit/tag, native-sign/notarize
  before packaging, generate final checksums/SBOM/provenance/OpenPGP signatures,
  independently verify downloaded bytes and identities, then publish. No
  production asset set, signatures, notarization, or publication is claimed.

The external setup and negative tests are specified in
[repository-security.md](repository-security.md), [circleci.md](circleci.md),
and [native-code-signing.md](native-code-signing.md). Do not create or push the
`v0.5.0` tag until every open gate in [ROADMAP.md](../ROADMAP.md) is satisfied.
