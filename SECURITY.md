# Security Policy

## Supported versions

Security fixes are provided for the latest released minor line. During the
`0.5.x` series, the latest `0.5.x` patch is supported; `0.4.x` and older receive
no routine fixes after `0.5.0` is released. Pre-release and untagged builds are
supported only for reproducing and validating a report.

## Reporting a vulnerability

Do not open a public issue for a suspected vulnerability. Use GitHub private
vulnerability reporting for `tsuna-n/libraryCU`:

1. Open the repository's **Security** tab.
2. Choose **Advisories** and **Report a vulnerability**.
3. Include the affected version/commit, platform, impact, reproduction steps,
   and any suggested mitigation. Remove API keys, proprietary source, and other
   secrets from the report.

If private reporting is unavailable, contact the repository owner privately and
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
3. Build release artifacts from a signed tag in CI. Generate checksums, CycloneDX
   SBOM, build provenance, and detached signatures.
4. Verify every signature and checksum before publishing the advisory and release.
5. Credit the reporter when requested and document affected/fixed versions and
   operational mitigations.

Release maintainers must use signed commits and signed annotated tags. Signing
keys and CI signing material must remain outside the repository. Rotate or revoke
them immediately after suspected exposure.

## Scope

Reports about path containment, unsafe file replacement, secret disclosure,
unexpected network access, package integrity, recovery records, verification
command execution, and release provenance are in scope. Dependency findings are
triaged based on whether the vulnerable code is reachable in libraryCube.
