# 0.4.0 release audit — 2026-09-07

This release adds explicit fix verification, local recovery records,
broader diagnostic parsing/rules, and a three-OS GitHub Actions workflow.
Cargo package and lockfile versions agree on 0.4.0.

## Behavior covered

- `fix --verify "COMMAND"` requires `--ai --apply`. The command is selected by
  the CLI user, parsed with portable shell-style quoting, and spawned directly
  in the exact project directory. No provider or retrieved note chooses it.
- A 120-second default deadline (1–3600 configurable) includes process and output
  completion. Stdout/stderr retain at most 32 KiB each and are redacted. Failure,
  spawn errors, and timeouts are distinct from a successful exit. A source edit
  during verification prevents a `passed` claim.
- Private bounded recovery records precede source publication. Explicit rollback
  works across invocations; automatic rollback follows unsuccessful verification.
  Both preserve conflicting edits and reject unsafe paths/storage. Records remain
  for manual recovery, and repeated restoration is refused if content differs.
- TypeScript tsc, Node JS/TS frames and explicit error codes, and Go compiler
  locations join the existing Rust/Python parsers. Tests exercise Windows drive
  paths, Unicode, CRLF, ANSI, multiple errors, and malformed location data.
- Known rules cover selected TypeScript type/module failures, Node module
  resolution, and Go undefined identifiers; `explain` remains read-only/offline.
- CI uses Linux/macOS/Windows runners, locked dependency resolution, strict
  Clippy, debug and release-binary tests, and tested binary artifacts. Official
  checkout/upload actions are pinned to immutable commits.

## Validation

Local Linux validation passed for the implementation introduced in `214f64c`:

```bash
cargo fmt --check
cargo check --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli
git diff --check
```

The final full suite passed **141 tests** (78 library, 51 CLI, 6 filesystem,
6 privacy), zero failed or ignored. All **51 release-binary CLI tests** also
passed, including the real 45-second provider timeout. The release executable
reports `lbc 0.4.0`; `fix --help` and `rollback --help` expose the new options.
Fixtures use local mock providers and isolated project/configuration/data stores.
The first complete test attempt hit sandbox loopback denial; the suite was rerun
with permission. That denied attempt is not counted as a passing run.

Hosted validation is **blocked**, not passed. The initial workflow had a runner
context error at job environment scope; `e84688f` moves those paths into a step.
GitHub then accepted the matrix, but all three jobs in
[run 34079391248](https://github.com/tsuna-n/libraryCU/actions/runs/34079391248)
were prevented from starting with: “The job was not started because your account
is locked due to a billing issue.” No macOS/Windows test or hosted artifact result
exists yet. After the account owner resolves billing, rerun the latest workflow
and inspect all three jobs before tagging/publishing a release.

The feature branch is `feat/0.4.0-verify-rollback-ci`. Main, release tags, and the
previously installed binary have not been changed by this task.

The final workflow also passes local `actionlint` 1.7.12. Existing scanner/CLI
path assertions compare canonical/native paths so they do not depend on Unix
path spelling. This review does not substitute for executing the Windows/macOS
matrix once hosted runners are available.

## Remaining limits

- Passing verification proves only that the selected command exited zero for
  the observed patched snapshot. It is not a general correctness certificate.
- Verification runs trusted user commands with inherited privileges/environment;
  it may access the network or change other files. Only the selected source file
  can be rolled back. Timeout stops the direct child, not every possible detached
  descendant; timeout output is discarded.
- Recovery records contain source snapshots, are not authenticated against local
  tampering, and have no automatic retention cleanup. Keep `.lbc/fixes/` private
  and out of version control. Atomic replacement/rechecks do not prevent all
  concurrent writers or hostile directory races, or guarantee power-loss durability.
- Pattern redaction and partial `.gitignore` support retain their prior limits.
  Complex Python ExceptionGroups, arbitrary log prefixes, full Node stack/source
  map resolution, and complete non-Rust rule coverage remain unsupported.
- No live AI provider quality evaluation or public release publication is implied.
  Historical audits remain relevant for unchanged subsystems.
