# Install libraryCube

The command is `lbc`. Prebuilt release binaries require no Rust, Cargo, Node,
Python, or AI provider. Core knowledge workflows work offline after installation.

The current source version is **0.6.0**; see the [release notes](release-0.6.0.md).

**Current availability (2026-11-12):** the public `v0.5.0` GitHub Release has zero
assets. It is not a verified production release. The binary instructions below
apply when verified assets are published; they cannot download a binary today.
Use the [source fallback](#source-fallback-requires-rust) for now. See the
[release audit](release-readiness-2026-11-08.md) for evidence and outstanding gates.

## Choose a binary

Download using your browser from
[GitHub Releases](https://github.com/tsuna-n/libraryCU/releases).
The release pipeline uses these filenames for version 0.6.0 (pending publication):

| Platform | Archive |
|---|---|
| Linux x86-64, glibc | `lbc-0.6.0-x86_64-unknown-linux-gnu.tar.gz` |
| macOS Intel or Apple Silicon | `lbc-0.6.0-universal-apple-darwin.tar.gz` |
| Windows x86-64 | `lbc-0.6.0-x86_64-pc-windows-msvc.zip` |

Do not substitute another CPU architecture. These instructions do not claim
Linux ARM64, musl, or Windows ARM64 release artifacts exist.

## Verify before installation

Use the [clean-keyring full-manifest procedure](circleci.md#reproducible-independent-verification)
before extracting or executing downloaded content. It verifies all 25 files,
trusted signing fingerprints, checksums, SBOM, and exact source/version/workflow
provenance. Obtain the maintainer’s full signing fingerprint through a separate
trusted channel. A key included with the archive is not its own trust anchor.
A checksum from the same untrusted source as the archive checks integrity,
not independent authenticity.

Preserve the [native platform trust checks](native-code-signing.md): Windows
Authenticode, and macOS Developer ID, hardened runtime, notarization, and the
stapled DMG. The DMG delivers the offline notarization ticket; a raw tar cannot.
Production signing credentials and native trust have not been verified by local
Linux tests. Do not bypass platform warnings to compensate for missing evidence.

After this verification, use the archive’s authenticated SHA-256 with a trusted
installer from a reviewed source checkout. The installer checks the digest and
archive safety before installing; it does not replace signature, provenance,
or native trust verification. Do not run an installer downloaded alongside
unverified assets. There is no automatic downloader or `curl | bash` step.

## Linux and macOS

From the reviewed checkout containing `install.sh`, replace `TRUSTED_SHA256`
with the authenticated **64 hexadecimal characters**, not the placeholder text.

Linux:

```bash
bash install.sh --archive ~/Downloads/lbc-0.6.0-x86_64-unknown-linux-gnu.tar.gz --sha256 TRUSTED_SHA256
```

macOS:

```bash
bash install.sh --archive ~/Downloads/lbc-0.6.0-universal-apple-darwin.tar.gz --sha256 TRUSTED_SHA256
```

Installation defaults to `~/.local/bin`. Ensure that directory is on your `PATH`.
For the current shell, if needed:

```bash
export PATH="$HOME/.local/bin:$PATH"
lbc --version
```

Add that export to your shell profile for future terminals. Installation itself
uses no Rust. An installer inside an already fully verified and safely extracted
release archive also supports `./install.sh`.

## Windows PowerShell

From the reviewed checkout containing `install.ps1`:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 -Archive "$HOME\Downloads\lbc-0.6.0-x86_64-pc-windows-msvc.zip" -Sha256 TRUSTED_SHA256
```

Replace `TRUSTED_SHA256` with the authenticated 64-character hexadecimal digest.
The execution-policy flag permits this reviewed script to run; it does not
verify the script or binary. Installation defaults to
`%LOCALAPPDATA%\Programs\libraryCube\bin` and updates the user `PATH`.
Open a new PowerShell window, then run:

```powershell
lbc --version
lbc setup
```

The installer inside an already fully verified and safely extracted release
archive also supports:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1
```

## Custom paths and uninstall

Archive installation accepts the existing prefix options: append
`--prefix "$HOME/bin"` on Unix or `-Prefix 'C:\Tools\lbc'` on Windows.
System installation is explicit: `--system` (Unix, may use sudo) or `-System`
(Windows, elevated PowerShell). User-local installation needs no administrator.

Run a trusted installer again with the original destination to remove `lbc`:

```bash
bash install.sh --uninstall
bash install.sh --uninstall --prefix "$HOME/bin"
```

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 -Uninstall
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 -Uninstall -Prefix 'C:\Tools\lbc'
```

These remove the installed executable, not your knowledge or config.

## Source fallback (requires Rust)

From a reviewed checkout, with Rust and Cargo installed:

```bash
cargo install --path . --locked
lbc --version
lbc setup
```

Cargo installs to its bin directory (normally `~/.cargo/bin`), which must be on
`PATH`. Initial dependency downloads may need network access. Alternatively,
`cargo build --locked --release` produces `target/release/lbc` (`lbc.exe` on
Windows). Source-checkout `./install.sh` or `install.ps1` builds with Cargo;
this is separate from binary archive installation.

Continue with the [five-minute quick start](quick-start.md).
