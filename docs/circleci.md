# CircleCI CI/CD

The CircleCI pipeline in `.circleci/config.yml` validates every branch and pull
request on Linux, macOS, and Windows. Version tags run the same platform gates
and, only after their prerequisites pass, publish all release assets together.

## CI behavior

The `build_and_test` job uses the pinned `cimg/rust:1.97.1` image and runs the
same gates documented for local development:

```bash
cargo fmt --check
cargo check --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli
git diff --exit-code
git diff --check
```

The independent `dependency_security` job installs pinned `cargo-audit 0.22.2`,
`cargo-deny 0.20.2`, and `cargo-cyclonedx 0.5.9`. It enforces the RustSec,
license, banned-crate, duplicate-version, and source policies in `deny.toml`, then
generates and validates `lbc-VERSION.cdx.json`. The SBOM must contain dependency
names, versions, licenses, and registry hashes. It also runs
`.circleci/test-release-scripts.sh` with a disposable one-day OpenPGP key and a
local GitHub API fixture, so provenance/signature failure modes and draft
publication behavior are exercised without production credentials or network
publication. The fixture proves complete and resumed 17-asset publication,
private retention after an interrupted upload, and rejection of missing,
malformed, unexpected, duplicate, remotely tampered, or already-public state.

Configuration, data, cache, and temporary directories are isolated inside each
job. Cargo downloads and Linux build output are cached by architecture, Rust
version, and `Cargo.lock` checksum.

The platform jobs produce and test these installable archives:

- `lbc-VERSION-x86_64-unknown-linux-gnu.tar.gz`, with `lbc` and `install.sh`.
- `lbc-VERSION-universal-apple-darwin.tar.gz`, with an Intel and Apple Silicon
  universal `lbc` binary and `install.sh`.
- `lbc-VERSION-x86_64-pc-windows-msvc.zip`, with `lbc.exe` and `install.ps1`.

Every archive has a sibling `.sha256` file. The release job refuses to continue
unless all three exact archives, their matching checksum files, and a valid SBOM
are present. It records the source commit, CircleCI workflow, and every input
SHA-256 digest in `lbc-VERSION.provenance.json`. It signs all archives, checksums,
SBOM, and provenance files with detached armored OpenPGP signatures, imports the
exported public key into a clean keyring, and verifies the exact manifest again
before upload. The GitHub Release remains a draft until every remote asset name,
byte size, upload state, and SHA-256 digest exactly matches the verified local
manifest; failures leave a non-public draft instead of a partial public release.
The macOS and Windows jobs run the
full test suite, exercise the packaged release binary, install it into a
temporary prefix, verify the installed version, and exercise uninstall. These
jobs run for every branch, pull request, and release tag so the exact candidate
can be validated before tagging. Release publication itself remains tag-only.

The macOS universal binary receives an ad-hoc signature so its combined binary
is internally consistent. Native Developer ID signing/notarization and Windows
Authenticode remain separate plans that require Apple and Microsoft credentials;
the OpenPGP release signature still authenticates both downloadable archives.
The exact ordering, credential boundaries, commands, failure behavior, and
completion evidence are defined in
[native-code-signing.md](native-code-signing.md).

## CD setup

1. Add this GitHub repository as a CircleCI project. CircleCI automatically
   reads `.circleci/config.yml`.
2. Create a restricted CircleCI context named exactly `lbc-release`; the publish
   workflow is bound to this context. Add `GITHUB_TOKEN` to it. Use a fine-grained
   GitHub token limited to this repository with **Contents: Read and write**
   permission. CI for branches and pull requests does not receive this secret.
3. Create a dedicated OpenPGP release-signing subkey kept outside the repository.
   In the `lbc-release` context, add the armored private key encoded
   as `LBC_RELEASE_SIGNING_KEY_BASE64` and the uppercase full fingerprint as
   `LBC_RELEASE_SIGNING_FINGERPRINT`. Mask both values and restrict the context
   to release maintainers. Publish the fingerprint through a separate trusted
   channel so users can authenticate the exported public key.
4. Apply and test the GitHub controls in
   [repository-security.md](repository-security.md), including required CI and
   dependency-security checks, one approval, resolved conversations, and blocked
   force pushes/deletion.
5. Update the package version in `Cargo.toml` and `Cargo.lock`, merge the tested
   change, then push a matching annotated tag:

   ```bash
   git tag -s v0.5.0 -m "libraryCube v0.5.0"
   git verify-tag v0.5.0
   git push origin v0.5.0
   ```

Tags must match `vMAJOR.MINOR.PATCH`, with optional SemVer prerelease and build
suffixes, and must equal `v` plus the package version in `Cargo.toml`. After all
CI gates pass on all three operating systems and dependency security,
`publish_github_release` creates a
draft GitHub Release with generated notes and uploads every archive, checksum,
SBOM, provenance record, public signing key, and detached signature. It publishes
the draft only after the complete remote asset names, sizes, states, and SHA-256
digests are confirmed. Re-running a failed draft replaces expected assets of the
same name; unexpected or duplicate assets prevent publication, and the script
refuses to modify an already-public release.

The provenance is generated and signed by repository-controlled scripts running
inside CircleCI. It is SLSA v1-shaped metadata, not hosted-builder/control-plane
attestation and not a claim of SLSA Build Level 2. To complete that gate, move
attestation generation/signing to a hosted control plane whose identity the
repository job cannot forge, bind every final subject digest and the exact source
revision, verify issuer/builder/workflow/ref policy with an independent verifier,
and retain the signed attestation plus verification output with the release.

The pipeline deliberately does not publish to crates.io. Add a separate,
approval-gated job and a scoped `CARGO_REGISTRY_TOKEN` if crate publication is
required later.
