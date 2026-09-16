# Security Policy

## Supported versions

Security fixes are provided for the latest released minor line. During the
`0.5.x` series, the latest `0.5.x` patch is supported; `0.4.x` and older receive
no routine fixes after `0.5.0` is released. Pre-release and untagged builds are
supported only for reproducing and validating a report.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Once the repository
owner enables it, use GitHub private vulnerability reporting for
`tsuna-n/libraryCU`:

1. Open the repository's **Security** tab.
2. Choose **Advisories** and **Report a vulnerability**.
3. Include the affected version/commit, platform, impact, reproduction steps,
   and any suggested mitigation. Remove API keys, proprietary source, and other
   secrets from the report.

At the 2026-09-16 release-candidate checkpoint, public API evidence shows that
private vulnerability reporting is not yet enabled. Until it is enabled, contact
the repository owner privately and
ask for a secure reporting channel without including exploit details in the
first message.

## Response targets

- Acknowledge a complete report within 3 business days.
- Provide an initial severity and remediation assessment within 7 business days.
- Aim to release a critical fix within 7 calendar days and a high-severity fix
  within 30 calendar days. Coordinated disclosure timing may change these goals.

These are response targets rather than guarantees. The maintainer will keep the
reporter informed when investigation, platform coordination, or release signing
requires more time.

## Security release process

1. Reproduce privately and add a regression test that contains no live secret or
   harmful payload.
2. Patch supported branches and run the complete release gate, including
   `cargo audit` and `cargo deny check`.
3. Verify the pinned signed release commit and annotated tag after all four
   candidate gates pass. Native-sign the existing tested Windows/macOS binaries,
   verify the exact identities/timestamps, notarize/staple the macOS DMG, re-test
   signed binaries, and package only verified bytes. Generate final checksums,
   CycloneDX SBOM, provenance and detached OpenPGP signatures.
4. Obtain independently verified hosted attestation and administrator evidence
   before the owner approval. Clean-keyring verify the complete 25-file manifest;
   upload a private snapshot only to a draft, verify every remote name/size/digest,
   and only then publish. Repeat native/OpenPGP/provenance verification on
   independent downloads before announcing the advisory and release.
5. Credit the reporter when requested and document affected/fixed versions and
   operational mitigations.

Release maintainers must use signed commits and signed annotated tags. Signing
keys and CI signing material must remain outside the repository. Rotate or revoke
them immediately after suspected exposure.

Native credential and protected-context setup is specified in
[native-code-signing.md](docs/native-code-signing.md) and
[circleci.md](docs/circleci.md). Local mocks/disposable keys, native signing
records and tenant-generated SLSA v1 provenance do not prove production signing
or SLSA Build Level 2. Offline OpenPGP checks detect only revocations present in
the imported key data; maintainers must publish fresh trusted revocation/rotation
information. GitHub controls remain EXTERNAL ADMIN ACTION REQUIRED until
authenticated settings and negative-test evidence are retained.

## Scope

Mutable stores follow the fail-closed single-owner
[ownership and ACL policy](docs/shared-store-security.md). Shared writable stores
are rejected, not made safe for collaboration. Native platform tests and exact
candidate CI evidence are tracked separately from production signing evidence.

Reports about path containment, unsafe file replacement, secret disclosure,
unexpected network access, package integrity, recovery records, verification
command execution, and release provenance are in scope. Dependency findings are
triaged based on whether the vulnerable code is reachable in libraryCube.
