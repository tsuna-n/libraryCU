# Installation and first-run setup

Implement the user's three onboarding tasks on the existing 0.5.0 worktree.
Do not push, tag or publish. Preserve the earlier release fixes and security gates.

## Contracts and ownership

1. Installer agent owns `install.sh`, `install.ps1`, `tests/install.rs`.
   Add explicit local-archive plus trusted SHA-256 mode; verify before extraction
   or prefix changes. Existing source/bundled modes remain compatible. Binary
   installation needs neither Rust nor Python. Uninstall preserves knowledge and
   configuration. Test bad hashes, unsafe archives, PATH and actual binary cycles.
2. Setup agent owns CLI/config modules and `tests/cli/setup.rs` (parent test module
   registration only). Add `setup`, `--yes`, `--project`, `--provider`, `--model`,
   `--base-url`, and `--json`. Default new configuration to offline, preserve
   existing TOML fields, use private locked atomic writes, validate/reload, and
   initialize established project/personal knowledge directories. No key prompts,
   key persistence, or provider connections. Test cancellation, no-mutation errors,
   repeated runs, secret rejection and offline knowledge use.
3. Documentation agent owns README and installation/quick-start/reference docs.
   Lead with the three real CLI commands, preserve detailed existing documentation
   under docs, document Windows/Linux/macOS and the current missing-assets limit.
4. Root owns integration, independent reviews, full validation and final report.
   No concurrent file edits; review handoffs precede integration edits.

## Verification

- Focused regressions, then fmt/check/strict Clippy/full locked tests.
- Release build, CLI tests against it, real verified Linux archive install/setup/
  capture/search/explain/uninstall, and unchanged fixed retrieval baseline.
- Existing security/release fixtures and dependency checks where affected.
- Report native Windows/macOS execution as unverified unless actually run.
- Record final results in `docs/onboarding-validation.md`; do not infer production
  readiness from local tests or replace the earlier dated release receipt.
