# Installation and setup validation — 2026-11-08

The three onboarding tasks are implemented locally on the existing 0.5.0
worktree. No push, tag change, publication or production credential provisioning
was performed. The earlier release-hardening changes and pre-existing untracked
`nvim.log` were preserved. This report supersedes earlier test counts for this
changed worktree; the dated release audit remains a historical snapshot.

## Delivered behavior

- Unix/macOS: `bash install.sh --archive FILE --sha256 TRUSTED_DIGEST`.
  Windows: `install.ps1 -Archive FILE -Sha256 TRUSTED_DIGEST`.
  Both hash a private copy before inspecting the archive or changing the prefix,
  require canonical platform packages and regular bounded members, and install
  only the verified executable. They never execute an installer from the archive.
  Source and extracted-binary installation remain supported. Binary mode does
  not call Rust/Cargo/rustup or require Python. SHA-256 authenticates the archive
  only when the expected digest is independently trusted; existing complete
  signing/provenance/native trust procedures remain required and documented.
- Installer output checks PATH, gives safely quoted shell guidance, and leads
  with setup/learn/search/explain. Uninstall removes the executable and, on Windows,
  its PATH entry; knowledge, config and unrelated prefix files remain intact.
- `lbc setup` detects the OS/current explicit project, initializes the established
  `.lbc/knowledge` and personal notes directories, and configures offline or an
  existing AI provider. `--yes` uses safe fresh defaults and preserves existing
  settings; `--project`, `--provider`, `--model`, `--base-url`, and `--json` are
  available. Setup validates before mutation, saves under one existing lock using
  private atomic replacement, and reloads configuration. Unknown TOML keys remain;
  comments/formatting can be rewritten. Keys are environment-only; setup never
  requests or persists a key, contacts a provider, or claims credential validity.
- Explicit LBC_CONFIG/XDG/HOME precedence remains. Without HOME/XDG, existing
  relative config/stores remain in place; fresh Windows profiles use USERPROFILE
  consistently for config, notes, packages and history. No data is migrated.
- README leads with installation and the three everyday commands. The
  [quick start](quick-start.md), [installation guide](installation.md) and
  [CLI reference](cli-reference.md) contain real examples and preserve deeper
  safety/release documentation.

## Actual verification

Local platform: Linux x86-64; Rust 1.99.1. Existing hosted CI pins Rust 1.97.1.
Tests use isolated, owned temporary directories and no production credentials.

| Check | Result |
|---|---|
| `cargo fmt --check` | PASS |
| `cargo check --locked` | PASS |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | PASS |
| `cargo test --locked` | PASS: **230 tests**; 123 library, 9 CI, 75 CLI, 12 filesystem, 3 installer, 6 privacy, 2 user-path tests; none failed/ignored |
| `cargo build --locked --release` | PASS; `lbc 0.5.0` |
| `LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli` | PASS: **75 tests** against the real release executable |
| `cargo audit`, `cargo deny check` | PASS with existing advisory/license/source policy; duplicate warnings retained |
| Existing `test-release-scripts.sh` | PASS: disposable signed-source/OpenPGP/provenance cases, 25 real archive cases, 12 mock draft API scenarios |
| Every `.circleci/*.sh` and `install.sh` checked with `bash -n` | PASS |
| Documentation local path links | PASS |
| Fixed retrieval benchmark, unchanged labels and baseline | PASS: no per-case regression; synthetic project memory 6/6 |
| Real release archive through GNU tar and BSD tar on Linux | PASS: bad digest rejected before prefix creation; no Rust calls; exact binary installed; setup → learn → search → explain; repeated uninstall preserved config/knowledge |
| README/quick-start commands with release binary | PASS: offline setup, capture, search, explain file/stdin, local doctor; no API keys needed |
| `git diff --check` | PASS |

Installer tests cover invalid/corrupt archives and executables, empty/missing/
duplicate/unexpected/oversized members, links, special files, traversal, absolute
paths, platform mismatch, unusual prefix quoting and no-Rust installation. The
packaged installer is deliberately invalid in the positive fixture, proving it
is not executed. Setup regressions cover every supported provider, config/unknown
key preservation, idempotence, secrets in URL/model arguments, unsafe destinations,
cancellation/EOF and no provider connection. USERPROFILE regressions reproduce
the old wrong-directory behavior before repair and preserve existing stores.

The sandbox initially remapped `/tmp` ownership, so an initial path test failed
at the unchanged owner gate. Actual runtime checks ran outside that sandbox with
owned fixture directories; no ownership/ACL or credential gate was relaxed.
Independent reviews found and fixed the empty-archive diagnostic and the
installer's invalid example treating an error string as an input filename.

The benchmark remains a small regression corpus: Top-1/Top-3 31/31, strict
citation relevance 33/40 and result precision 35/75, not real-world correctness.
Logs, local unsigned archive and smoke receipts are retained under ignored
`target/onboarding-validation/`. Source identity is starting
`21a8ea21362b0f5502d357cd045cd972428037fb` **plus local changes**. The local source
manifest includes untracked implementation/test files; it is not a signed hosted
provenance statement.

## Changed files for this task

- `install.sh`, `install.ps1`, `tests/install.rs`: verified archives and installer
  compatibility/PATH/data-preservation tests.
- `src/cli/args.rs`, `src/cli/dispatch.rs`, `src/cli/commands/mod.rs`,
  `src/cli/commands/setup.rs`, `tests/cli.rs`, `tests/cli/setup.rs`: setup command.
- `src/config/settings.rs`, `src/knowledge/packages.rs`,
  `src/knowledge/storage.rs`, `src/history.rs`, `tests/user_paths.rs`: safe
  config updates and compatible user-directory resolution.
- `README.md`, `docs/installation.md`, `docs/quick-start.md`,
  `docs/cli-reference.md`, `docs/usage-examples.md`, `CHANGELOG.md`, this report,
  `tasks/plan.md`, `tasks/todo.md`: guides, plan and verification.

## Remaining limits

Native Windows/macOS execution was unavailable locally. Native Windows ZIP/PATH
tests were added to the standard Cargo suite; both platforms require fresh hosted
CI on the final reviewed revision. BSD tar on Linux and printed shell guidance
are not native macOS/fish/zsh runtime evidence. No production signatures,
Authenticode, notarization, or downloaded production binaries were verified.

As recorded in the [release audit](release-readiness-2026-11-08.md), the public
v0.5.0 release has zero assets and outstanding source/signing/admin gates.
Binary download instructions cannot be exercised against that empty release.
For now the working source fallback requires Rust; binary installation itself
does not. Publication remains a separately authorized owner action. Supported
archive targets are Linux x86-64 glibc, universal macOS and Windows x86-64; no
automatic downloader, ARM Linux/Windows package or musl package was added.
