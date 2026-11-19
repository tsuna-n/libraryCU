# libraryCube

libraryCube (`lbc`) is a **local-first developer knowledge engine**.
Save solutions as Markdown, find them when an error returns, and reuse what
worked. The engine runs offline and uses deterministic retrieval over local knowledge.

**Save. Find. Reuse.**

## Install

Prebuilt binaries need **no Rust**. Platforms: Windows x86-64, Linux x86-64
(glibc), and universal macOS (Apple Silicon and Intel).

**Current source version is 0.7.1.** Use the source installation below until
verified binary assets are available. The public `v0.5.0` release had zero assets
when checked on 2026-11-12. [Changelog](CHANGELOG.md).

Once available, download your platform archive from
[GitHub Releases](https://github.com/tsuna-n/libraryCU/releases).
Before installing, follow the [verification and installation guide](docs/installation.md):
it retains the full signed-manifest and native platform trust checks.
Use an installer from a reviewed checkout and an independently authenticated
archive SHA-256. Replace `TRUSTED_SHA256` below with that 64-character digest.
A checksum downloaded beside an untrusted archive is not independent authentication.

Linux:

```bash
bash install.sh --archive ~/Downloads/lbc-0.7.1-x86_64-unknown-linux-gnu.tar.gz --sha256 TRUSTED_SHA256
```

macOS:

```bash
bash install.sh --archive ~/Downloads/lbc-0.7.1-universal-apple-darwin.tar.gz --sha256 TRUSTED_SHA256
```

Windows PowerShell:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\install.ps1 -Archive "$HOME\Downloads\lbc-0.7.1-x86_64-pc-windows-msvc.zip" -Sha256 TRUSTED_SHA256
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

Setup initializes your project and personal knowledge directories and shows
their paths and the config path. Use `lbc setup --yes` in scripts to skip
confirmation. [Setup details](docs/quick-start.md#set-up-once).

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
`fix` offers local diagnostic guidance. `apply-proposal` and `rollback` remain
available for compatible saved proposals and recovery records.

Codex, Claude Code, OpenCode, and other coding agents can consume CLI
`--json` output or use
`lbc mcp --project /absolute/path/to/project --project-only` for three read-only
tools over stdio. See the [Codex CLI and OpenCode setup](docs/mcp.md).
Optional [VS Code tasks and Neovim Lua](docs/editor-integration.md) send
selections or error logs through the CLI. `search`, `ask`, and `explain` also
accept `--project-only`. See [project memory](docs/project-memory.md) for scope
and context ranking. Historical knowledge remains guidance to check against the
current problem.

Back up project knowledge with `lbc backup create ./knowledge-backup.json` and
restore with `lbc backup restore ./knowledge-backup.json`; use `--user` for
personal notes. `lbc export` and `lbc import` also support sharing bundles and
importing legacy Markdown. Backups preserve notes exactly and exclude private
application records. See [transfer behavior and conflict handling](docs/cli-reference.md#back-up-and-transfer-knowledge).
New `.lbc` stores stay private in Git by default; explicitly select reviewed notes
for sharing and use `lbc doctor` to [check Git safety](docs/cli-reference.md#git-safety).

Maintain the library with `lbc knowledge validate` (read-only warnings) and
`lbc knowledge review --max-age-days 365` (review candidates). Edits keep stable
IDs, custom YAML metadata, and exact previous revisions. Install the optional
40-note Python/FastAPI package offline with
`lbc knowledge install --bundled python-fastapi-basics`; use
`lbc knowledge list` to check installed packages and integrity.

## Upgrading to the offline CLI

Existing Markdown knowledge, project and user stores, installed packages, and
qualified source IDs work in place. Remove `--ai` and provider options from
scripts. See the [upgrade notes](docs/cli-reference.md#upgrading-to-the-offline-cli)
for config, JSON, and saved proposal compatibility.

## Learn more

- [Quick start](docs/quick-start.md): capture, search, and explain offline
- [Installation](docs/installation.md): binaries, verification, PATH, uninstall, and source builds
- [CLI reference](docs/cli-reference.md): commands, Markdown, storage, safety, and development
- [Complete usage examples](docs/usage-examples.md): detailed command walkthroughs
- [Product direction](docs/product-direction.md) and [roadmap](ROADMAP.md)
- [Changelog](CHANGELOG.md) and [security policy](SECURITY.md)

## Release status and integrity

Publishing `v0.7.1` requires hosted CI, source identity checks, native signing,
and owner approval. [Release procedure](docs/circleci.md).

Use `lbc-main-reviewed-signed-ci` as the GitHub ruleset name for `main`.
Version tags use separate rulesets for creation and immutability.
[Repository controls guide](docs/repository-controls/README.md).

The empty public `v0.5.0` Release is not a verified production artifact set.
[Release evidence](docs/release-readiness-2026-11-08.md),
[full 25-file verification](docs/circleci.md#reproducible-independent-verification),
and [native trust checks](docs/native-code-signing.md) track the remaining gates.

## Development

See [development checks](docs/cli-reference.md#development). No release is
published by building or testing this checkout.
The [onboarding validation report](docs/onboarding-validation.md) records local
test results and the remaining native-platform limits.
The [offline retrieval optimization report](docs/retrieval-offline-optimization.md)
records the before-and-after accuracy, timing, and memory measurements.
