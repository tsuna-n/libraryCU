# 0.5.0 release-candidate audit — 2026-09-14

This audit began on the local `release/v0.5.0` worktree based on revision
`04b4e2ae1d8de8eeb8ecbccb5159c87ea775173c` and continues on
`roadmap/complete-v0.4-v0.5`. It is evidence for a release candidate, not
evidence that GitHub controls, exact-revision cross-platform CI, hosted
provenance, or production signing have completed.

## Follow-up hardening — `roadmap/complete-v0.4-v0.5`

The follow-up branch based on `e37439ac7de9a537ed718acad16b902c308e11b1`
adds the remaining locally actionable v0.4/v0.5 hardening:

- Git-compatible matching now combines global excludes, `.git/info/exclude`,
  project/nested ignore files, precedence, negation, and ignored-parent behavior.
- Unix atomic replacement anchors validation and rename to an opened parent
  directory and rejects a parent-symlink swap. Injected failures cover atomic
  file replacement and staged package publication cleanup.
- Version 2 recovery records are HMAC-SHA256 authenticated with a private random
  project key and retained for at most 30 days/100 records. Version 1 records are
  rejected as unauthenticated and remain available for manual inspection.

At that intermediate checkpoint, strict Clippy passed and the full debug suite
passed 154 tests (89 library, 1 CI config, 52 CLI, 6 filesystem, 6 privacy). Final audit,
release-binary, SBOM, provenance, and signing results are recorded in
`docs/roadmap-progress.md`; all local checks passed against the completed diff.

## Final repository-side hardening

The current follow-up worktree adds release and filesystem guarantees found
missing during the requirement-by-requirement audit:

- Release scripts share an exact manifest for the Linux, universal macOS, and
  Windows archives, their checksums, the CycloneDX SBOM, and provenance. Missing,
  empty, symlinked, checksum-mismatched, or malformed inputs fail before signing.
- Provenance subjects cover the final archives, checksum files, and SBOM bytes.
  Signing covers those exact inputs plus provenance, and publication independently
  verifies the exported key fingerprint and every signature in a clean keyring.
- GitHub publication uses a draft and refuses to alter an already-public release.
  The draft is published only after the exact remote asset set is present.
- Linux dependency CI exercises the release scripts with a disposable key and
  local GitHub API fixture. It tests missing credentials, fingerprint mismatch,
  tamper rejection, complete 17-asset publication, an interrupted upload that
  remains a private draft, and refusal to modify an existing public release.
- Package staging syncs validated files/directories and uses anchored no-clobber
  publication on Unix. Tests cover symlink input, target creation during install,
  parent replacement, pre-publication failure, and cleanup.
- Atomic file replacement compares the target digest before publication so a
  same-size concurrent edit is preserved. Recovery regressions now cover every
  authenticated field, malformed/legacy records, missing or invalid keys, and
  unsafe key permissions.

Exact current validation results are maintained in `docs/roadmap-progress.md`.

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
- `cargo test --locked`: passed 166 tests (98 library, 4 CI/release, 52 CLI,
  6 filesystem, and 6 privacy), plus zero doc-test failures.
- `cargo build --locked --release`: passed; the binary reports `lbc 0.5.0`.
- Release-binary CLI suite: all 52 passed with
  `LBC_TEST_BINARY=/home/tsuna/project/libraryCU/target/release/lbc`.
- `cargo audit` 0.22.2: passed after scanning 224 locked dependencies against
  1,245 loaded RustSec advisories.
- `cargo deny check` 0.20.2: advisories, bans, licenses, and sources passed. It
  reported allowed duplicate-version warnings for `core-foundation`, `hashbrown`,
  `syn`, and `windows-sys`; `deny.toml` keeps duplicates visible as warnings.
- `cargo cyclonedx` 0.5.9: generated a valid JSON SBOM with 179 components; all
  179 had names, versions, licenses, and package hashes.
- `.circleci/test-release-scripts.sh`: passed end to end with a disposable GPG
  key and local GitHub API fixture. Every signature was verified with the
  exported public key; a complete 17-asset draft was published, an injected
  upload failure remained private, an existing public release was refused, and
  tampered bytes were rejected. Temporary key material and artifacts were
  removed.
- `bash -n` for all CircleCI shell scripts, CI configuration regressions, and
  `git diff --check`: passed.

The first release-binary run was inside a socket-restricted sandbox: 37 tests
passed and 15 loopback-dependent tests failed with `Operation not permitted`.
The complete 52-test suite was rerun with loopback permission and passed; the
environmental failure was not treated as a product regression or skipped result.

## Gates that remain open

- Public GitHub API evidence on 2026-09-14 reports no repository rulesets,
  private vulnerability reporting disabled, and no `v0.5.0` release. Revision
  `de2839e886040598b018ba59e3b9a116f6621517` passed CircleCI
  [dependency security 21](https://circleci.com/gh/tsuna-n/libraryCU/21) and
  [build and test 22](https://circleci.com/gh/tsuna-n/libraryCU/22). Those runs
  predate the current uncommitted hardening and therefore do not validate it.
- Apply and test the hosted GitHub ruleset described in
  [repository-security.md](repository-security.md): required pull request,
  review, required checks, resolved conversations, and blocked deletion/force push.
- Enable GitHub private vulnerability reporting.
- Provision the restricted CircleCI `lbc-release` context with
  `LBC_RELEASE_SIGNING_KEY_BASE64` and `LBC_RELEASE_SIGNING_FINGERPRINT`; publish
  the fingerprint through an independent trusted channel.
- Run the committed candidate revision in CircleCI on Linux, universal macOS,
  Windows, and dependency security. Earlier Linux results do not cover this
  worktree.
- Confirm the tag workflow produces all archives, checksums, SBOM, provenance,
  public key, and signatures, then verify them before publishing `v0.5.0`.
- Native Windows Authenticode, macOS Developer ID/notarization, multi-user store
  validation, non-Unix directory-race hardening, and SLSA Build Level 2 provenance
  remain roadmap work and are not claimed by this candidate.

The repository-side native-signing design is complete in
[native-code-signing.md](native-code-signing.md); the credentials, hosted jobs,
and production verification evidence described there remain external gates.

Do not create or push the `v0.5.0` tag until these release gates are complete.
