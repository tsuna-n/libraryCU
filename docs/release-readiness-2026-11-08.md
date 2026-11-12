# v0.5.0 release readiness — 2026-11-08

**Production status: NOT READY.** Repository fixes and local checks do not
establish production signing, native trust, hosted provenance, or publication.
Version remains 0.5.0. No push, repository tag change, release mutation, CI retry,
or production credential provisioning was performed during this audit.

Read `AGENT.md`, `docs/product-direction.md`, the v0.5.0 release audit,
completion handoff, CircleCI runbook and native-signing runbook before changes.
The starting worktree was `main` at
`21a8ea21362b0f5502d357cd045cd972428037fb`, with pre-existing untracked `nvim.log`
left untouched. Final changes are local and have no hosted CI result.

## Why there are no release assets

Fresh read-only GitHub and CircleCI queries confirmed the following. A normalized
receipt is retained in [release-readiness-2026-11-08.json](evidence/release-readiness-2026-11-08.json).

- [GitHub Release 390049263](https://github.com/tsuna-n/libraryCU/releases/tag/v0.5.0)
  is public (`draft=false`, `prerelease=false`), has zero uploaded assets, and
  still says DRAFT in its title/body. GitHub's automatic source ZIP/tarball links
  are not uploaded CLI release archives. The publisher correctly refuses to
  replace assets on this public release; that refusal remains unchanged.
- The annotated tag object is
  `d4d47c86915515f473fff8b237a28e2bc365560f`, targeting
  `ac16da5b7f8944a3331f8c5c61812bbb30e3b86a`, not current main or these fixes.
  GitHub reports `unknown_key` for both tagged commit and tag. Starting main is
  `unsigned`. Signature text and historical candidate passes do not establish
  pinned production source trust.
- [Tagged source gate 153](https://circleci.com/gh/tsuna-n/libraryCU/153) failed
  explicitly on `LBC_MAINTAINER_SIGNING_KEY_BASE64: EXTERNAL CREDENTIAL REQUIRED:
  configure the maintainer PUBLIC verification key`. This prevents its downstream
  native signing/approval/publication chain. No later successful production
  source gate was observed. Adding a key alone would not resolve the stale tag
  or existing public-release conflict.

Latest hosted candidates all used exactly
`21a8ea21362b0f5502d357cd045cd972428037fb` and had no release tag:

| Job | Observed result | Scope |
|---|---|---|
| [Linux 184](https://circleci.com/gh/tsuna-n/libraryCU/184) | PASS | Original starting source |
| [macOS 183](https://circleci.com/gh/tsuna-n/libraryCU/183) | PASS | Original starting source |
| [Windows 182](https://circleci.com/gh/tsuna-n/libraryCU/182) | FAIL | 108 library and 2 CI tests passed; CLI 62 passed, 1 failed |
| [Dependency 185](https://circleci.com/gh/tsuna-n/libraryCU/185) | PASS | Historical advisory database; fresh audit found newer advisories |

Windows failed at `explicit_project_scope_is_preserved_and_other_projects_do_not_see_it`:
the test compared a canonical returned path against a noncanonical expected path.
Rust [documents Windows extended path syntax for canonicalize](https://doc.rust-lang.org/std/fs/fn.canonicalize.html).
The assertion now compares the exact parent with the canonical expected store,
retaining the project-isolation checks and all production path security.
Windows job 182 stopped before its release build/archive installer checks.

## Repository fixes and security verification

- The artifact signer rejects canonical output paths that are symlinks or
  nonregular files before importing its private key or writing anything.
  A regression reproduced overwriting an external sentinel through the public-key
  output symlink before the fix. Public-key and detached-signature symlink cases
  now fail without changing the sentinel.
- The Unix installer now requires `lbc --version` to succeed and identify `lbc`.
  Previously a failing executable was reported as successfully installed.
  Both failing-command and wrong-command cases reproduced that false success.
  Real tar extraction/install/exact-version/uninstall tests cover prefixes with
  spaces, repeated uninstall, and preservation of unrelated files.
- CI regressions retain extraction and installation from Linux/macOS tarballs
  and Windows ZIPs, including Windows user PATH addition/removal. The existing
  workflow already runs the required platform checks; no job was removed or
  made optional. All release credentials remain tag-only and context-scoped.
- Fresh baseline audit found five rustls/rustls-webpki vulnerability entries;
  deny also rejected Tokio's I/O-safety advisory and the archived YAML backend.
  Updated only rustls 0.23.45 → 0.23.46, rustls-webpki 0.103.15 → 0.103.16 and
  Tokio 1.53.1 → 1.53.3 in the lockfile. Replaced archived serde_yaml/unsafe-libyaml
  with the [YAML organization's maintained yaml_serde](https://github.com/yaml/yaml-serde)
  0.10.7 and [libyaml-rs](https://github.com/yaml/libyaml-rs) 0.3.0, retaining the
  `serde_yaml` Cargo alias, existing runtime callers and metadata format.
  Two regressions pass both the old parser and its chosen successor: legacy
  null/string scalars, aliases, literal merge keys, maximum u64 timestamps,
  Thai/multiline metadata, capture and edit serialization. The replacement
  restores maintenance, not a memory-safety rewrite: libyaml-rs retains translated
  unsafe Rust. Its MSRV fits the existing CI toolchain. No advisory ignores,
  licenses/source exceptions or unrelated Windows resolutions were added.
- The exact 25-asset manifest, archive SHA-256 checks, 12 detached signatures,
  independently pinned source/artifact/native identities, SBOM validation,
  exact 11-subject provenance binding, native signing, owner approval, remote
  digest checks and existing-public-release refusal remain enforced.
  Real disposable OpenPGP fixtures and simulated native receipts are test data,
  never production signature or certificate evidence. Generated provenance alone
  does not satisfy the hosted-builder attestation gate.

## Final local validation

Tested source identity is `21a8ea21362b0f5502d357cd045cd972428037fb+worktree`.
The receipt records the implementation-diff SHA-256 separately from documentation,
and hashes of the actual release binary, local Linux archive and SBOM. This is
local evidence, not hosted evidence for a new commit or a production asset set.
Logs and unsigned local artifacts are in ignored
`target/release-readiness-2026-11-08/`.

| Command/check | Actual final result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo check --locked` | PASS |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --locked`, private `TMPDIR` | PASS: 215 tests; 123 library, 9 CI, 65 CLI, 12 filesystem, 6 privacy; zero failures/ignored tests |
| `cargo build --locked --release` | PASS; actual binary reports `lbc 0.5.0` |
| `LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli`, private `TMPDIR` | PASS: all 65 tests against the built release executable |
| `cargo audit` | PASS: fresh 1,422-advisory database, 224 locked dependencies, no vulnerability findings |
| `cargo deny check` | PASS advisories/bans/licenses/sources; existing duplicate warnings remain |
| `cargo cyclonedx --format json --all-features --override-filename lbc-0.5.0.cdx` | PASS: CycloneDX 1.3, 179 components, all with name/version/license/hash metadata |
| `bash .circleci/test-release-scripts.sh` | PASS after integration: 25 real archive cases, disposable source/OpenPGP fixtures, checksum/SBOM/provenance/native-record negatives and 12 mock draft API scenarios |
| `bash .circleci/test-macos-signing.sh` | PASS: 22 MOCK cases, not macOS production trust |
| `for release_script in .circleci/*.sh install.sh; do bash -n "$release_script"; done` | PASS for every shell script |
| `python3 benchmarks/retrieval/run.py --binary target/release/lbc --source-sha 21a8ea21362b0f5502d357cd045cd972428037fb+worktree --baseline docs/evidence/retrieval-baseline.json --output target/release-readiness-2026-11-08/retrieval.json` | PASS: no per-case baseline regressions; labels unchanged; synthetic project memory 6/6 |
| Built Linux five-member tarball: SHA-256, extract, install, exact binary hash/version, uninstall twice, unrelated knowledge file preservation | PASS; unsigned local candidate, not downloaded production bytes |
| `git diff --check` | PASS |

Retrieval retains the baseline's limits: Top-1/Top-3 31/31, status and citation
identity 40/40, strict citation relevance 33/40, result precision 35/75 (46.67%)
and no-match correctness 8/8. These are small-corpus regression measurements,
not real incident-resolution accuracy.

Intermediate failures are retained honestly: the pre-fix installer/signing
regressions failed as expected; initial audit/deny rejected the old dependencies.
The first integrated sandbox test run had 29 library failures because sandbox
`/tmp` was owned by UID 65534, which existing ancestor-owner checks reject.
Benchmark execution similarly rejected sandbox-remapped `/home`. Both succeeded
outside that sandbox using a private workspace `TMPDIR`, matching CI isolation;
no ownership/ACL check was disabled. Advisory refresh initially could not lock
the read-only external database; the authorized rerun refreshed it successfully.
Sandbox failure logs are kept beside the successful logs.

Two independent reviews found no confirmed problems in the signing/installer
guards, canonical path assertion or compatible YAML successor. No production
keys, native certificates, notarization or hosted attestation were tested.

## Files changed

| Files | Purpose |
|---|---|
| `.circleci/sign-release-assets.sh`, `.circleci/test-release-scripts.sh` | Safe signing outputs and regression |
| `install.sh`, `tests/ci_config.rs` | Honest executable validation and archive/CI regressions |
| `tests/cli/learn.rs` | Windows canonical path assertion |
| `Cargo.toml`, `Cargo.lock`, `src/knowledge/storage.rs` | Security patches, maintained YAML backend, compatibility regressions |
| `README.md`, `CHANGELOG.md` | Current release status and change notes |
| `docs/circleci.md`, `docs/release-0.5.0.md`, `docs/release-completion-0.5.0.md` | Point historical receipts to this audit |
| `docs/release-readiness-2026-11-08.md`, `docs/evidence/release-readiness-2026-11-08.json` | Current findings, command ledger, source/digest receipt and owner handoff |

The CircleCI workflow, `deny.toml`, Windows installer, native signers and runtime
knowledge implementation were retained. `src/knowledge/storage.rs` changes add
tests only. No user-store migration is required.

## Owner actions and external blockers

1. Review the complete local diff and dependency changes. Only after separate
   authorization, commit the final reviewed source and run all four hosted
   candidate jobs on that exact SHA. CI uses Rust 1.97.1; local checks use
   Rust 1.99.1. All new platform and dependency changes need fresh native passes.
2. Make an explicit recovery decision for the existing public empty release and
   stale signed tag. Publishing these fixes as v0.5.0 cannot succeed against a tag
   still naming the older source. Any tag/release change requires separate owner
   authorization and the existing repository protections; no automatic retagging
   or public-release overwrite is introduced.
3. Verify GitHub protections and CircleCI context/project/maintainer/ref/revision
   restrictions according to [repository-security.md](repository-security.md)
   and [circleci.md](circleci.md). No fresh administrator-permission or context
   settings proof was obtained in this audit. Never grant branch/PR jobs the
   release credentials to make a gate pass.
4. In protected `lbc-release-identity`, supply
   `LBC_MAINTAINER_SIGNING_KEY_BASE64` (base64 PUBLIC armored export, one primary
   identity plus valid signing subkeys) and
   `LBC_MAINTAINER_SIGNING_FINGERPRINT` (independently trusted full uppercase
   PRIMARY fingerprint). The source gate needs no maintainer private key.
   Review/sign the actual final commit and annotated tag with the trusted owner
   key; pass the gate against exactly that source.
5. Provision external Authenticode and Developer ID/notarization identities in
   their separate protected contexts, and the artifact OpenPGP key/fingerprint
   plus repository-scoped Contents read/write `GITHUB_TOKEN` in `lbc-release`.
   Use [native-code-signing.md](native-code-signing.md) for exact native variables,
   timestamp service, Apple pins and independent verification commands.
6. Run real native signing/notarization and inspect staged signed outputs.
   Independently establish the hosted-builder attestation requirement, then
   approve publication only after every gate is satisfied. After authorized
   publication, independently download all 25 assets, run
   `verify-release-assets.sh` with separately trusted pins and expected source/CI
   metadata, and repeat Windows/macOS platform trust checks.

Production credentials, GitHub write permissions, administrator controls and
fresh Windows/macOS runners were not supplied or exercised. Their absence is
reported as external work, not bypassed. An empty public release provides no
production archive on which to claim installation, signature or digest success.
