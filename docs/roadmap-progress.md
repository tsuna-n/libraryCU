# Roadmap progress

## Checkpoint — 2026-09-14

- Branch: `roadmap/complete-v0.4-v0.5`
- Committed HEAD: `de2839e886040598b018ba59e3b9a116f6621517`
- Candidate state: the current worktree includes uncommitted release-integrity,
  filesystem, test, and documentation hardening on top of that HEAD
- Active milestone: `v0.5.0 — Security & Software Supply Chain`
- Version: `Cargo.toml`, `Cargo.lock`, and the release binary report `0.5.0`
- Release state: release candidate only; there is no `v0.5.0` tag release or
  production artifact set

## Requirement audit classification

- **COMPLETE (repository implementation and local tests):** explicit project
  scope, project escape rejection, Git-compatible ignore evaluation, bounded
  safe reads, Unix anchored file and package publication, atomic mutable-file
  writes, advisory store locks, transaction-safe checksummed package install,
  corruption/duplicate/symlink/target-race rejection, complete override-cycle
  validation, authenticated bounded recovery, redaction, dependency policy,
  SBOM generation, exact release manifest, provenance generation, detached
  signing/verification, and draft-only publication logic.
- **PARTIALLY COMPLETE:** multi-user/shared-store validation (single-user
  protections exist; cross-platform ownership/ACL policy does not), native
  binary trust (detached OpenPGP pipeline exists; Authenticode and Developer ID/
  notarization do not), and provenance (accurate tenant-generated metadata
  exists; hosted-builder SLSA Build Level 2 does not).
- **TEST REQUIRED:** hosted Linux, macOS, Windows, and dependency-security jobs
  must run on the exact committed version of this worktree.
- **EXTERNAL CONFIGURATION REQUIRED:** GitHub rulesets/branch and tag controls,
  private vulnerability reporting, signed-maintainer enforcement/evidence, the
  CircleCI `lbc-release` context, and hosted-builder provenance configuration.
- **RELEASE-TIME ONLY:** create and independently verify production archives,
  checksums, SBOM, provenance, signatures, signed tag, and GitHub Release.
- **OUT OF v0.5 SCOPE:** enterprise policy, organization audit logging,
  RBAC/SSO, and organization knowledge server are explicitly assigned to
  v0.6–v0.8 and were not started.

## Repository-side work completed

- Release generation now requires the exact Linux, universal macOS, and Windows
  archives, their matching checksum files, and a structurally valid CycloneDX
  SBOM. Missing, empty, symlinked, mismatched, malformed, or unexpected files
  fail closed.
- Provenance subjects bind every final archive, checksum, and SBOM. The signing
  stage signs those exact inputs plus provenance. A separate clean-keyring stage
  verifies the configured full fingerprint and every signature before upload.
- GitHub publication creates or resumes a draft, refuses to modify an already
  public release, verifies the exact uploaded asset set, then publishes the
  completed draft. The workflow is bound to the `lbc-release` context.
- Dependency CI now tests provenance, signing, verification, and publication
  with a disposable one-day key and a local GitHub API fixture. Coverage includes
  missing credentials, mismatched fingerprints, tampered bytes, complete
  17-asset publication, interrupted upload, and refusal to modify a public
  release.
- Package staging syncs the validated snapshot and directories before atomic
  publication. Linux uses `renameat2(RENAME_NOREPLACE)` anchored to an opened
  parent; other Unix platforms use anchored `renameat`, and portable platforms
  retain checked publication with documented target-creation race limitations.
- Atomic file replacement rechecks the target digest before rename. Tests prove
  same-size concurrent edits, existing package targets, and swapped parent
  directories are preserved rather than overwritten or redirected. Store lock
  paths reject FIFOs and other non-regular files without blocking.
- Recovery tests now cover every authenticated field, malformed and legacy
  records, missing/invalid keys, unsafe key permissions, pruning, symlinks, and
  later-edit conflicts.
- `ROADMAP.md` now distinguishes implemented pipelines, local validation,
  hosted exact-revision evidence, external configuration, and production
  release artifacts.

## Local validation of the current worktree

- `cargo fmt --check`: passed.
- `cargo check --locked`: passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passed.
- `cargo test --locked`: passed 166 tests total (98 library, 4 CI/release,
  52 CLI, 6 filesystem, 6 privacy), including the real 45-second timeout.
- `cargo build --locked --release`: passed; `target/release/lbc --version`
  reports `lbc 0.5.0`.
- `LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli`:
  passed all 52 tests with loopback permission. The first sandboxed attempt
  passed 37 and denied sockets for 15; it was not treated as a product result.
- `cargo audit` 0.22.2: passed against 1,245 RustSec advisories and 224 locked
  crate dependencies.
- `cargo deny check` 0.20.2: advisories, bans, licenses, and sources passed;
  documented duplicate warnings remain for `core-foundation`, `hashbrown`,
  `syn`, and `windows-sys`.
- `cargo cyclonedx` 0.5.9: generated 179 components; all 179 have names,
  versions, licenses, and package hashes, and the validation query passed.
- `bash .circleci/test-release-scripts.sh`: passed with a disposable key and
  local GitHub API fixture after granting the local sandbox permission to start
  `gpg-agent` and bind loopback. It proved complete publication, private-draft
  retention after an injected upload failure, public-release refusal, and
  tamper rejection.
- `bash -n .circleci/*.sh`, CI YAML regression tests, and `git diff --check`:
  passed.

## Hosted and external evidence

- Public GitHub/CircleCI evidence shows committed HEAD
  `de2839e886040598b018ba59e3b9a116f6621517` passed dependency-security build
  21 and Linux build/test build 22. These runs predate the current uncommitted
  worktree and therefore are historical evidence only.
- Public GitHub API responses on 2026-09-14 show zero repository rulesets,
  private vulnerability reporting disabled, no check runs beyond commit status
  contexts, committed HEAD marked `unsigned`, and no `v0.5.0` tag or GitHub
  Release.
- No hosted macOS or Windows result exists for the current candidate worktree.
- No production signing identity, signed tag, final artifacts, or
  control-plane-generated SLSA Build Level 2 attestation was available.

## Remaining gates and exact next actions

1. Commit and push this worktree, then run `dependency_security`,
   `build_and_test`, `build_macos`, and `build_windows` on that exact revision.
2. With repository-admin access, apply both branch and tag rulesets from
   `docs/repository-security.md`, enable private vulnerability reporting, and
   record API exports plus negative tests.
3. Configure the restricted CircleCI `lbc-release` context with the production
   release subkey, its independently published full fingerprint, and scoped
   GitHub token. Configure a hosted builder/control plane if SLSA Build Level 2
   remains a release requirement.
4. After review and every hosted gate passes, create and verify a signed
   `v0.5.0` tag. Let the draft workflow build all assets, then independently
   compare checksums, signatures, provenance subjects, SBOM, tag, and published
   bytes before considering the release complete.

Do not start v0.6 work or publish `v0.5.0` before these gates are satisfied.
