# Native release signing and independent verification

Repository implementation is prepared, not proof of production execution.
Candidate macOS binaries are ad-hoc signed; Windows candidates are unsigned.
Actual Developer ID, Authenticode, timestamp and notarization execution remains
`EXTERNAL CREDENTIAL REQUIRED`. No production identity was generated here.

## Sign the tested bytes that are shipped

All four credential-free candidate gates produce tested archives and an SBOM.
The tag-only signed-source gate verifies the exact commit and signed annotated
tag with a pinned maintainer public key. Separate credential-scoped native jobs
unpack candidates, sign the existing binary (never rebuild it), verify it, run
the release-binary CLI suite, and repackage the same verified bytes into
`production-dist`. Checksums and digest-bound signing records follow successful
native checks. The publisher uses only native outputs plus Linux/SBOM candidates;
provenance, OpenPGP and clean-keyring verification precede draft upload.
Owner approval is a hold, not native trust or hosted-attestation evidence.

Packages contain exactly the binary, installer, README, CHANGELOG and LICENSE.
The 19-case real ZIP/tar policy suite covers slash/backslash normalization,
absolute/traversal paths, normalized duplicates, missing members, symlinks,
hardlinks, Windows reparse metadata and other special file types. Traversal,
duplicate, missing, unexpected, linked and oversized members fail.
Private staging, credentials and mounts are cleaned in EXIT/finally handlers;
no private material is persisted to workspaces, artifacts or logs. Credentialed
jobs require trusted ephemeral release workers, not untrusted PR workers.

## Windows: implemented `.circleci/sign-windows-release.ps1`

Requires Windows SDK SignTool, an externally issued Windows-trusted code-signing
identity and an approved HTTPS RFC 3161 timestamp service. Managed/HSM keys
exposed through the Windows certificate provider can use CurrentUser My or a
configured store. An optional supplied PFX is decoded in memory, imported into
a unique CurrentUser `LbcRelease-<GUID>` store without `PersistKeySet`, kept
alive until signing finishes, then removed/disposed. Existing managed identities
are never deleted.

Context `lbc-release-windows`:

| Variable | Owner-supplied value |
|---|---|
| `LBC_WINDOWS_CERT_THUMBPRINT` | Full uppercase 40-hex leaf thumbprint |
| `LBC_WINDOWS_TIMESTAMP_URL` | Approved HTTPS SHA-256 RFC 3161 service |
| `LBC_WINDOWS_CERT_STORE` | Optional CurrentUser store; default My |
| `LBC_WINDOWS_PFX_BASE64`, `LBC_WINDOWS_PFX_PASSWORD` | Optional masked externally issued PFX/password; both required together |
| `LBC_SIGNTOOL_PATH` | Optional SDK tool path; otherwise newest installed x64 SDK tool |

Zero/multiple pinned matches, absent private keys, expired/not-yet-valid
certificates, wrong code-signing EKU and pre-signed candidates fail.
SignTool uses `/sha1 PIN /fd SHA256 /tr URL /td SHA256`.
Verification `/pa /all /tw /v` must exit zero, including no warnings.
Get-AuthenticodeSignature must report Valid, embedded Authenticode, the pinned
signer and a timestamp certificate. Verification repeats after signed CLI tests
and ZIP re-extraction; packaged binary hashes must match.

26 mocked cases exercise certificate policy, timestamps, embedded-vs-catalog
signatures, traversal, failures and PFX cleanup, not actual certificate trust.
`.circleci/test-windows-pfx.ps1` additionally runs three real lifecycle cases
only on the ephemeral hosted Windows VM: disposable one-day self-signed TEST
certificate import/private-key availability/cleanup, wrong pin and malformed
PFX. The TEST certificate is never installed as a trusted root.

## macOS: implemented `.circleci/sign-macos-release.sh`

Context `lbc-release-macos`:

| Variable | Owner-supplied value |
|---|---|
| `LBC_MACOS_CERTIFICATE_BASE64` | Masked externally issued Developer ID Application P12 |
| `LBC_MACOS_CERTIFICATE_PASSWORD` | Masked P12 password |
| `LBC_MACOS_IDENTITY_SHA1` | Full uppercase 40-hex leaf fingerprint |
| `LBC_MACOS_TEAM_ID` | Expected 10-character Apple Team ID |
| `LBC_NOTARY_KEY_BASE64` | Masked App Store Connect notarization API P8 private key |
| `LBC_NOTARY_KEY_ID` | Expected 10-character API key ID |
| `LBC_NOTARY_ISSUER_ID` | App Store Connect issuer UUID |

Imports into an ephemeral private keychain and checks the pinned identity.
The final universal CLI is signed with hardened runtime and secure timestamp.
Strict codesign verification requires Apple anchor, Developer ID Application
certificate OID and pinned Team ID. Extracted leaf SHA-1 must match the trusted
pin; display checks require Developer ID authority, timestamp and CLI runtime.
Signed version and the full release-binary CLI suite must pass.

A signed DMG contains the same CLI and installer, not a fabricated app bundle.
notarytool submit --wait must return Accepted and a well-formed request UUID;
info and log must match the ID and successful status/code. Stapling and ticket
validation must succeed. Final codesign and Gatekeeper execute/open assessments
follow notarization. The read-only mounted DMG binary signature/digest must
match the tested signed binary. Only then is the final tar created and its
binary digest checked again.

Raw tar cannot carry a stapled ticket. The additional
`lbc-VERSION-universal-apple-darwin.dmg` is the offline-ticket delivery path;
both deliverables contain the same Developer ID-signed binary. Install from a
verified mounted DMG using its install.sh, then detach.
22 mocked native-command cases run real archive/hash/record checks, not real
Apple credentials, notarization or Gatekeeper trust.

## Independent downloaded-asset verification

Authenticate all 25 OpenPGP assets using [circleci.md](circleci.md).
Obtain expected source/workflow, native pins and Team ID independently.
Digest-bound JSON records are tenant job verification records, not standalone
native trust evidence or unforgeable hosted-builder attestations.

On a clean Windows machine, safely extract the authenticated ZIP:

```powershell
signtool.exe verify /pa /all /tw /v .\lbc-0.5.0-x86_64-pc-windows-msvc\lbc.exe
if ($LASTEXITCODE -ne 0) { throw 'Authenticode failed or warned' }
$signature = Get-AuthenticodeSignature -LiteralPath .\lbc-0.5.0-x86_64-pc-windows-msvc\lbc.exe
if ($signature.Status -ne 'Valid' -or $signature.SignatureType -ne 'Authenticode' -or
    $signature.SignerCertificate.Thumbprint -cne $env:LBC_WINDOWS_CERT_THUMBPRINT -or
    $null -eq $signature.TimeStamperCertificate) { throw 'Signer/trust/timestamp mismatch' }
```

On a clean Mac, authenticate DMG signature/checksum, validate its ticket and
Gatekeeper assessment, then mount read-only:

```bash
dmg=lbc-0.5.0-universal-apple-darwin.dmg
xcrun stapler validate "$dmg"
codesign --verify --strict --verbose=2 "$dmg"
spctl --assess --type open --context context:primary-signature --verbose=2 "$dmg"
hdiutil attach -readonly -nobrowse "$dmg"
```

Verify both DMG and mounted/extracted CLI with this requirement, substituting
the independently trusted Team ID and actual PATH:

```bash
codesign --verify --strict --verbose=2 \
  -R='anchor apple generic and certificate leaf[subject.OU] = "TEAM_ID" and certificate leaf[field.1.2.840.113635.100.6.1.13] exists' PATH
codesign --display --verbose=4 PATH
codesign --display --extract-certificates ./release-certificate PATH
openssl x509 -inform DER -in release-certificate0 -noout -fingerprint -sha1
spctl --assess --type execute --verbose=2 MOUNTED_BINARY_PATH
```

Compare leaf fingerprint, Team ID, Developer ID authority and timestamp to
trusted values; require CLI runtime. Compare mounted binary SHA-256 with the
record and safely extracted tar binary. Detach with hdiutil detach MOUNT_PATH.
Retain exact tag/SHA, job URLs, signer pins, accepted notarization ID/log,
ticket, native verifier output and final/downloaded digests in the release
record. External PKI/revocation availability and SmartScreen reputation mean a
certificate does not guarantee absence of user prompts.

References: [Microsoft SignTool](https://learn.microsoft.com/en-us/windows/win32/seccrypto/signtool),
[Apple notarization workflow](https://developer.apple.com/documentation/security/customizing-the-notarization-workflow).
