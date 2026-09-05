# Readiness audit — 2026-09-05

This is a hardening checkpoint, not certification of the entire implementation
contract or every operating system. The existing completion checklist must not
be read as evidence that all production edge cases were tested.

## Fixes covered by regressions

- UTF-8-safe redaction of multiple structured credentials, URI user information,
  provider content/errors, and private-key material selected from inside a block.
- Bounded prompt fields and aggregate context; large histories cannot displace
  retrieved knowledge. Empty/oversized queries fail with actionable errors.
- Bounded regular-file reads, FIFO rejection, parent-symlink checks before store
  creation, and no knowledge loading from a symlinked project store.
- Duplicate IDs detected even in renamed files, and serialized note-size
  validation before publication.
- Atomic note publication/replacement with temporary-file cleanup; quoted editor
  arguments; reuse of the current override body; preserved retrieval metadata;
  changed guidance no longer inherits an earlier verification status.
- Unhealthy doctor reports return a failure exit code while preserving JSON.
- Actual loopback tests: knowledge and redaction in outgoing requests, zero
  connections without `--ai`, and offline fallback after the real 45-second timeout.
- Persistent chat history survives a new process, remains redacted, and can be
  explicitly cleared. Default chat does not create history files.

## Validation performed

On Linux with the workspace Rust toolchain:

- `cargo fmt --check`: passed.
- `cargo check --locked`: passed.
- `cargo clippy --locked --all-targets --all-features -- -D warnings`: passed.
- `cargo test --locked`: 89 passed (44 library, 33 CLI, 6 filesystem, 6 privacy),
  zero failed or ignored; includes the real 45-second timeout.
- `cargo build --locked --release`: passed.
- Release binary: 32 CLI regressions passed via `LBC_TEST_BINARY`, including the
  add/list/inspect/ask/edit walkthrough, builtin E0308 pipeline, mock provider,
  and offline privacy checks. Isolated command fixtures run in temporary working
  directories outside the checkout. The timeout case was filtered from this
  additional release run; it passed in the complete debug test run above.
- `git diff --check`: passed.

The release mock tests initially hit the sandbox's loopback denial. They were
rerun with permission and passed; the denial was not treated as a skipped success.
No live AI credentials or remote provider were used for validation.

## Remaining work before broad production use

- Project evidence implements only a subset of root `.gitignore` syntax. Nested
  rules, complete Git semantics, and consistent inventory/detector exclusions
  need a shared implementation and regression tests. Do not treat the current
  ignore matcher as a confidentiality boundary.
- Root discovery may ascend from a supplied project path; strict explicit-project
  scope still needs separation from automatic discovery.
- Detector content counts are inferred from manifest presence rather than a
  record of successful reads. Missing/inaccessible context needs more coverage.
- Atomic file replacement is not concurrency control or a power-loss durability
  guarantee. Concurrent note edits can lose updates, and duplicate-ID detection
  is not transactional. Parent-directory symlink checks are not a race-proof
  defense against another process actively replacing directories.
- Package installation and configuration/history writes need broader failure
  injection and transaction tests. A package install can leave a partial target
  on an I/O failure. Multi-user/shared writable stores are not validated.
- Override cycles, self-overrides, and competing same-priority overrides need
  deterministic validation. Author-declared `recorded-check` is not independently
  validated evidence.
- Thai note retrieval/answers work with matching Thai text, but offline diagnostic
  bodies are not fully localized. English source passages are not translated by
  offline retrieval.
- The CLI fixtures isolate user stores; some library-level explanation tests still
  resolve environment-derived stores. Finish dependency injection before claiming
  every test is independent of the developer's environment.
- Add multi-turn mock-AI history assertions and broader malformed/oversized HTTP
  response cases. Pattern-based redaction cannot recognize every possible secret.

Use controlled, single-user local stores and review selected content before remote
AI. These limitations remain open; no claim of full production readiness is made.
