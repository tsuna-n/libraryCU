# CircleCI candidate gates and protected release workflow

Every branch and PR runs dependency_security, build_and_test (Linux),
build_macos and build_windows. These four jobs receive no release context.
They build/test installable candidates, verify packaged versions and
install/uninstall, and persist archives/checksums/SBOM to dist.

Linux runs fmt, locked check, strict all-target/all-feature Clippy, full tests,
release build and release-binary CLI tests. Rust is pinned to 1.97.1.
Dependency security pins cargo-audit 0.22.2, cargo-deny 0.20.2 and
cargo-cyclonedx 0.5.9, enforces deny.toml and validates dependency
name/version/license/hash metadata. It runs real disposable OpenPGP and signed
source fixtures, 12 local draft API scenarios, 19 real ZIP/tar policy cases and
22 mocked Mac signing cases. Windows additionally runs 26 mocked signing cases
and three real disposable certificate/PFX lifecycle cases. None establishes
production trust.
Config/data/cache/temp directories are isolated; Linux caches include Rust,
architecture and lockfile checksum.

## Version-tag workflow

```text
four candidate gates (tests/lint/security/platform packaging/SBOM)
 -> release_source_gate (signed commit + signed annotated tag + pinned identity)
 -> sign_macos_release + sign_windows_release (existing tested binary, no rebuild)
 -> approve_production_release (owner hold)
 -> publisher source recheck + final production-dist
 -> checksum/record validation -> provenance -> OpenPGP -> clean verification
 -> verified private snapshot -> draft upload -> remote verification -> public
```

Credential-bearing jobs and approval are version-tag-only; their branches
filter is ignore: /.*/. Tags must match SemVer and equal v plus Cargo.toml's
version. Source verification requires all four candidates, native signers
require that gate, and publication explicitly requires all candidates, source,
both native jobs and owner approval. Native jobs persist only distinct
production-dist files, not credentials. The publisher copies only Linux
archive/checksum and SBOM from dist, not unsigned Mac/Windows candidates.
See [native-code-signing.md](native-code-signing.md) for executable commands,
credential lifecycle and the additional stapled DMG.

The owner must not approve without administrator controls, production identities,
native evidence and independently verified hosted-attestation evidence satisfying
the release gates. Approval does not verify external settings, generate an
attestation or establish SLSA Level 2. Candidate passes alone do not authorize
a production tag.

## Protected contexts: owner setup required

EXTERNAL ADMIN ACTION REQUIRED: create/restrict these exact contexts to release
maintainers, approved protected release refs and trusted ephemeral workers.
Check organization security groups/expression restrictions and project
permissions; demonstrate branch/PR jobs cannot access them. Tag filters alone
do not secure contexts against malicious modified config. Retain redacted
permissions exports and negative-test evidence. Context settings/credentials
were not provisioned or authenticated here.

| Context | Jobs | Owner-provisioned variables |
|---|---|---|
| lbc-release-identity | source gate and publisher | LBC_MAINTAINER_SIGNING_KEY_BASE64 (PUBLIC armored export), LBC_MAINTAINER_SIGNING_FINGERPRINT (full uppercase PRIMARY fingerprint) |
| lbc-release-windows | Windows signer only | Certificate/store/PFX/timestamp variables in native runbook |
| lbc-release-macos | Mac signer only | Developer ID P12/pins/Team ID and App Store Connect P8/key/issuer variables in native runbook |
| lbc-release | publisher only | GITHUB_TOKEN, LBC_RELEASE_SIGNING_KEY_BASE64, LBC_RELEASE_SIGNING_FINGERPRINT, optional LBC_RELEASE_SIGNING_PASSPHRASE, and PUBLIC LBC_WINDOWS_CERT_THUMBPRINT, LBC_MACOS_IDENTITY_SHA1, LBC_MACOS_TEAM_ID |

EXTERNAL CREDENTIAL REQUIRED: supply externally issued native identities and
a dedicated production OpenPGP primary identity/signing subkey outside the
repository. Fingerprints always identify the full PRIMARY key, including when
a subkey signs. Mask private exports/passwords; the optional OpenPGP passphrase
is read via private file descriptor, not argv. Scope the fine-grained GitHub
token to tsuna-n/libraryCU, Contents read/write. Publish PUBLIC pins/Team ID via
a separate trusted channel. The source gate rejects secret material, multiple
primary identities, wrong pins, unsigned commits, lightweight/unsigned tags and
tags targeting a different SHA. Production source-signing identity/evidence
has not been supplied.

## Canonical production manifest: exactly 25 assets

.circleci/release-common.sh:set_release_assets is authoritative.
VERSION is 0.5.0 for this candidate:

| Unsigned input class | Exact names/count |
|---|---|
| Linux | lbc-VERSION-x86_64-unknown-linux-gnu.tar.gz and .sha256 (2) |
| macOS | lbc-VERSION-universal-apple-darwin.tar.gz and .sha256; lbc-VERSION-universal-apple-darwin.dmg and .sha256 (4) |
| Windows | lbc-VERSION-x86_64-pc-windows-msvc.zip and .sha256 (2) |
| SBOM | lbc-VERSION.cdx.json (1) |
| Native verification records | lbc-VERSION.windows-signing.json, lbc-VERSION.macos-signing.json (2) |
| Repository provenance | lbc-VERSION.provenance.json (1) |

Each of these 12 files has an armored sibling NAME.asc (12 more).
lbc-release-signing-key.asc is the public-key asset (1). Extra files/directories,
duplicate names, links and empty files fail. Native signatures are embedded in
binaries/DMG and verified on their platforms.

Provenance uses in-toto Statement v1 / SLSA provenance v1 structure. It binds
actual origin and CI identity to tsuna-n/libraryCU, full SHA, version, tag/ref,
fixed build type/builder and exact workflow/job/build metadata. Its 11 subjects
are all final build outputs above except provenance itself. Provenance,
signatures and public key are downstream authentication metadata, not circular
subjects; provenance itself is OpenPGP-signed. Sets/digests must be exact and
unique. Strict JSON rejects duplicate members/nonfinite values; malformed SBOM,
native records or provenance fail.

OpenPGP uses fresh private homes with no user options or automatic key retrieval.
One usable primary identity must match the independently trusted full pin.
GPG status must authenticate primary-or-signing-subkey identity and SHA-256/384/512;
bad/missing/expired/known-revoked signatures fail. Public exports may not contain
secret material. Offline verification cannot discover revocation omitted from
an old export: owners must distribute fresh trusted revocation/key data and
rotation notices. Tests use only disposable identities, including protected
keys, expired/revoked keys and a signing subkey.

## Reproducible independent verification

Download all 25 files into an otherwise empty directory. Use a reviewed checkout
at the expected release SHA, with the expected origin. Obtain PUBLIC expected
values independently from the trusted release record, not downloaded JSON:

```bash
export LBC_RELEASE_SIGNING_FINGERPRINT=TRUSTED_FULL_PRIMARY_FINGERPRINT
export LBC_WINDOWS_CERT_THUMBPRINT=TRUSTED_WINDOWS_LEAF_THUMBPRINT
export LBC_MACOS_IDENTITY_SHA1=TRUSTED_DEVELOPER_ID_LEAF_FINGERPRINT
export LBC_MACOS_TEAM_ID=TRUSTED_TEAM_ID
export CIRCLE_SHA1=EXPECTED_FULL_RELEASE_COMMIT_SHA CIRCLE_TAG=v0.5.0
export CIRCLE_PROJECT_USERNAME=tsuna-n CIRCLE_PROJECT_REPONAME=libraryCU
export CIRCLE_WORKFLOW_ID=EXPECTED_RELEASE_WORKFLOW_ID CIRCLE_WORKFLOW_NAME=ci_cd
export CIRCLE_JOB=publish_github_release CIRCLE_BUILD_URL=EXPECTED_PUBLISH_JOB_URL
bash .circleci/verify-release-assets.sh /absolute/path/to/downloads
```

The command creates/removes its clean keyring and validates the entire manifest,
SBOM, records, provenance and all 12 signatures. Repeat downloaded native trust
checks using the native runbook; Linux receipt validation is not a Windows or
Apple trust verifier. Retain all outputs and independently verify the hosted
attestation against its issuer/builder/ref policy separately.

## Draft recovery and remote safety

The publisher verifies files, copies all 25 into a private snapshot, re-verifies
it and freezes names/sizes/digests. Only snapshot bytes upload. Existing PUBLIC
releases are never replaced. Interrupted uploads may resume only in a matching
draft, replacing expected names; initial unexpected/duplicate assets abort
before mutations. Draft/tag/ID is re-read before each deletion/upload and final
publish. Remote names, sizes, uploaded states and API SHA-256 must exactly match
the frozen set; snapshot verification repeats before publication. Failure never
calls final publication. Observed intervening public state stops further writes.
GitHub offers no atomic draft-conditioned upload/delete: exclusive controlled
draft ownership is an external prerequisite, not protection against an
adversarial concurrent release administrator.

Production tokens can go only to https://api.github.com and the exact pinned
GitHub upload endpoint. HTTP loopback override is allowed only with literal
fixture-token, never production credentials. Twelve scenarios cover success,
resume, interruption, public refusal, API/malformed responses, unexpected,
duplicate and tampered assets, original-directory mutation after snapshotting,
intervening public state, and invalid asset IDs before any mutation.

## Hosted attestation and production execution

EXTERNAL INFRASTRUCTURE REQUIRED: repository-generated OpenPGP provenance is
not an unforgeable hosted/control-plane attestation and does not establish SLSA
Build Level 2. The owner must choose/configure a provider whose authenticated
issuer/builder identity the tenant job cannot forge; bind exact source/tag/
workflow and all final subjects; define independent issuer/builder/ref policy;
retain signed attestation and verifier output. No remote control plane or
provider identity was invented. Additional public attestation assets require a
reviewed/tested versioned manifest extension; do not inject unexpected files
into the current contract. Hold approval without this required evidence.

Current status: NOT READY. Only after ALL external pre-tag gates and exact
reviewed signed-revision candidate passes are evidenced may the owner verify the
commit and create/verify/push the matching signed annotated tag. Tag candidate,
source and native jobs must pass. Independently verify staged native outputs
and hosted attestation before owner approval; the publisher then signs/verifies
final files and remote digests before public transition. Re-download and repeat
verification, retain the release record/advisory. No tag, production run or public
release is claimed. crates.io and v0.6 are out of scope.
