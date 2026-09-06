# 0.3.4 release audit — 2026-09-06

Status: implemented and locally validated on Linux. The release binary is
`target/release/lbc`; it reports `lbc 0.3.4`. The same binary is now installed at
`/home/tsuna/.local/bin/lbc` and resolves through PATH. It has not been published
remotely. This is a release checkpoint, not production certification.

## Behavior

- Cargo.toml and Cargo.lock agree on 0.3.4. Clap uses Cargo's version metadata;
  the CLI regression checks top-level and propagated `ask --version` output.
- `fix` supplies offline guidance. `fix --ai` proposes one exact replacement
  using retrieved knowledge and a bounded diagnostic-file excerpt.
  `fix --ai --apply` explicitly publishes the replacement. A provider cannot
  choose paths or invoke tools. Every invocation generates a new proposal.
- Application checks unique before text, diagnostic-line intersection, byte
  limits, recognizable secrets (including across replacement boundaries),
  containment, symlinks, regular files, and detected source changes. Publication
  is atomic and preserves file permissions. No verification commands run;
  both proposals and applied changes remain `unverified`.
- Ordinary Python tracebacks, basic syntax-error frames, chained exceptions,
  ANSI input, and mixed Rust/Python logs are parsed. Unknown/malformed locations
  are not invented. The optional Python/FastAPI package contains 40 bilingual
  Markdown notes and is installed explicitly from the source checkout.
- Weak Python matches can produce explicitly labeled `general_guidance` in
  `ask`, with source citations and an insufficient-evidence statement. That
  fallback cannot authorize a patch. Exact-code matches keep their priority,
  and Python fallback detection does not mistake `pipeline` for `pip`.
- README, usage examples, and AGENT.md describe the new explicit write operation,
  limits, JSON status values, English/Thai behavior, and failure semantics.

## Validation

All required commands passed on the final code:

```bash
cargo fmt --check
cargo check --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
git diff --check
LBC_TEST_BINARY="$PWD/target/release/lbc" cargo test --locked --test cli
```

`cargo test --locked`: **127 passed**, zero failed or ignored (70 library,
45 CLI, 6 filesystem, 6 privacy). The additional release-binary run passed all
**45 CLI tests**, including the real 45-second timeout. Release `--version` and
`fix --help` were also inspected directly.

Targeted iteration passed the fixer library tests and `cargo test --locked
--test cli fix::` (9 CLI tests). New mock tests cover offline zero connections,
preview versus apply, English/Thai and stdin behavior, JSON and exit codes,
provider/validation failures with offline guidance, redacted requests,
source changes during a request, exact project scope, and no execution of
generated code. The Python package is actually installed and used by
search/ask/explain/fix fixtures. Filesystem tests cover symlinks, FIFOs, stale
sources, permission preservation, and no temporary-file residue. Byte-limit
tests exercise limit minus one, limit, and limit plus one, including Unicode.

Initial mock tests hit the sandbox's loopback denial; they were rerun with
permission and passed. An initial scanner fixture also lacked its own root
marker; the fixture now supplies one to avoid depending on ancestor directories.
All new fixtures use temporary projects and isolated configuration/data stores.
No live provider credentials or remote service were used.

## Installed-binary follow-up

The existing PATH binary was 0.3.3-d. Installation was updated using the already
validated release build:

```bash
cargo build --locked --release
./install.sh --no-build --prefix /home/tsuna/.local/bin
lbc --version
sha256sum target/release/lbc /home/tsuna/.local/bin/lbc
LBC_TEST_BINARY=/home/tsuna/.local/bin/lbc cargo test --locked --test cli
git diff --check
```

Build and installation passed. `lbc --version` reports `lbc 0.3.4`, and the two
binaries have identical SHA-256 digests. All **45 installed-binary CLI tests**
passed with zero failures or ignored tests, including the real 45-second timeout.
Tests used isolated projects/configuration/data and loopback mock providers.
`lbc fix --help` exposes the documented options.

The existing user store already contained `python-fastapi-basics` 0.2.0 with
40 documents, as confirmed by `lbc knowledge list`; no package replacement was
needed. Configuration, notes, and chat history were not changed by this update.
The [changelog](../CHANGELOG.md) records the completed version changes. No remote
publication was performed.

## Remaining limits

- Structural patch validation does not prove semantic correctness. Review
  generated changes and run the relevant project checks yourself.
- Source rechecks and atomic replacement do not serialize every concurrent
  writer, defend against all hostile directory races, or guarantee power-loss
  durability. Extended file metadata preservation is not tested.
- `.gitignore` handling remains a subset of Git semantics; pattern-based
  redaction cannot detect every secret. Fix rejects a complete source snapshot
  if recognizable secrets appear anywhere in it.
- Fix handles one replacement in the first diagnostic's file. Complex
  ExceptionGroup trees and arbitrary log prefixes are not fully parsed.
  Existing general scanner-root discovery and localization limits remain.
- Live-provider quality, non-Linux platforms, and remote publication were not
  validated or performed.

The historical [readiness audit](readiness-audit.md) remains applicable to the
unchanged subsystems and their documented limitations.
