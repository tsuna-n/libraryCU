# 0.5.0 release-candidate audit — 2026-09-16

This audit covers branch `roadmap/complete-v0.4-v0.5`, whose starting checkpoint
was `61381debe1a9d3f35c79b0bdab84db09da71b64d`, plus the release-candidate
hardening recorded here. It is repository-side, local, and hosted candidate
evidence; it is not evidence that GitHub controls, production signing, or the
final release have completed. Any later candidate commit must repeat the four
hosted jobs before it can become the release revision.

## Candidate outcome

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
- Release inputs and all 17 published assets come from a canonical manifest.
  Provenance binds exact source/repository/workflow metadata; clean-keyring
  verification authenticates every input; draft publication compares every
  remote asset's name, size, state, and SHA-256 digest before going public.
- Release regressions explicitly reject missing archives/checksums, malformed
  SBOMs, provenance for a different commit, and artifact changes after
  provenance generation.
- macOS and Windows validation jobs are scheduled for candidate branches and
  pull requests. Release publication remains version-tag-only and secret-scoped.

## Local evidence

Formatting, locked check, strict all-target/all-feature Clippy, the 169-test
debug suite, release build, all 52 CLI tests against the release binary,
`cargo audit`, `cargo deny check`, a 179-component CycloneDX validation, shell
syntax, CI configuration regressions, and the expanded release-script fixture
all passed. Exact commands and counts are maintained in
[roadmap-progress.md](roadmap-progress.md).
The requirement-by-requirement classification is maintained in
[requirement-matrix-v0.4-v0.5.md](requirement-matrix-v0.4-v0.5.md).

## Hosted candidate evidence

- [Dependency-security build 28](https://circleci.com/gh/tsuna-n/libraryCU/28),
  [Linux build/test build 26](https://circleci.com/gh/tsuna-n/libraryCU/26),
  [macOS build 27](https://circleci.com/gh/tsuna-n/libraryCU/27), and
  [Windows build 25](https://circleci.com/gh/tsuna-n/libraryCU/25) all passed on
  exact candidate SHA `76e5df64bf1af2ab43ec5941d1595e4baaa4deb0`.
- The dependency result supersedes failed
  [build 24](https://circleci.com/gh/tsuna-n/libraryCU/24), which found
  `RUSTSEC-2026-0285` in rustls 0.23.43 at the starting checkpoint.

## Evidence that remains open

- Public evidence showed no rulesets, private vulnerability reporting disabled,
  an unsigned candidate commit, and no `v0.5.0` tag or release. Authenticated
  branch-protection state and release-context secrets were unavailable.
- Production archives, checksums, SBOM, provenance, OpenPGP/native signatures,
  signed tag, independent verification, and hosted-builder SLSA Build Level 2
  evidence do not yet exist.

The external setup and negative tests are specified in
[repository-security.md](repository-security.md), [circleci.md](circleci.md),
and [native-code-signing.md](native-code-signing.md). Do not create or push the
`v0.5.0` tag until every open gate in [ROADMAP.md](../ROADMAP.md) is satisfied.
