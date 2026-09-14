# Roadmap progress

## Checkpoint — 2026-09-14

- Branch: `roadmap/complete-v0.4-v0.5`
- Base revision: `e37439ac7de9a537ed718acad16b902c308e11b1`
- Candidate revision: the branch HEAD containing this checkpoint
- Active milestone: `v0.5.0 — Security & Software Supply Chain`
- Version: `Cargo.toml` and `Cargo.lock` report `0.5.0`
- State: local release candidate; no release tag has been created or published

## Completed in this checkpoint

- Git ignore evaluation now includes the configured global excludes,
  `.git/info/exclude`, nested `.gitignore` files, precedence/negation, and Git's
  ignored-parent rule. Unit and CLI tests cover the added behavior.
- Unix atomic replacement opens and validates directory components without
  following symlinks, anchors the final rename to an opened directory descriptor,
  and detects a parent-directory swap. Portable platforms retain the existing
  checked tempfile fallback.
- File replacement and package publication have injected pre-publication failure
  tests that prove the original/target stays intact and temporary staging cleans up.
- New recovery records are version 2 and authenticated with HMAC-SHA256 using a
  random private project key. Restore verifies authentication before trusting
  record paths or content. Records are serialized under a process lock and pruned
  to 100 entries and 30 days. Legacy version 1 records require manual recovery.
- `ROADMAP.md` marks the corresponding v0.4 known gaps and v0.5 hardening items
  complete. Usage, safety, changelog, and agent handoff documentation were updated.

## Validation

- `cargo fmt --check`: passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passed.
- `cargo test --locked`: passed with 154 tests (89 library, 1 CI config, 52 CLI,
  6 filesystem, 6 privacy), including the real 45-second timeout regression.
- `cargo audit`: passed against 1,245 loaded RustSec advisories and 224 locked
  dependencies.
- `cargo deny check`: passed advisories, bans, licenses, and sources. Existing
  duplicate-version warnings remain visible for `core-foundation`, `hashbrown`,
  `syn`, and `windows-sys`.
- `cargo build --locked --release`: passed; `target/release/lbc` reports 0.5.0.
- Release-binary CLI suite: all 52 tests passed, including the 45-second timeout.
- `cargo cyclonedx` 0.5.9 generated 179 components; all 179 had names,
  versions, licenses, and package hashes.
- The provenance generator recorded SHA-256 subjects for a Linux archive and
  SBOM fixture. The signing script produced detached signatures for the archive,
  SBOM, and provenance; independent `gpgv` checks passed with a disposable
  one-day key. Temporary key material and artifacts were removed.
- `bash -n` for CircleCI/release shell scripts and `git diff --check`: passed.
- CircleCI build 19 passed `build_and_test` for code revision `54820a8`, including
  the full suite, release-binary suite, version/diff checks, Linux packaging,
  packaged install/uninstall, and artifact upload. Build 20 passed dependency
  audit/deny, SBOM validation, and artifact upload for the same revision.
- Cross-compilation was not run because `rustup` is not installed in this
  environment. Hosted platform gates must be run after the diff is committed.

## Open v0.4/v0.5 requirements

- The v0.4 Known Gaps entries for enterprise policy, organization audit logging,
  RBAC/SSO, and organization knowledge server are owned by v0.6-v0.8 and remain
  unchecked. Multi-user/shared-store validation and native signed binaries also
  remain open.
- Public GitHub API evidence on 2026-09-14 reports no repository rulesets and
  private vulnerability reporting disabled. GitHub administration is blocked by
  missing credentials in this session: protect `main`, enforce PR/review/CI/
  conversation/force-push/deletion rules, apply and test the ruleset, enable
  private reporting, and verify signed maintainer commits/tags.
- SLSA Build L2 remains open. Current tenant-generated provenance is valid SLSA
  v1-shaped metadata, but Level 2 requires provenance generated and signed by the
  hosted build platform's control plane.
- CircleCI credentials are unavailable. Linux, macOS, and Windows hosted jobs,
  production signing context, generated checksums/signatures, and the exact
  release artifacts are not verified for this revision.

## Next actionable work

1. Open a pull request from this branch for the required human review.
2. With repository-admin access, apply and adversarially test
   `docs/repository-security.md`, enable private vulnerability reporting, and
   require signed maintainer changes.
3. Configure a hosted builder capable of generating and signing SLSA Build L2
   provenance, then validate its identity and artifact subjects.
4. Run all CircleCI platform jobs on the exact reviewed revision. Only after every
   gate passes, merge and create a signed `v0.5.0` tag; verify every release asset
   before publication.
