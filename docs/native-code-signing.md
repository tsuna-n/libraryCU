# Native Windows and macOS signing plan

This is an implementation and operations plan, not evidence that `v0.5.0`
binaries have native platform signatures. Detached OpenPGP signatures remain
the implemented cross-platform release mechanism.

## Invariant: sign the bytes that are shipped

Native signing must happen before packaging. The required order is:

1. Build and test the release binary.
2. Native-sign that exact binary and verify the native signature.
3. Put the verified binary into the final archive/package.
4. Generate the archive checksum and SBOM.
5. Generate provenance over those final files.
6. OpenPGP-sign and independently verify the final files.
7. Upload only the verified manifest to a draft release.

Never native-sign one binary and package a different build or rebuild after
signing.

## Windows Authenticode

External prerequisites:

- An organization-controlled code-signing certificate trusted by Windows,
  preferably backed by a managed signing service or hardware-protected key.
- A restricted CircleCI context available only to the Windows release job.
- A SHA-256 RFC 3161 timestamp service and the expected certificate thumbprint
  published in the private release runbook.

Implementation steps for `build_windows`:

1. Obtain the certificate through the approved managed signer. If a temporary
   PFX is unavoidable, decode it only into the job's temporary directory, import
   it into a temporary certificate store, mask its password, and delete both the
   file and imported key in an always-run cleanup step.
2. Resolve the signing certificate and compare its complete thumbprint with the
   configured expected value. Abort on zero or multiple matches.
3. Sign `target\release\lbc.exe` with `signtool.exe sign`, SHA-256 file digest,
   SHA-256 RFC 3161 timestamp digest, and the approved timestamp URL.
4. Run `signtool.exe verify /pa /all /v target\release\lbc.exe`; inspect the
   embedded signer and timestamp and abort on mismatch.
5. Run `lbc.exe --version` and the release-binary CLI suite against the signed
   executable. Only then copy it into the ZIP and create its checksum.

Completion evidence is the job log showing thumbprint comparison and successful
`signtool verify`, the signed executable's digest, a clean hosted Windows job on
the exact release commit/tag, and independent verification after downloading
the final ZIP. No private key or PFX may be retained as an artifact.

## macOS Developer ID and notarization

External prerequisites:

- Developer ID Application and, if an installer package is used, Developer ID
  Installer identities controlled by the maintainer organization.
- App Store Connect notarization credentials in a restricted macOS release
  context, preferably an issuer/key ID and scoped API private key.
- An ephemeral keychain and a documented Team ID/signing identity allowlist.

Implementation steps for `build_macos`:

1. Import the certificate into a randomly named temporary keychain, set the
   keychain partition list only for `codesign`, and verify the full identity and
   Team ID before use. Remove the keychain in an always-run cleanup step.
2. After `lipo` creates the universal binary, sign that final binary with
   `codesign --force --options runtime --timestamp --sign IDENTITY lbc`.
3. Verify with `codesign --verify --strict --verbose=2`, inspect the designated
   requirement and Team ID, run `spctl --assess --type execute`, then run the
   release-binary CLI suite against the signed universal binary.
4. Package the signed binary as a Developer ID-signed `.pkg` if offline stapled
   notarization evidence is required. Submit that final package with
   `xcrun notarytool submit --wait`; require an `Accepted` result.
5. Run `xcrun stapler staple` and `xcrun stapler validate` on the package, then
   `spctl --assess --type install`. A raw tar archive cannot carry a stapled
   notarization ticket; it must remain documented as OpenPGP-authenticated only
   or be accompanied/replaced by the notarized package.
6. Generate checksums, provenance, and detached OpenPGP signatures only after
   the final native-signed/notarized deliverable bytes exist.

Completion evidence is the exact hosted macOS job and tag, verified identity and
Team ID, accepted notarization request ID/log, successful stapler and Gatekeeper
assessment, final digest, and independent verification of the downloaded asset.
Certificate/API private keys must never appear in logs, workspaces, or release
artifacts.
