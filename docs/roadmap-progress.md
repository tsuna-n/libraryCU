# Roadmap progress

## Release/security completion — 2026-09-17 (current)

- Branch `main`, clean starting SHA `ac16da5b7f8944a3331f8c5c61812bbb30e3b86a`.
  The continuation changes release scripts/tests/config and matching docs only;
  core functionality, version and the existing signed tag are preserved.
- Exact starting SHA passed candidate macOS 149, Windows 150, dependency 151 and
  Linux 152. Source gate 153 failed for missing maintainer PUBLIC verification
  key. GitHub reports commit verification `unknown_key`, not verified trust.
- Existing annotated/signed `v0.5.0` tag points to that SHA. GitHub Release
  `390049263` is PUBLIC (`draft=false`, `prerelease=false`), empty, and labeled
  DRAFT in title/body. This is an owner-action inconsistency, not production
  completion. No tag/release changes were made; publisher refusal is preserved.
- Repository changes: canonical GitHub transport/origin allowlist; explicit
  source/context diagnostics; independent negative source fixtures and signed
  primary/subkey positives; fixed verifier selection/replacement-object handling;
  RSA/ECC primary/signing-subkey strength enforcement; Linux archive validation
  and all-member ZIP CRC reads; staged native artifact access before approval.
  ZIP directory/volume bits and ambiguous tar member types are rejected; Windows
  adds two native-host-only policy cases. A resumed draft cannot retain DRAFT
  title/body or prerelease status when the final gated publication succeeds.
- The fresh command ledger, full matrix and exact owner actions are in
  [release-completion-0.5.0.md](release-completion-0.5.0.md). Current edits require
  review/signing and all four hosted jobs on their eventual final source SHA;
  old CI passes never validate a changed continuation. The owner subsequently
  authorized committing/pushing this work, without moving the tag or publishing.
- External admin, context/credential, hosted-attestation and production
  execution/download gates remain open. Next: owner reconciles the accidental
  public Release and the existing tag/new-revision conflict, then imports the
  independently verified PUBLIC maintainer export/pin into restricted
  `lbc-release-identity`. Do not approve, publish, retag or start v0.6.

## Historical checkpoints (all sections below are superseded)

## Native release completion continuation — 2026-09-16

- Branch `roadmap/complete-v0.4-v0.5` started at local and remote revision
  `d2093e8b811a93e92d3e3f4fce6cff4d539239d4`. The interrupted working tree was
  recovered intact: native archive/Windows-signing changes plus the associated
  release documentation were preserved and reviewed rather than reset.
- Repository-side Windows Authenticode and macOS Developer ID/notarization jobs,
  signed-source verification, immutable final-input validation, 25-asset draft
  publication and owner approval are implemented. Production credentials,
  native trust, hosted control-plane attestation, GitHub controls and production
  execution remain external; mocks are not production signing evidence.
- ZIP paths are normalized independent of the host before exact-name and
  duplicate checks. The validators reject slash/backslash traversal, absolute
  paths, normalized duplicates, missing/unexpected members, symlink/hardlink,
  Windows reparse metadata and other special-file types without extracting the
  archive. Final binary hashes come from the one exact expected member.
- Local continuation gate: fmt/check/strict Clippy passed; 180 debug tests and
  52 release-binary CLI tests passed; release binary reports `lbc 0.5.0`;
  audit scanned 224 dependencies against 1,246 advisories with no findings;
  deny passed all policies; CycloneDX 1.3 contained 179 licensed/hashed
  components; 19 real archive-policy cases, 22 mocked macOS cases, the complete
  release fixture with 12 draft API scenarios, six CI/provenance tests, shell
  syntax, Python compilation and diff checks passed.
- PowerShell is unavailable on this Linux host. On implementation revision
  `8e721592197b141c374a054d5e10ac7a5c486c16`, Windows build 61 passed 26
  explicitly mocked cases and three self-signed disposable PFX lifecycle cases;
  dependency 62, macOS 63 and Linux 64 also passed. These fixtures are not real
  Authenticode or Apple production evidence.
- Repository-side remaining work is empty after this evidence-recording update.
  Do not create `v0.5.0` until the external administrator, credential, hosted-
  attestation and production verification gates are satisfied.

## Final-source checkpoint — 2026-09-16

- Branch: `roadmap/complete-v0.4-v0.5`; started clean at `f89c648`.
- Source revision: `e5e265ae2c97886f768d41bd0762d5f4ed514b41`; version `0.5.0`,
  release candidate only. No production tag or release exists.
- Implemented [single-owner store validation](shared-store-security.md) in
  `src/security/permissions.rs`: Linux UID/mode/access/default ACL checks,
  macOS opened-descriptor ACL checks, and Windows opened-handle owner/DACL and
  junction/reparse checks. Unsafe shared stores/ancestors/inherited grants fail
  closed. Windows administrative ownership requires enabled effective-token
  membership, not disabled/deny-only UAC membership; TrustedInstaller is trusted
  only on system ancestors. A native restricted-token regression proves this.
- Enforcement also covers recovery-record creation/reads/pruning, private keys,
  history reads/deletion, editor temporary bytes, knowledge creation, atomic
  publication, and all package descendants before deletion. macOS replacement
  refuses even deny-only existing ACLs to avoid stripping privacy restrictions.
- Local final source: fmt, locked check, strict all-target/all-feature Clippy,
  **178 tests** (104 library, 4 CI/release, 52 CLI, 12 filesystem, 6 privacy),
  release build, **52 release-binary CLI tests**, and diff/shell syntax checks
  passed, with zero failures/ignored tests/doc-test failures.
- Audit passed (1,246 advisories / 224 locked dependencies); deny policies passed
  with allowed duplicates visible. CycloneDX structural validation passed for
  179 components, all with name/version/license/hash. Disposable one-day signing
  key and mock-GitHub release fixture passed, not production signing/publication.
- Ownership/loopback-sensitive tests ran with real host metadata and permission;
  sandbox-mapped/denied runs were not successes and checks were not weakened.
- Current-source hosted dependency [51](https://circleci.com/gh/tsuna-n/libraryCU/51),
  Linux [49](https://circleci.com/gh/tsuna-n/libraryCU/49), Windows
  [50](https://circleci.com/gh/tsuna-n/libraryCU/50), and macOS
  [52](https://circleci.com/gh/tsuna-n/libraryCU/52) all passed this exact revision.
  Native macOS passed 174 debug tests and 52 release CLI tests; Windows passed
  157 debug tests and 51 release CLI tests, including effective-token ownership,
  foreign grants, NULL DACL, and junction regressions. Platform-conditional test
  counts differ; no failure was converted to an ignored test.
  The cross-platform ownership/ACL item and four hosted candidate gates are
  now checked in ROADMAP.md and COMPLETE in the requirement matrix.
- External GitHub admin gates: public API still shows zero rulesets and private
  reporting disabled. Authenticated branch protection/context permissions are
  unverified; apply/export/negative-test controls and all four CI contexts.
- Credential/service gates: protected OpenPGP material/fingerprint and scoped
  release context/token, signed commit/tag identity, Windows Authenticode and
  macOS Developer ID/notarization credentials. Credentialed native signing jobs
  remain an operational integration plan, not completed production signing.
- Hosted provenance gate: tenant-generated SLSA-shaped metadata is implemented,
  but control-plane attestation and independent SLSA Build Level 2 evidence are
  still unconfigured/unverified.
- Production execution gate: reviewed signed commit/tag, final native signing/
  notarization before packaging, production manifest/SBOM/provenance/signatures,
  independent downloaded-byte verification, then publication. None is claimed.
- This checkpoint records tested source, not the hash of its own documentation
  commit. The final documentation commit must also obtain four hosted passes;
  inspect CircleCI against branch HEAD before using it as a release candidate.
- Next release-owner actions: configure/export/negative-test GitHub controls and
  protected signing/service identities; implement the credentialed native jobs
  described in `native-code-signing.md`; obtain independently verified hosted
  attestation if claiming Build Level 2. Then gate the reviewed signed revision
  and execute/independently verify the production release. Do not tag/publish or
  start v0.6 while those gates remain open.

## Historical checkpoints (superseded by the final-source checkpoint above)

## Checkpoint — 2026-09-16

### Shared-store continuation (supersedes the earlier candidate counts below)

- `867f82e` passed dependency 33, Linux 35, and macOS 36. Windows 34 compiled
  and reached native tests, but rejected the system drive's TrustedInstaller
  owner. The follow-up recognizes only the fixed Windows Modules Installer SID
  on system ancestors (never foreign store owners), and distinguishes
  inherit-only ancestor ACEs from effective delete/control grants.
- `de022f5` passed dependency 37, Linux 40, and macOS 39. Windows 38 reached
  native tests but rejected newly created administrative-owned temp objects.
  The next follow-up accepts Administrators ownership only if that SID is
  enabled in the effective token; disabled/deny-only UAC membership cannot pass.
  DACL checks still reject non-administrative foreign principals.
- Final review also validates recovery-record bytes/reads, unsafe recovery
  directories, editor temporary bytes, history deletion, recovery pruning, and
  every package descendant before deletion. Targeted Linux library 104 and
  filesystem 12 tests pass. Final full suite/native CI is being repeated.

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
