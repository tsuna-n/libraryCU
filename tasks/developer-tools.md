# Developer tools integration

Scope: implement the three requested capabilities while retaining local retrieval,
optional AI/MCP, and the existing uncommitted terminal work.

| Capability | Contract | Depends on |
| --- | --- | --- |
| project-memory | Explicit project root, project-only retrieval, stack ranking, historical evidence labels | Existing loader/retrieval |
| editor-integration | Process arguments or stdin, explicit workspace, read-only explain | Existing CLI; additive `explain --text` |
| mcp-server | Optional `lbc mcp --project PATH`, stdio JSON-RPC, three read-only tools | Project-memory APIs |

Build order: project-memory and editor-integration in parallel; MCP protocol and CLI
adapters alongside them; integrate, review, and run repository checks.

- [x] Project memory: test separate roots, shared-store exclusion, stack ranking,
  symlink boundaries, and historical verification semantics.
- [x] Editors: provide small VS Code tasks and Neovim Lua examples; test selected
  text and logs, including shell-looking input without command execution.
- [x] MCP: implement initialize/ping/tools/list/tools/call, bounded messages and
  arguments, fixed root, no mutation/AI/shell tools; test real stdio processes.
- [x] Document Codex CLI/OpenCode configuration and actual editor workflows.
- [x] Run focused tests, full `cargo test --locked`, `cargo fmt --check`,
  `cargo clippy --locked --all-targets --all-features -- -D warnings`, and build.

Boundaries: preserve existing APIs with wrappers and additive flags; no new
dependencies, no network in the MCP server, no writes to source or knowledge,
no parent-directory project discovery, no claim that historical checks verify
the present error. Treat knowledge text as untrusted data for consuming agents.

Workspace validation (2026-11-12, including the uncommitted terminal work):
all 391 tests passed, including 17 new integration tests
and a bounded-frame MCP unit test. `cargo fmt --check`, `git diff --check`,
`cargo clippy --locked --all-targets --all-features -- -D warnings`, and
`cargo build --locked --release` passed. Real stdio subprocesses and Neovim
0.12.6 headless were exercised; VS Code tasks were run as process arguments.
Independent review found no remaining issues after the shared-store fixture
and MCP validation-error classification fixes.

Before committing, the staged snapshot was exported and checked separately
without the unrelated TUI/completion changes: all 378 tests, formatting and
Clippy passed. The exported test repository used the canonical origin so the
release-provenance tests could validate repository metadata.

One sandboxed full-suite attempt refused the sandbox's `/tmp` owner; the final
full suite passed with the existing approved Cargo test permission, preserving
all owner/security checks. No code was weakened to accommodate the sandbox.

Not exercised: live model sessions in Codex/OpenCode, the VS Code GUI, native
Windows/macOS, or filesystem-race/performance stress. No client configuration
was installed and no remote publication was performed.
