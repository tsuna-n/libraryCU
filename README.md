# libraryCube

libraryCube (`lbc`) is a **local-first developer knowledge engine**.
Save solutions as Markdown, find them when an error returns, and reuse what
worked. Your notes stay on your machine. Core commands work offline; AI is optional.

**Save. Find. Reuse.**

## Install

Prebuilt binaries need **no Rust**. Platforms: Windows x86-64, Linux x86-64
(glibc), and universal macOS (Apple Silicon and Intel).

**Current source version: 0.6.0.** The `v0.6.0` release is being prepared;
verified binary assets are not published yet. The existing public `v0.5.0`
Release still has **zero assets**, as checked on 2026-11-12. Use the source
installation below until verified binaries are available.
See [v0.6.0 release notes](docs/release-0.6.0.md).

Once available, download your platform archive from
[GitHub Releases](https://github.com/tsuna-n/libraryCU/releases).
Before installing, follow the [verification and installation guide](docs/installation.md):
it retains the full signed-manifest and native platform trust checks.
Use an installer from a reviewed checkout and an independently authenticated
archive SHA-256. Replace `TRUSTED_SHA256` below with that 64-character digest.
A checksum downloaded beside an untrusted archive is not independent authentication.

Linux:

```bash
bash install.sh --archive ~/Downloads/lbc-0.6.0-x86_64-unknown-linux-gnu.tar.gz --sha256 TRUSTED_SHA256
```

macOS:

```bash
bash install.sh --archive ~/Downloads/lbc-0.6.0-universal-apple-darwin.tar.gz --sha256 TRUSTED_SHA256
```

Windows PowerShell:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 -Archive "$HOME\Downloads\lbc-0.6.0-x86_64-pc-windows-msvc.zip" -Sha256 TRUSTED_SHA256
```

Unix installs to `~/.local/bin`; ensure it is on `PATH`. Windows installs to
`%LOCALAPPDATA%\Programs\libraryCube\bin` and updates the user `PATH`;
open a new terminal. [Custom paths and uninstall](docs/installation.md#custom-paths-and-uninstall).

Source fallback (requires Rust and Cargo), from this checkout:

```bash
cargo install --path . --locked
lbc --version
```

The Rust crate is `librarycube`; the installed command is `lbc`.

## Set up once

Run this from the project directory:

```bash
lbc setup
```

Choose offline mode for no API key or network dependency. For scripts,
`lbc setup --yes` creates fresh offline defaults and preserves existing config.
Setup shows your project knowledge, personal notes, and config paths. It never
contacts AI providers. [Setup and optional AI](docs/quick-start.md#optional-ai).

## Five-minute quick start

Save a solved problem, find it, and explain a diagnostic using local evidence:

```bash
lbc learn --yes --new --problem "Rust E0308 String to &str mismatch" --solution "Borrow the String with &value" --verification "cargo check passed after borrowing"
lbc search E0308
lbc explain --stdin
```

For the last command, paste `error[E0308]: mismatched types`, then end input
with Ctrl-D (Unix/macOS) or Ctrl-Z followed by Enter (Windows).
The [quick start](docs/quick-start.md) also includes a copy-paste log-file example.

Prefer `lbc learn` for guided capture, or `lbc learn --from build.log` to prefill a
supported diagnostic offline; supply the solution that actually worked. Similar
notes are shown before a preview and `Save [y/N]`; cancellation saves nothing.
The scripted example uses `--new` to deliberately save a separate E0308 note
despite the built-in match. No automatic merging or overwriting occurs.
`learn` saves portable Markdown under the current directory’s `.lbc/knowledge`;
use `--user` for personal notes. Optional `--cause` and `--verification` are flags;
verification text records a historical check and is never executed. Review the
preview, including redacted log context, before saving.
Search shows source IDs and match reasons. Explain retrieves previous solutions
and records their evidence state; an inadequate match is reported honestly.

## How knowledge works

Your notes are ordinary Markdown with YAML metadata. Edit or back them up with
your own tools. Capture generates IDs and metadata; you can supply your own.
When relevance is comparable, sources rank project → user → installed package →
builtin. Use `--project PATH` to select an exact directory; LBC does not search
upward for a project root.

The everyday commands are `learn`, `add`, `search`, `ask`, `inspect`, `edit`,
and `explain`. Try `lbc --help` or `lbc COMMAND --help`.
Optional `--ai` enhances retrieved evidence and sends bounded, redacted context
to your configured provider. Review sensitive notes before enabling it.
`fix`, `apply-proposal`, and `rollback` are experimental source-changing tools.

Back up project knowledge with `lbc backup create ./knowledge-backup.json` and
restore with `lbc backup restore ./knowledge-backup.json`; use `--user` for
personal notes. `lbc export` and `lbc import` also support sharing bundles and
importing legacy Markdown. Backups preserve notes exactly and exclude private
application records. See [transfer behavior and conflict handling](docs/cli-reference.md#back-up-and-transfer-knowledge).
New `.lbc` stores stay private in Git by default; explicitly select reviewed notes
for sharing and use `lbc doctor` to [check Git safety](docs/cli-reference.md#git-safety).

## Learn more

- [Quick start](docs/quick-start.md): capture, search, and explain offline
- [Installation](docs/installation.md): binaries, verification, PATH, uninstall, and source builds
- [CLI reference](docs/cli-reference.md): commands, Markdown, AI, storage, safety, and development
- [Complete usage examples](docs/usage-examples.md): detailed command walkthroughs
- [Product direction](docs/product-direction.md) and [roadmap](ROADMAP.md)
- [Changelog](CHANGELOG.md) and [security policy](SECURITY.md)

## Release status and integrity

The `v0.6.0` candidate requires hosted CI, source identity checks, native signing
and owner approval before production publication. See the
[release notes](docs/release-0.6.0.md).
The empty public `v0.5.0` Release is not a verified production artifact set.
[Release evidence](docs/release-readiness-2026-11-08.md),
[full 25-file verification](docs/circleci.md#reproducible-independent-verification),
and [native trust checks](docs/native-code-signing.md) track the remaining gates.

## Development

See [development checks](docs/cli-reference.md#development). No release is
published by building or testing this checkout.
The [onboarding validation report](docs/onboarding-validation.md) records local
test results and the remaining native-platform limits.
