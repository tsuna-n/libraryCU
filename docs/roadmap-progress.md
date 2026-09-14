# Roadmap progress

## Checkpoint — 2026-09-14

- Branch: `release/v0.5.0`
- Base revision: `04b4e2ae1d8de8eeb8ecbccb5159c87ea775173c`
- Candidate revision: the branch HEAD containing this checkpoint
- Active milestone: `v0.5.0 — Security & Software Supply Chain`
- Version: `Cargo.toml` and `Cargo.lock` report `0.5.0`
- State: local release candidate; no release tag has been created or published

The candidate implements dependency audit/deny policy, CycloneDX SBOM,
provenance and OpenPGP artifact-signing automation, security/repository guidance,
strict explicit project scope, Git-compatible project ignore matching, staged
checksummed package installation, advisory store locks, atomic config/history
writes, and override-graph validation. `ROADMAP.md` contains the canonical
checkbox state.

Validation evidence and limitations are recorded in the
[0.5.0 release-candidate audit](release-0.5.0.md). Local code, security, SBOM,
provenance, and disposable-key signing checks pass. Hosted GitHub controls,
private vulnerability reporting, production signing credentials, cross-platform
CircleCI results, and actual release artifacts remain unverified.

## Next actionable work

1. Push this branch and open a pull request for review.
2. Apply `docs/repository-security.md` on GitHub and require both build/test and
   dependency-security checks.
3. Configure the restricted CircleCI release signing context without committing
   key material.
4. Run all branch checks. Resolve platform or supply-chain failures on this
   branch and update the audit with links to the exact CI revision.
5. After approval and merge, create a signed `v0.5.0` tag. Verify every generated
   asset and signature before publication, then close the remaining release-gate
   checkboxes in `ROADMAP.md`.

If external configuration is unavailable, continue independent `v0.5.0` work on
the unchecked hostile symlink-race and failure-injection items. Do not start
`v0.6.0` or claim `v0.5.0` released while any release gate remains open.
