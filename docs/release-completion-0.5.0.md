# v0.5.0 repository completion and owner handoff — 2026-09-17

## Release readiness: NOT READY

This is repository implementation/local test evidence, not a production release
certificate. No external settings, production identities or release/tag objects
were changed. Version stays 0.5.0; v0.6 was not started. This report records
pre-commit validation; the owner subsequently authorized committing and pushing
the continuation. That authorization does not move the release tag or authorize
production publication. Fresh hosted CI must validate the resulting commit.

Starting branch/HEAD: `main`, `ac16da5b7f8944a3331f8c5c61812bbb30e3b86a`.
Remote annotated `v0.5.0` object: `d4d47c86915515f473fff8b237a28e2bc365560f`,
targeting that SHA. Both commit/tag contain OpenPGP signatures; their trusted
production identity is not established merely by their presence.

Read-only GitHub/CircleCI checks established:

- Candidate macOS 149, Windows 150, dependency 151 and Linux 152 passed the
  exact starting SHA. These do not validate the continuation's changed files.
- [Source gate 153](https://circleci.com/gh/tsuna-n/libraryCU/153) failed with
  `LBC_MAINTAINER_SIGNING_KEY_BASE64: EXTERNAL CREDENTIAL REQUIRED: configure
  the maintainer PUBLIC verification key`. It must not be bypassed.
- GitHub commit verification: `verified=false`, `reason=unknown_key`.
- Public rulesets response: `[]`; private reporting: `enabled=false`.
  Main protection endpoint: `404 Branch not protected`. Authenticated/inherited
  protection and context permissions remain unverified; no controls are COMPLETE.
- [GitHub v0.5.0 Release](https://github.com/tsuna-n/libraryCU/releases/tag/v0.5.0),
  ID `390049263`: `draft=false`, `prerelease=false`, zero uploaded assets,
  published `2026-09-16T15:29:44Z`, but title/body say DRAFT. This is a PUBLIC
  release-state inconsistency, not a private draft or production evidence.

## 1. Exact repository changes

| File | Reason |
|---|---|
| `.circleci/verify-release-source.sh` | Actionable missing context-variable/key/signature diagnostics; exact SHA/annotated target/internal tag name/direct commit checks; disable replacement objects and override both Git OpenPGP verifier settings |
| `.circleci/release-common.sh` | Allowlist canonical HTTPS/Git SSH GitHub origin (reject bare paths); reject weak/unsupported primary and actual signing-subkey algorithms in addition to full pins and signature verdicts |
| `.circleci/test-release-scripts.sh` | Independent source negatives, primary/subkey positives, verifier/replacement/internal-tag tests, RSA-1024 primary/subkey rejection and canonical resumed-draft publication metadata regression |
| `.circleci/native_release.py` | Validate Linux as well as native archives; read every ZIP/tar member; reject corrupt ZIP CRC/data, DOS directory/volume file metadata, regular tar files with trailing slashes and nonempty tar directories |
| `.circleci/test-native-archives.py` | 25 real archive-policy cases, including production Linux validation, non-binary ZIP corruption and ambiguous type metadata |
| `.circleci/sign-windows-release.ps1` | Match Linux ZIP DOS directory/volume/reparse member policy before extraction/repackaging |
| `.circleci/test-windows-signing.ps1` | Two real ZIP metadata cases added to the previous 26 cases; 28 intended cases require hosted Windows execution |
| `.circleci/config.yml` | Store verified native `production-dist` outputs/receipts as staged artifacts so owners can inspect/download them before approval |
| `tests/ci_config.rs` | Assert native evidence is stored after the signer; retain tag-only/context and all-source-prerequisite checks |
| `.circleci/publish-github-release.sh` | Final gated publish request/response requires canonical version/source title/body and non-prerelease state instead of retaining stale draft candidate metadata; public-release refusal is unchanged |
| `.circleci/mock-github-release.py` | Start resumed draft with stale DRAFT/prerelease metadata; record/validate the final canonical payload without touching GitHub |
| `README.md` | Candidate/unverified status and public-empty-release inconsistency |
| `CHANGELOG.md` | Unreleased production/candidate state and tested hardening changes |
| `ROADMAP.md` | Latest starting-SHA hosted evidence and existing-tag/public-release owner conflict; production gates remain unchecked |
| `docs/roadmap-progress.md` | Current continuation/evidence/next-action section; prior checkpoints explicitly historical |
| `docs/requirement-matrix-v0.4-v0.5.md` | Exact allowed status classes; current source/release state and fresh-final-revision hosted requirements |
| `docs/release-0.5.0.md` | Authoritative current source/release/CI evidence ahead of the historical audit |
| `docs/circleci.md` | Exact PUBLIC source variable formats/preflight, key-strength policy, staged evidence, draft metadata and existing-release recovery prerequisites |
| `docs/native-code-signing.md` | 25-case archive policy, staged output access and honest unrun changed Windows evidence |
| `docs/repository-security.md` | Current rules/reporting/API/unknown-key/release observations; owner verification stays mandatory |
| `docs/release-completion-0.5.0.md` | This command ledger, requirement audit and exact owner handoff |

No Cargo/core implementation, maintainer key material, production signing
identity, tag object or GitHub Release was modified.

## 2. Validation command ledger

Commands ran from `/home/tsuna/project/libraryCU`. Repeated successful commands
are consolidated; failed/intermediate attempts are explicitly included.

| Command | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo check --locked` | PASS |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Initial FAIL: new test comparison allocated an owned YAML value; fixed comparison, fresh reruns PASS |
| `cargo test --locked` | PASS: 180 tests (104 library, 6 CI, 52 CLI, 12 filesystem, 6 privacy), zero failures/ignored tests; zero doc-test failures |
| `cargo build --locked --release` | PASS |
| `LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli` | PASS: all 52 tests against actual release binary |
| `./target/release/lbc --version` | PASS: `lbc 0.5.0` |
| `cargo audit` | PASS: 1,246 advisories, 224 locked dependencies, no vulnerability findings |
| `cargo deny check` | PASS advisories/bans/licenses/sources; allowed duplicate warnings retained for core-foundation/hashbrown/syn/windows-sys |
| `cargo cyclonedx --format json --all-features` | PASS: local `librarycube.cdx.json`, CycloneDX 1.3, 179 components; retained under ignored `target/release-validation-2026-09-17/`, not a production SBOM |
| `jq '{format:.bomFormat,spec:.specVersion,count:(.components\|length),valid:all(.components[]; (.name\|length)>0 and (.version\|length)>0 and (.licenses\|length)>0 and (.hashes\|length)>0)}' librarycube.cdx.json` | PASS: valid=true; all 179 have required name/version/license/hash metadata |
| `cargo test --locked --test ci_config` | PASS: 6 tests, including all context/tag/manifest/source dependencies and staged native artifact access |
| `python3 .circleci/test-native-archives.py` | PASS: all 25 real archive-policy cases |
| `bash .circleci/test-release-scripts.sh` | PASS: real disposable primary/subkey/protected/expired/revoked/weak OpenPGP fixtures, independently diagnostic source cases, strict native/provenance policy and 12 draft API scenarios including canonical resumed-draft metadata |
| `bash .circleci/test-macos-signing.sh` | PASS: 22 mocked Apple/native cases; NOT actual signing/notarization evidence |
| `bash -n .circleci/*.sh` | PASS |
| `python3 -m py_compile .circleci/*.py` | PASS |
| `git diff --check` | PASS |
| `jq -e '.bomFormat == "CycloneDX" and (.components\|length) == 179 and all(.components[]; (.name\|length)>0 and (.version\|length)>0 and (.licenses\|length)>0 and (.hashes\|length)>0)' target/release-validation-2026-09-17/librarycube.cdx.json` | PASS: retained local SBOM revalidated |
| `command -v pwsh powershell cargo-audit cargo-deny cargo-cyclonedx` | PowerShell absent; three Cargo tools present |

The fixture internally invokes the actual source/provenance/sign/verify/publish
scripts in isolated temporary repositories, fresh keyrings and a loopback API;
no ordinary development test used production credentials. Source negatives assert
their specific diagnostic; archive/native policy cases exercise the validators
directly instead of passing only because edited bytes have stale signatures.

Read-only audit commands included `git status --short`, `git status --branch
--short`, `git log -4 --oneline`, `git show -s --format=fuller HEAD`, `git tag -n`,
`git remote -v`, `git cat-file -p v0.5.0`, file inventories/line-counts/complete
task-doc-script reads, and `git diff --stat`, `--numstat`, and scoped review.
`gh api` read release/tag/current-commit/rulesets/private-reporting/protection
endpoints; `curl -fsS` read CircleCI v1.1 job listings/job 153/failed-step output.
First network reads failed DNS in the sandbox; approved read-only reruns succeeded.
An initial CircleCI listing jq projection failed on nullable `steps`; nullable
handling was corrected and explicit job details established the actual failure.
The protection endpoint returned HTTP 404, not a passing protection check.
The final current-commit status API confirmed the four actual context names from
the repository-security runbook and the failed `ci/circleci: release_source_gate`.
Generated validation output was safely moved (not deleted or committed):
`test ! -e target/release-validation-2026-09-17/librarycube.cdx.json && mkdir -p
target/release-validation-2026-09-17 && mv -n librarycube.cdx.json
target/release-validation-2026-09-17/librarycube.cdx.json` succeeded.

Not run: changed 28-case PowerShell policy suite, Windows SignTool/PFX production
execution, actual macOS codesign/notary/stapler/Gatekeeper, protected production
OpenPGP execution, hosted attestation verification, publication and independent
production downloads. These require the appropriate hosted OS/admin/credentials.
Previous Windows 150 verifies the older 26-case/PFX suite, not the new cases.

## 3. v0.5.0 requirement audit

The [full v0.4/v0.5 matrix](requirement-matrix-v0.4-v0.5.md) classifies each
core/security/hosted/admin/credential/production requirement separately.
The release audit preserves the user's complete A–I scope:

| Requirement | Repository evidence | Remaining production classification |
|---|---|---|
| A. Exact origin, SHA, Cargo/tag, annotated direct target/internal name, both signatures, imported usable public primary, full pin, no short ID/subkey ambiguity/unsigned source | COMPLETE: actual scripts and independent diagnostic signed Git fixtures | EXTERNAL CREDENTIAL REQUIRED: trusted PUBLIC source export/pin absent from gate 153; RELEASE-TIME ONLY: reviewed signed final revision/tag and hosted gate |
| B. Candidate/unreleased production language; detect public DRAFT contradiction; production metadata only after gates | COMPLETE: reconciled docs and final canonical payload/resume/public-refusal fixtures | EXTERNAL ADMIN ACTION REQUIRED: reconcile existing public Release and signed-tag/new-revision conflict |
| C. Exact Linux/Windows/macOS tar/DMG/checksums/SBOM/11 subjects/provenance/native receipts/12 signatures/public key (25 total); missing/extra/duplicate/empty/link/special/traversal/archive integrity/mutation/checksum/digest rejection | COMPLETE: canonical arrays, six CI tests, 25 real archive cases and real signing/provenance/mutation/API fixtures | RELEASE-TIME ONLY: actual final production manifest and independent native/downloaded verification |
| D. Exact executable, signer pin, code-signing EKU, trusted timestamp, signed CLI, verified repackage, final digests | COMPLETE: prepared SignTool/certificate/ZIP/hash/retest pipeline; previous hosted 26 cases + PFX; changed ZIP policy parity tested on Linux | RELEASE-TIME ONLY: changed 28-case suite on hosted Windows; EXTERNAL CREDENTIAL REQUIRED: real trusted signing identity/timestamp execution |
| E. Developer ID Application, runtime, Team/leaf pin, Accepted submit/info/log ID, staple/ticket, codesign/Gatekeeper/DMG/mounted binary/final hashes | COMPLETE: actual native script, 22 explicitly mocked cases, actual Linux archive/hash/receipt checks | EXTERNAL CREDENTIAL REQUIRED: Apple identities and real Accepted/stapled/native run; RELEASE-TIME ONLY: hosted OS and independently downloaded trust verification |
| F. Protected primary/subkey key, exact full primary pin, all 12 detached signatures, clean keyrings, wrong/expired/revoked/weak key/signature and final mutation rejection | COMPLETE: real disposable fixtures incl. RSA-1024 primary/signing-subkey failures, protected passphrase and strong subkey positives | EXTERNAL CREDENTIAL REQUIRED: externally provisioned production artifact key/pin; RELEASE-TIME ONLY: actual final signed bytes/current trusted revocation data |
| G. Repository/SHA/version/tag/ref/workflow/builder/exact final subject digests | COMPLETE: strict statement generation/verification and negative metadata/subject/digest/mutation cases | EXTERNAL INFRASTRUCTURE REQUIRED: externally verifiable hosted-builder evidence; tenant metadata is not a Build Level 2 claim |
| H. Main PR/review/resolution/four CI/strict/signed rules, force/deletion blocks, v* creation/update/deletion, private reporting and post-config/negative-test procedure | COMPLETE: exact owner runbook and read-only evidence limitations | EXTERNAL ADMIN ACTION REQUIRED: actual owner configuration/export/authorized disposable-ref negative tests |
| I. Exact source/windows/macos/publication contexts, all variable names/formats, no branch/PR context binding and source/native/approval dependencies | COMPLETE: config/tests/runbooks; staged native evidence now accessible before approval | EXTERNAL ADMIN ACTION REQUIRED: real groups/project/ref restrictions and negative secret-access proof; EXTERNAL CREDENTIAL REQUIRED: protected context values |

No REPOSITORY WORK REQUIRED item remains implemented only as a plan. COMPLETE
here means repository implementation with applicable local/previous hosted
evidence, never actual production trust. Final changed-native/hosted execution is
explicitly unverified, not skipped or asserted green.

## 4. Exact remaining owner actions (ordered)

1. Preserve/export the accidental public release metadata. Authenticate as its
   owner; edit the EXISTING release to an unpublished draft (`draft=true`) with
   honest candidate labeling. Verify ID/tag/draft/asset count through authenticated
   API. Do not create/delete/recreate the release or move/delete the tag as a
   shortcut. If hosted policy prevents unpublishing, keep NOT READY and resolve
   through owner-approved administration. No such external edit was made here.
2. Review the working-tree changes. The existing v0.5.0 tag points to old source;
   committing these fixes creates a different SHA. Record an explicit owner
   recovery decision for that tag/revision conflict. Do not retry the OLD tag
   and assume it runs these new fixes, use a mismatched tag/SHA, create a final
   production tag prematurely, or weaken the source gate.
3. Configure both active GitHub rulesets exactly as
   [repository-security.md](repository-security.md): main PR, >=1 reviewer,
   conversation resolution, all four actual candidate CI contexts/strict checks,
   signed commits, force/deletion blocks; v* creation/update/deletion restrictions
   and minimal documented bypass. Enable private vulnerability reporting. Retain
   authenticated rule exports, enabled=true and owner-authorized disposable-ref
   rejection evidence. Add the matching PUBLIC source key to the maintainer's
   GitHub signing keys; re-query the final commit's verification.
4. In CircleCI Organization Settings → Contexts, create/restrict the four exact
   contexts below to this project/release maintainers and approved protected tag
   AND exact reviewed SHA, with SSH reruns/untrusted config disallowed. Do not
   put credentials in project-wide variables. Export redacted restrictions and
   prove ordinary branch/PR/malicious config cannot obtain them; filters alone
   are insufficient. Use trusted ephemeral native release workers.
5. Independently obtain the trusted PUBLIC source identity, verify primary pin
   and usable signing subkeys, export its PUBLIC armored key/current revocations,
   and set both source variables. See [source preflight](circleci.md#source-identity-preflight).
   Missing fingerprint/public export must remain actionable failures.
6. Provision independently issued Windows/Apple identities and approved timestamp/
   notarization services, plus a dedicated protected production artifact-signing
   key and repository-only Contents read/write GitHub token. All expected pins
   must be published via an independent trusted channel, not taken from JSON.
7. Choose/configure externally verifiable hosted-builder provenance satisfying
   the release gate. Retain authenticated issuer/builder/ref/subject policy,
   attestation and independent verifier output. The current 25-asset manifest
   must not be silently expanded for an attestation: review/test a versioned
   manifest extension if distribution requires extra public assets.
8. Owner reviews/signs the final continuation revision, reruns all four candidate
   jobs (including changed Windows tests) on that exact SHA, and executes only an
   explicitly approved existing-tag recovery after ALL pre-tag prerequisites.
   Verify commit and direct signed annotated tag with the trusted public identity.
   Then the protected source gate and both production native jobs must pass.
9. Download staged native job artifacts/receipts. Independently perform Windows
   Authenticode/expected EKU/signer/timestamp and Apple accepted notarization/
   ticket/runtime/Team/leaf/codesign/Gatekeeper/DMG/payload checks. Verify hosted
   attestation. Retain commands, logs, IDs, source/workflow and final digests.
   Only then consider owner approval. Approval alone is not evidence.
10. After final provenance/OpenPGP/clean-manifest/remote draft checks pass, execute
    the gated publisher under exclusive draft ownership. Re-download every final
    asset, run the independent 25-file verifier with separately trusted pins and
    exact source/workflow metadata, repeat native trust checks and hosted policy,
    retain all evidence. Only this can establish PRODUCTION RELEASE VERIFIED.

### Every protected context variable (no secret values)

| Context | Required variables | Optional variables |
|---|---|---|
| `lbc-release-identity` | `LBC_MAINTAINER_SIGNING_FINGERPRINT`, `LBC_MAINTAINER_SIGNING_KEY_BASE64` (PUBLIC only) | None |
| `lbc-release-windows` | `LBC_WINDOWS_CERT_THUMBPRINT`, `LBC_WINDOWS_TIMESTAMP_URL` and externally available pinned private signing identity | `LBC_WINDOWS_CERT_STORE` (CurrentUser, default My), paired `LBC_WINDOWS_PFX_BASE64`/`LBC_WINDOWS_PFX_PASSWORD`, `LBC_SIGNTOOL_PATH` |
| `lbc-release-macos` | `LBC_MACOS_CERTIFICATE_BASE64`, `LBC_MACOS_CERTIFICATE_PASSWORD`, `LBC_MACOS_IDENTITY_SHA1`, `LBC_MACOS_TEAM_ID`, `LBC_NOTARY_KEY_BASE64`, `LBC_NOTARY_KEY_ID`, `LBC_NOTARY_ISSUER_ID` | None |
| `lbc-release` | `GITHUB_TOKEN`, `LBC_RELEASE_SIGNING_KEY_BASE64`, `LBC_RELEASE_SIGNING_FINGERPRINT`, PUBLIC `LBC_WINDOWS_CERT_THUMBPRINT`, `LBC_MACOS_IDENTITY_SHA1`, `LBC_MACOS_TEAM_ID` | `LBC_RELEASE_SIGNING_PASSPHRASE` (required when key is protected) |

OpenPGP fingerprints: uppercase full primary 40/64 hex, no spaces/prefix/short
ID/subkey pin. Native leaf SHA-1 pins: uppercase 40 hex. Team/key ID: 10 uppercase
alphanumeric characters. Issuer: UUID. Keys/P12/P8/PFX exports: base64 of the exact
external identity; secret exports/passwords masked in their restricted contexts.
Windows timestamp: approved HTTPS RFC 3161 SHA-256 service. Secret values must
never appear in Git, configs, artifacts, logs or this report.
CircleCI supplies exact lowercase full `CIRCLE_SHA1`, matching `CIRCLE_TAG`,
`CIRCLE_PROJECT_USERNAME=tsuna-n`, `CIRCLE_PROJECT_REPONAME=libraryCU`, actual
`CIRCLE_WORKFLOW_ID`, `CIRCLE_JOB`, `CIRCLE_BUILD_URL` and workflow `ci_cd` metadata.
Independent verification sets expected source/workflow values from the retained
trusted record, not downloaded provenance. The Linux verifier cannot establish
Windows/Apple PKI/Gatekeeper trust.

## 5. Next action

Owner: first inspect the actual release state using this read-only command, then
perform steps 1–5 above; do not approve/release/re-tag merely to retry gate 153:

```bash
gh api repos/tsuna-n/libraryCU/releases/390049263 --jq '{id,tag_name,name,draft,prerelease,assets:[.assets[].name]}'
```

The immediate CI credential action is CircleCI → Organization Settings →
Contexts → restricted `lbc-release-identity`: supply
`LBC_MAINTAINER_SIGNING_KEY_BASE64` and
`LBC_MAINTAINER_SIGNING_FINGERPRINT` from independently trusted PUBLIC material.
Do not supply the maintainer's private source key to CircleCI.
