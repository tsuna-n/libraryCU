# Roadmap progress

## Checkpoint — 2026-09-16

### Shared-store continuation (supersedes the earlier candidate counts below)

- Starting revision: `f89c648c7d5d2a225028f19775a878383a27a803`; clean branch.
- Implemented fail-closed single-owner store policy in
  `src/security/permissions.rs` and native regressions in `tests/filesystem.rs`.
  Linux descriptor ACL access/default checks, macOS extended allow/deny checks,
  Windows owner/DACL and reparse/junction checks now protect mutable operations,
  sensitive temporary bytes, configuration/history reads, and recovery keys.
- Targeted Linux filesystem suite: 11 passed. CLI suite: 52 passed with real
  host metadata/loopback permission. Strict Clippy passed before the latest
  Windows-only fixture; the final full gate is being rerun.
- Sandboxed CLI execution reports `/tmp` and `/home` owned by `nobody`, so it
  cannot validate the real ownership policy. Denied/mapped runs are not successes.
- Public CircleCI state rechecked: all four jobs passed the starting `f89c648`
  revision (macOS 29, Linux 30, dependency 31, Windows 32). That result does not
  validate the shared-store changes. The roadmap hosted gates are reset pending
  native runs of the changed candidate; the cross-platform item remains open.
- GitHub admin configuration, production signing credentials, hosted-builder
  provenance, and release execution remain separate external gates. No release
  tag/publication or credential setup is authorized by this continuation.

- Branch: `roadmap/complete-v0.4-v0.5`
- Starting checkpoint: `61381debe1a9d3f35c79b0bdab84db09da71b64d`
- Candidate state: the branch changes contain the dependency-security fix,
  release-integrity hardening, cross-platform CI scheduling change, regression
  tests, and documentation updates on top of that commit.
- Active milestone: `v0.5.0 — Security & Software Supply Chain`
- Version: `Cargo.toml`, `Cargo.lock`, and the release binary report `0.5.0`.
- Release state: release candidate only; there is no `v0.5.0` tag, GitHub
  Release, or production artifact set.

## Requirement audit classification

- **Complete in the repository and locally tested:** strict explicit project
  scope, project-escape rejection, Git-compatible ignore evaluation, bounded
  safe reads, Unix anchored atomic file and package publication, advisory store
  locks, checksummed transaction-safe package installation, corruption and
  race rejection, override-cycle validation, authenticated bounded recovery,
  redaction, dependency policy, SBOM generation, a canonical release manifest,
  exact provenance subjects, detached signing and clean-keyring verification,
  and draft-only publication with exact remote asset verification.
- **Partially complete:** the v0.4 shared-store item has single-user and Unix
  owner/mode protections but not cross-platform multi-user ownership/ACL policy;
  native binary trust has detached OpenPGP plus documented Authenticode and
  Developer ID/notarization plans but no production native signatures; and the
  repository generates accurate SLSA v1-shaped provenance but not hosted-builder
  SLSA Build Level 2 provenance.
- **Hosted tested:** `dependency_security`, Linux, macOS, and Windows passed on
  exact candidate SHA `76e5df64bf1af2ab43ec5941d1595e4baaa4deb0`.
- **External configuration required:** GitHub branch/tag rulesets, private
  vulnerability reporting, signed-maintainer evidence, the restricted CircleCI
  `lbc-release` context, and any hosted provenance control plane.
- **Release-time only:** production archives, checksums, SBOM, provenance,
  signatures, signed tag, independent verification, and GitHub Release.
- **Intentionally deferred:** organization knowledge is v0.6 scope; enterprise
  policy and audit logging are v0.7 scope; RBAC/SSO is v0.8 scope. None was
  started as part of this candidate.

The remaining unchecked v0.4/v0.5 roadmap items remain unchecked for a concrete
reason: incomplete cross-platform implementation, external configuration,
release-time evidence, or an explicitly later milestone. No
checkbox is being used to claim evidence that does not exist.

## Repository-side work completed

- `Cargo.lock` now resolves `rustls` 0.23.45, fixing the
  `RUSTSEC-2026-0285` failure reported by hosted dependency-security build 24.
- The release scripts derive all 17 publication assets from one canonical
  manifest: three archives, three checksum files, the SBOM, provenance, eight
  detached signatures, and the exported public key.
- Provenance binds the exact repository and source commit plus CircleCI workflow,
  job, and build metadata. Validation reconstructs every subject digest and
  rejects repository, builder, source, or metadata mismatches.
- Signature verification imports the release key into a clean keyring, requires
  exactly one primary key and the configured full fingerprint, verifies every
  detached signature, and rejects files outside the canonical manifest.
- GitHub publication creates or resumes a private draft, replaces named assets,
  refuses an already-public release, and compares the complete remote name,
  byte-size, upload-state, and SHA-256 digest set before publishing.
- Release-script regressions now cover missing or malformed inputs, short and
  wrong fingerprints, missing/bad signatures and keys, symlinks, empty files,
  checksum-name mismatch, interrupted upload, resumed drafts, API errors,
  malformed responses, unexpected/duplicate assets, remote digest tampering,
  and public-release refusal.
- Unix mutable-store lock files must be regular, owned by the current effective
  user, and grant no group/other permissions. Existing FIFO, symlink, unsafe-mode,
  same-size concurrent-edit, and parent-swap regressions pass.
- Package integrity regressions cover malformed and duplicate checksum entries,
  missing entries, unexpected files, symlinks, and content tampering. A source
  mutation after validation cannot change the staged snapshot that is published.
- Recovery retention has an integration test proving expired records are pruned
  while fresh records remain.
- macOS and Windows build jobs now run on candidate branches and pull requests,
  not only `main` and version tags. Publication remains restricted to version
  tags and the `lbc-release` context.
- Named `openai`, `zai`/`glm`, and `ollama` answer/chat requests now use their
  effective default model when `ai.model` is omitted, and answer JSON preserves
  the configured provider identity. The mock-provider CLI regression exercises
  the OpenAI default end to end.
- The full requirement classification and owning evidence are recorded in
  [requirement-matrix-v0.4-v0.5.md](requirement-matrix-v0.4-v0.5.md).

## Local validation of the candidate tree

- `cargo fmt --check`: passed.
- `cargo check --locked`: passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passed.
- `cargo test --locked`: passed 169 tests (101 library, 4 CI/release, 52 CLI,
  6 filesystem, and 6 privacy), plus zero doc-test failures. A first sandboxed
  attempt reached the CLI suite but denied loopback sockets to 15 tests; the
  complete suite was rerun with loopback permission and passed. The denied run
  is not counted as a product result.
- `cargo build --locked --release`: passed; `target/release/lbc --version`
  reports `lbc 0.5.0`.
- `LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli`:
  passed all 52 tests against the release binary.
- `cargo audit` 0.22.2: passed after loading 1,246 RustSec advisories and
  scanning 224 locked dependencies.
- `cargo deny check` 0.20.2: advisories, bans, licenses, and sources passed.
  Allowed duplicate-version warnings remain visible for `core-foundation`,
  `hashbrown`, `syn`, and `windows-sys`.
- `cargo cyclonedx` 0.5.9: generated 179 components; all 179 had names,
  versions, licenses, and package hashes, and the validation query passed.
- `bash .circleci/test-release-scripts.sh`: passed end to end with a disposable
  one-day OpenPGP key and local GitHub API fixture, including explicit missing
  archive/checksum, malformed SBOM, wrong-commit provenance, and
  post-provenance artifact mutation cases. One sandboxed rerun could not start
  `gpg-agent`; the fixture then passed with the required process permission.
- `bash -n .circleci/*.sh`, Python fixture parsing, CI configuration
  regressions, and `git diff --check`: passed.

## Hosted and external evidence

- Exact candidate SHA `76e5df64bf1af2ab43ec5941d1595e4baaa4deb0`
  passed [dependency security build 28](https://circleci.com/gh/tsuna-n/libraryCU/28),
  [Linux build/test build 26](https://circleci.com/gh/tsuna-n/libraryCU/26),
  [macOS build 27](https://circleci.com/gh/tsuna-n/libraryCU/27), and
  [Windows build 25](https://circleci.com/gh/tsuna-n/libraryCU/25).
- The dependency result supersedes failed
  [build 24](https://circleci.com/gh/tsuna-n/libraryCU/24) on the starting
  checkpoint, whose lockfile contained vulnerable rustls 0.23.43.
- Public GitHub API responses on 2026-09-16 showed zero repository rulesets,
  private vulnerability reporting disabled, the starting checkpoint marked `unsigned`,
  and no `v0.5.0` tag or release. The branch-protection endpoint required
  authentication, so branch protection could not be independently verified or
  configured from this environment.
- No production signing identity, final artifacts, signed tag, or
  control-plane-generated SLSA Build Level 2 attestation was available.

## Remaining gates and exact next actions

1. Re-run `dependency_security`, `build_and_test`, `build_macos`, and
   `build_windows` after any later candidate commit; all four must pass on the
   final release revision.
2. With repository-admin access, apply and test both rulesets from
   [repository-security.md](repository-security.md), enable private vulnerability
   reporting, and record authenticated API exports plus negative-test evidence.
3. Provision the restricted CircleCI `lbc-release` context with the production
   release subkey, its independently published full fingerprint, and a scoped
   GitHub token. Use a hosted control plane if SLSA Build Level 2 remains a
   release requirement.
4. After review and every hosted gate passes, create and verify a signed
   `v0.5.0` tag. Independently compare all production checksums, signatures,
   provenance subjects, SBOM, tag, and published bytes before completing the
   remaining release-time checkboxes.

Do not start v0.6 work or publish `v0.5.0` before these gates are satisfied.
