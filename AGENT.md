# AGENT.md — libraryCube Engineering Contract

This file defines how coding agents and contributors must work on **libraryCube** (`lbc`).
The objective is not only to make changes compile; every change must preserve the project's offline-first behavior, privacy boundaries, deterministic retrieval, honest verification language, CLI compatibility, and reproducible tests.

If an intentional product change makes this file stale, update the implementation, tests, README/docs, and this file together.

## Start here — roadmap execution and Sol handoff

Read [ROADMAP.md](ROADMAP.md) before selecting implementation work. It defines
the product direction, milestone order, backlog, and release gates. This file
defines how to implement and verify that work. The implementation and tests are
the evidence of what exists today; roadmap examples are planned behavior until
implemented and validated. Explicit user instructions determine the current task
scope; a request to edit documentation alone does not start roadmap implementation.

When asked to continue development (including when Sol takes over):

1. Read this file, `ROADMAP.md`, `Cargo.toml`, and `git status --short`. Preserve
   existing changes, including untracked roadmap or handoff files.
2. Read `docs/roadmap-progress.md` if present, then the relevant implementation,
   tests, README, and readiness audit. Recheck saved progress against the current
   worktree; historical test results do not validate new changes.
3. Select the earliest incomplete milestone and a small, dependency-ready group
   of unchecked items. Compare the checklist with code before choosing work:
   an unchecked item may be partially implemented, and a checked item may still
   have regressions. Map baseline Known Gaps to their owning milestone instead
   of implementing every future enterprise feature as baseline cleanup.
4. State the milestone, exact checklist items, intended behavior, and validation
   before editing. Implement the selected work through tests and documentation;
   a plan or scaffold alone is not completion. Continue within the user's
   authorized scope without requesting confirmation for routine local choices.
5. Update the affected roadmap checkboxes only when supported by implementation
   and applicable validation. Leave partial or blocked items unchecked and record
   what remains. Do not mark an entire section complete from one successful test.
6. Update `docs/roadmap-progress.md` with the handoff described below. If it does
   not exist, create it when doing roadmap implementation. Do not stop merely
   to write a handoff while authorized, actionable work remains in scope.

### Milestone order

| Milestone | Work to prioritize |
|---|---|
| `v0.4.0` | Preserve the core CLI baseline and verify its known gaps |
| `v0.5.0` | Security, supply chain, project isolation, and core hardening |
| `v0.6.0` | Organization/team knowledge, signed packages, lifecycle, and quality |
| `v0.7.0` | Policy enforcement, audit, and verification sandbox |
| `v0.8.0` | Identity, SSO/RBAC, enterprise server, devices, and updates |
| `v0.9.0` | Pilot, benchmarks, reliability, recovery, and production hardening |
| `v1.0.0` | Stable contracts, secure operations, and all enterprise release gates |

At this handoff, `Cargo.toml` reports `0.5.0`; the active milestone remains
`v0.5.0` until its hosted repository and signed-release gates are verified.
Re-evaluate this checkpoint on later runs. For the next implementation session,
inspect `docs/roadmap-progress.md`, dependency checks, `.circleci/config.yml`,
installers, security docs, and core-hardening coverage. Continue an unchecked
local hardening item or resolve new CI evidence. Repository administration or
signing credentials must not block other independent work within that milestone.

Follow the roadmap's priority: security → supply chain → project isolation →
organization knowledge → policy → audit → RBAC/SSO → pilot → v1.0.
Keep GUI/TUI redesigns, editor integrations, autonomous agents, vector services,
marketplaces, and other explicitly deferred features outside the v1.0 work.
Do not advance to a later milestone by silently skipping an earlier release gate;
record the blocker and continue independent work in the current milestone.

### Scope, external dependencies, and release evidence

- Roadmap work may evolve current behavior through a documented design, tests,
  and compatibility handling. Preserve offline, privacy, and explicit-action
  invariants while doing so; do not present future guarantees as current ones.
- A roadmap checkbox does not itself authorize changing hosted repository rules,
  provisioning services, publishing releases, or using signing identities. Use
  existing session authorization where available. Prepare concrete local config,
  scripts, docs, and validation before requesting any missing external authority.
- For repository rules, signing, SSO, pilot deployments, and other external work,
  distinguish locally prepared artifacts from externally configured and verified
  behavior. Keep unfinished requirements and their release gates open.
- Apply the baseline checks in section 12 plus the current milestone's release
  gate. For dependency-security work, run `cargo audit` and `cargo deny check`
  once tooling/configuration is introduced; record missing tools, database/network
  failures, and findings accurately. Do not suppress findings just to pass CI.
- Linux checks cannot prove macOS/Windows success. Record CI evidence against
  the tested revision. Checksums do not prove signatures; a signing workflow
  file does not prove artifacts were signed; a sandbox plan does not prove isolation.
- Do not bump the version, create a release tag, or declare a milestone released
  merely because implementation started or local tests passed. Release work must
  satisfy the roadmap gate and the user's authorized release scope.

### Persistent handoff

Keep `docs/roadmap-progress.md` concise and current, with:

- Checkpoint date, branch/revision, and relevant uncommitted work.
- Active milestone and exact roadmap section/checklist items in scope.
- Completed behavior with implementation/test paths and corresponding checkbox updates.
- Validation commands and results, including failed or unrun checks and CI evidence.
- Partial work, remaining release gates, and concrete external blockers.
- The next actionable task, its dependencies, affected files, and acceptance checks.

Use `ROADMAP.md` as the canonical checklist; the handoff adds execution evidence
and continuation context rather than maintaining a competing backlog. Never put
credentials, signing material, or sensitive project content in either document.

---

## 1. Product identity

libraryCube is a Rust terminal knowledge library and troubleshooting assistant.
Its roadmap target is a **Local-first Enterprise Developer Knowledge &
Troubleshooting Platform**. Enterprise capabilities become current product claims
only after their milestone work is implemented and validated.

```text
question / diagnostic
        ↓
bounded local project evidence
        ↓
local knowledge retrieval
        ↓
deterministic offline answer
        ↓
optional AI enhancement only when --ai is explicit
```

- Rust package/crate: `librarycube`
- Executable: `lbc`
- Current version must be read from `Cargo.toml`; do not duplicate it as a constant unless required.

libraryCube is:
- a local Markdown knowledge library,
- an offline retrieval and troubleshooting tool,
- a lightweight project scanner,
- an optional bridge to local/remote OpenAI-compatible AI providers,
- a privacy-conscious developer-memory tool.

libraryCube is **not** automatically:
- an autonomous coding agent,
- a shell execution framework,
- a mandatory cloud/vector service,
- a background daemon,
- a repository-wide automatic repair system.

Do not expand into those roles without an explicit design change, tests, and documentation.

---

## 2. Non-negotiable invariants

### 2.1 Offline-first is a first-class mode

`search`, `ask`, `explain`, `chat`, `scan`, `inspect`, `list`, `index`, `doctor`, and deterministic analysis must remain useful without an API key, Internet access, model server, Python service, or vector database.

### 2.2 Network access is explicit

No AI provider call may occur unless the user explicitly requests AI behavior, currently with `--ai`.

Never use a remote provider as an implicit fallback for weak retrieval.
Tests must continue to prove zero provider connections when `--ai` is absent.

### 2.3 Retrieval comes before AI

The local retrieval pipeline is the evidence source. AI may enhance selected evidence; it must not replace retrieval.

`search`, `ask`, `explain`, and `chat` should continue to share `knowledge::retrieval` rather than developing divergent retrieval paths.

### 2.4 Never invent evidence

If evidence is insufficient, report that state.
Do not fabricate source IDs, file locations, dependency states, executed-command results, verification results, or root causes.

### 2.5 Analysis commands do not repair projects

Read-only analysis may suggest commands such as `cargo check`; it must not execute repairs automatically.
Writes are limited to explicit user operations such as knowledge add/edit, package install/remove, config updates, history clearing, `fix --ai --apply`, and `rollback`.

`fix` is an explicit, single-file patch workflow: without `--ai` it provides offline
guidance; `--ai` proposes one replacement; adding `--apply` writes it. It never
executes provider-selected commands. Only explicit `--verify "COMMAND"` with
`--ai --apply` executes a user-selected executable, without an implicit shell.
Verification has a 1–3600 second deadline, closed stdin, bounded/redacted output,
and truthful exit/timeout/error status. These commands inherit the user's
environment and may have side effects; LBC does not sandbox them. Timeout stops
the direct child, not necessarily detached descendants.
The target must be the first diagnostic's
file inside the exact supplied project directory, with no root discovery. Reject
hidden/generated/recognized gitignored targets, symlinks, special files, recognized
secrets, missing or out-of-range locations, ambiguous replacements, and source
changes detected before publication. Provider output cannot select a path.
Patches remain `unverified` unless the explicitly requested check passes and the
applied source is unchanged. `passed` only describes that command's exit status.
Every CLI apply must persist a bounded private recovery record first. Failed
verification attempts rollback; explicit `rollback ID` supports later recovery.
Both restore only when the current source equals the applied snapshot. Preserve
later edits, authenticate version 2 records with the private project recovery key,
enforce the 30-day/100-record retention bound when preparing a record, and report
recovery failures with nonzero exit status and JSON when requested. Version 1
records are legacy unauthenticated data and require manual recovery. Recovery is
single-file and does not undo verification side effects. Structural validation
does not establish semantic correctness. Unix directory-descriptor publication
protects final rename from parent-symlink replacement; portable fallbacks and
content rechecks do not protect against every non-cooperating concurrent writer.

### 2.6 Privacy is correctness

Before data is persisted or sent to an AI provider:
- bound it,
- redact recognizable secrets,
- preserve source/provenance,
- use safe bounded file access,
- respect path/symlink/special-file protections.

Do not weaken redaction, containment, symlink rejection, file limits, prompt limits, HTTP response limits, or timeout behavior without regression tests.

### 2.7 Verification wording must remain honest

`verification_status` is note metadata, not proof that the current command reran a check.
`user-reported` and `recorded-check` must never be presented as fresh verification by libraryCube.
Changed guidance must not inherit stale verification claims.

### 2.8 JSON output is an API surface

For `--json` commands:
- stdout must contain machine-readable JSON only,
- warnings belong on stderr,
- exit codes remain meaningful,
- key/type changes are compatibility-sensitive and require tests.

### 2.9 English and Thai behavior

Supported output modes are `en`, `th`, and `auto`.
`auto` selects Thai when the current input contains Thai characters and English otherwise.
Preserve commands, paths, source IDs, error codes, code, and JSON keys.
Offline mode does not promise translation of arbitrary user notes.

---

## 3. Repository map

```text
src/
├── ai/             provider abstraction, prompt/context, streaming
├── answer.rs       offline answer assembly + optional AI enhancement
├── cli/            Clap arguments, dispatch, command implementations
├── config/         loading, defaults, validation, persistence
├── diagnostics/    parsing, deterministic rules, explanations
├── history.rs      bounded optional persistent chat history
├── fixer.rs        bounded single-file patch validation and explicit application
├── knowledge/      documents, loading, storage, retrieval, index, packages
├── output/         human/JSON output helpers
├── scanner/        project detection, evidence, file/ignore handling
├── security/       redaction and safe bounded file access
├── lib.rs          module surface + app entry
└── main.rs         binary entry

tests/
├── cli.rs          CLI and provider integration regressions
├── cli/fix.rs      fix workflow, Python package, and provider regressions
├── filesystem.rs   filesystem safety regressions
└── privacy.rs      redaction/network/privacy regressions

knowledge/          built-in Markdown knowledge
docs/               usage, readiness audits, specs/plans
.circleci/           Linux/macOS/Windows CI and release publication
install.sh           source and prebuilt Unix/macOS installer
install.ps1          source and prebuilt Windows installer
```

Prefer extending an existing subsystem over introducing a new top-level module without need.

---

## 4. Current CLI contract

Current command families include:

```text
lbc add
lbc list
lbc inspect
lbc edit
lbc index
lbc ask
lbc chat
lbc history ...
lbc scan
lbc explain
lbc fix
lbc rollback
lbc search
lbc config ...
lbc doctor
lbc knowledge ...
```

When adding/changing a command:
1. update `src/cli/args.rs`,
2. keep dispatch explicit,
3. test normal output,
4. test JSON output when supported,
5. test exit/failure behavior,
6. update README and/or `docs/usage-examples.md`.

Do not add flags with no tested behavioral effect.

---

## 5. Agent workflow

For roadmap implementation, apply the startup, milestone selection, and persistent
handoff rules above before following this workflow.

For non-trivial changes:

1. **Inspect first** — read the implementation, adjacent security boundary, existing tests, and relevant docs.
2. **Define the behavior** — identify what changes, what must remain invariant, and which tests prove it.
3. **Implement the smallest coherent change** — avoid unrelated speculative refactors.
4. **Add regression tests with the change** — a bug fix without a regression test is incomplete when testable.
5. **Run targeted tests first**, then the full gate.
6. **Update docs** for user-observable behavior.
7. **Report exactly what was validated** — never guess test results.

Prefer existing abstractions and dependencies. Do not add a crate when std or an existing dependency is adequate.

---

## 6. Rust implementation rules

- Use `anyhow::Result` consistently where the application already does.
- Add useful context at I/O/config/provider boundaries without leaking secrets.
- Avoid `unwrap()` in production paths unless a local invariant makes failure impossible and obvious.
- Keep user-visible limits centralized; when changing a limit, update code, errors, boundary tests, and docs.
- Preserve deterministic ranking/output ordering for identical inputs.
- Do not invoke `sh -c`, `bash -c`, or equivalent for user-controlled strings without a dedicated security design.
- Preserve the existing property that editor arguments are parsed without implicitly launching a shell.

---

## 7. Knowledge and retrieval rules

### 7.1 Source-qualified identity

Current source forms include:

```text
builtin:ID
package:NAME:ID
user:ID
project:ID
```

Do not collapse distinct source IDs into one ambiguous ID.
Duplicate IDs within one source remain validation errors.

### 7.2 Knowledge documents

Supported metadata includes fields such as:

```text
id, title, title_th, language, tool, category,
error_code, tags, keywords, kind, verification_status
```

Documents must remain bounded, valid, and have non-empty bodies.
Do not silently index malformed partial documents.

### 7.3 Current retrieval is lexical/metadata based

Do not describe the current implementation as vector/embedding search.
Existing ranking intentionally uses signals such as:
- exact error-code match,
- title match,
- metadata match,
- title/body term matches,
- term coverage.

If semantic retrieval is introduced later, preserve a deterministic offline baseline and exact error-code semantics. Prefer a tested hybrid model rather than silently deleting current behavior.

### 7.4 Adequacy threshold

`ask` must not treat every non-zero search result as adequate evidence.
Preserve an explicit adequacy/relevance threshold to avoid confident weak-keyword answers.

A Python traceback playbook may appear as explicitly labeled `general_guidance`
when specific evidence is inadequate. This status is an additive JSON value,
not proof of adequacy; it must not authorize AI patch generation. Preserve stronger
exact-code/title matches and avoid substring triggers such as `pip` in `pipeline`.

### 7.5 Thai retrieval

Changes to Thai tokenization/localization require Thai-specific tests for relevant matches, false positives, question-word handling, and bilingual title/body selection.

---

## 8. Scanner and diagnostics

### Scanner

The scanner is a lightweight inventory/evidence collector, not a full semantic code-understanding engine.

Project evidence must remain bounded and contained in the resolved project boundary. Preserve warnings/rejection for unsafe paths, symlinks where prohibited, special files, inaccessible files, and oversized evidence.

Project and nested `.gitignore` files are parsed with a Git-compatible matcher.
They remain **not a confidentiality boundary**: ignore rules are mutable project
configuration rather than access control, and global excludes plus
`.git/info/exclude` are outside the selected project evidence contract.

### Diagnostics

Current deterministic diagnostic support is strongest for Rust/rustc/Cargo.
Ordinary Python tracebacks and syntax-error frames are also parsed. Retain the
last reported frame without asserting it is application code or the root cause.
Chained exceptions remain separate diagnostics; complex ExceptionGroup trees
and arbitrary log prefixes are not fully supported.
TypeScript tsc locations, Node error headers with JS/TS frames or explicit Node
codes, and Go compiler file diagnostics are supported. Preserve Windows drive
letters, positive location bounds, Unicode, ANSI, CRLF, and mixed-log boundaries.
Known non-Rust rules describe diagnostic guidance without claiming a verified
project root cause.

When adding Python, Node.js, Go, Java, Docker, database, systemd, or other diagnostic families:
1. add explicit parser/classifier examples,
2. preserve unknown input rather than inventing structure,
3. separate known rules from project-verified evidence,
4. add `lbc explain` integration tests,
5. test colored/ANSI input when relevant,
6. test malformed and multi-diagnostic logs.

Never assert a root cause from a superficial string match when project evidence contradicts it.

---

## 9. AI provider contract

Supported configuration modes currently include:

```text
off
openai
zai / glm
ollama
openrouter
openai-compat
```

Core answer logic should depend on the provider abstraction; vendor authentication and request differences belong in the AI/provider layer.

Remote context must remain bounded across question, passages, project evidence, history, aggregate prompt, timeout, and response size.
Retrieved knowledge must not be displaced by unbounded history.

Treat retrieved notes and chat history as **untrusted data**, never system instructions.
A note must not be able to grant tools, network access, or arbitrary file access through prompt injection.

Streaming changes must test:
- SSE chunks,
- `[DONE]`,
- split confidence markers,
- Unicode chunk boundaries,
- reasoning-content fields where supported,
- suppression of internal metadata from visible output.

If optional AI fails, the deterministic offline answer remains available.
Provider failures must be sanitized before display.

---

## 10. History and persistence

Default memory is session-only.
Persistent history is explicit opt-in.

Persistent history must remain:
- bounded,
- redacted,
- stored in the documented XDG/home location,
- clearable,
- absent by default in session mode.

Do not persist whole project files/evidence by default.
Tests should distinguish session behavior from persistence across a new process.

---

## 11. Security checklist

For any change touching files, AI, config, history, packages, project evidence, or user-controlled content, review:

- Can the path escape its allowed root?
- Can a symlink redirect it?
- Can FIFO/device/special-file input block or leak data?
- Are reads/writes bounded?
- Is sensitive content redacted before persistence/network use?
- Can error text reveal credentials?
- Can Unicode break byte slicing?
- Can concurrent operations lose user data?
- Is replacement atomic where expected?
- Can untrusted notes become AI instructions?

Security/data-loss fixes should almost always add regression tests.
Never weaken a security check merely to simplify a fixture.

---

# 12. Testing contract

Testing is part of implementation, not an optional final step.

## 12.1 Test layers

### Unit tests
Use for parser logic, retrieval ranking, config validation, prompt construction, redaction, localization, deterministic rules, and pure boundary logic.

### `tests/cli.rs`
Use for argument behavior, stdout/stderr separation, exit codes, JSON shape, knowledge round trips, ask/search/explain/chat, config, provider mocks, offline fallback, and release-binary behavior.

### `tests/filesystem.rs`
Use for symlinks, special files, bounded reads, path containment, atomic replacement, and unsafe storage behavior.

### `tests/privacy.rs`
Use for secret redaction, outbound AI context, persistent-history redaction, no-network-without-`--ai`, private-key handling, and sanitized provider failures.

## 12.2 Required baseline gate for code changes

Before declaring a code change complete, run:

```bash
cargo fmt --check
cargo check --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
git diff --check
```

If a command cannot run, report it as **not run** with the exact blocker. Never convert a blocked regression into a skipped success.

## 12.3 Targeted iteration

Examples:

```bash
cargo test knowledge::
cargo test diagnostics::
cargo test ai::
cargo test --test cli
cargo test --test filesystem
cargo test --test privacy

cargo test exact_error_code_dominates_and_is_reported
cargo test parses_rust_error_code_and_location
```

Targeted tests speed iteration but do not replace the full gate.

## 12.4 Release-binary regression

For CLI/install/release wiring changes:

```bash
cargo build --locked --release
LBC_TEST_BINARY="$PWD/target/release/lbc" \
  cargo test --locked --test cli
```

Use isolated fixtures rather than the real user environment.

## 12.5 Provider testing

Do not require paid/live credentials for normal tests. Use a loopback mock provider.
Test:
- request shape,
- auth handling without secret leakage,
- redacted context,
- timeout,
- malformed JSON,
- oversized responses,
- provider errors,
- streaming,
- offline fallback.

If loopback is denied by the execution environment, report the environmental blocker; do not silently skip.

## 12.6 Test isolation

Tests must not depend on or modify the developer's real:

```text
~/.config/lbc
~/.local/share/lbc
knowledge packages
chat history
provider configuration
```

Use temporary directories and isolated `XDG_*` / `LBC_CONFIG` values as appropriate.
Do not depend on test execution order.

## 12.7 Boundary values

For every new limit/parser boundary, test:

```text
limit - 1
limit
limit + 1
```

Include Unicode/multibyte tests where byte-vs-character boundaries matter.

## 12.8 Bug-fix rule

For a reproducible bug:
1. add a test that demonstrates it,
2. confirm the test exercises the broken path,
3. fix the implementation,
4. keep the regression test.

## 12.9 Documentation-only changes

At minimum run:

```bash
git diff --check
```

Also compare documented commands/settings against current `src/cli/args.rs`, `src/config/settings.rs`, README, and implementation. Do not claim code tests passed if they were not run.

---

## 13. Change-specific test matrix

| Changed area | Minimum targeted validation |
|---|---|
| CLI args/dispatch | CLI integration + help/error paths |
| JSON output | JSON assertions + exit-code assertions |
| Knowledge parsing/storage | unit + CLI round trip + filesystem tests for writes |
| Retrieval/ranking | ranking unit tests + `search` + `ask` |
| Thai/localization | Thai unit tests + CLI behavior |
| Scanner/evidence | scanner unit + containment/filesystem tests |
| Diagnostics | parser/rule unit + `lbc explain` integration |
| Config | config unit + config/doctor CLI tests |
| History | history + privacy + cross-process persistence |
| AI provider | provider unit + mock HTTP CLI + privacy |
| Streaming | SSE parser + split-chunk + CLI streaming tests |
| Security/redaction | privacy suite + affected CLI path |
| Packages | package tests + CLI + partial/failure writes |
| Installer/release | release build + release-binary CLI regression |
| Fix preview/apply | CLI/mock provider + no-network offline + bounded/contained target + stale-source/failed-write preservation |
| Verify/rollback | Portable child-process fixtures + success/failure/timeout/spawn error + output bounds/redaction + recovery across invocations + edit conflict and unsafe-storage rejection |

This matrix is a floor, not a ceiling.

---

## 14. Known readiness limitations

Do not document these as solved until implementation and regression tests prove otherwise:

- scanner readability/evidence edge cases,
- non-cooperating writers, shared multi-user stores, and cross-session history merging,
- transactional duplicate-ID behavior across legacy and current project stores,
- durability beyond atomic rename,
- platform-complete ownership and access validation for shared mutable stores,
- complete Thai diagnostic localization,
- broader malformed/oversized provider cases,
- pattern-based redaction cannot detect every secret,
- broad non-Rust diagnostic coverage.

Use `docs/readiness-audit.md` as the audited checkpoint. Prefer adding a new dated audit after substantial hardening rather than rewriting historical evidence.

---

## 15. Forbidden regressions

Reject implementations that:
- call AI without explicit `--ai`,
- send an entire repository to a provider,
- persist project/chat context by default,
- execute repair commands automatically,
- use `.gitignore` as the only privacy control,
- make vector/cloud infrastructure mandatory,
- fabricate AI/offline answers when provider/retrieval fails,
- silently swallow invalid knowledge,
- bypass current symlink/safe-reader protections,
- read unbounded files/HTTP responses,
- contaminate JSON stdout with warnings,
- treat author-recorded verification as fresh verification,
- weaken tests to preserve broken behavior,
- depend on personal credentials/configuration.

---

## 16. Definition of Done

A change is complete only when all applicable items are true:

- [ ] Requested behavior is implemented.
- [ ] Offline-first behavior remains intact.
- [ ] No implicit network access was added.
- [ ] Trust-boundary data is bounded/redacted.
- [ ] Filesystem containment/symlink rules are preserved or improved.
- [ ] AI failure does not break deterministic local behavior.
- [ ] New behavior has regression tests.
- [ ] JSON compatibility is tested when affected.
- [ ] Thai/English behavior is tested when affected.
- [ ] Failure paths are tested.
- [ ] `cargo fmt --check` passes for code changes.
- [ ] `cargo check --locked` passes for code changes.
- [ ] Clippy passes with `-D warnings` for code changes.
- [ ] `cargo test --locked` passes for code changes.
- [ ] Release build passes for code changes.
- [ ] `git diff --check` passes.
- [ ] User-facing docs are updated when needed.
- [ ] Roadmap implementation maps to specific milestone items; only proven items are checked off.
- [ ] `docs/roadmap-progress.md` records evidence, blockers, and the next task for roadmap work.
- [ ] Milestone completion claims satisfy the applicable roadmap release gate.
- [ ] Final report says exactly what was and was not validated.

---

## 17. Commit/review guidance

Prefer focused conventional commits, for example:

```text
feat(ai): stream provider responses
fix(security): reject symlinked history paths
test(cli): cover offline provider fallback
docs: update troubleshooting workflow
```

Review priority:
1. privacy/security,
2. data loss/corruption,
3. accidental network access,
4. false evidence/verification claims,
5. CLI/JSON compatibility,
6. retrieval correctness,
7. test isolation/reliability,
8. performance,
9. style.

---

## 18. Final agent report

When finishing a repository task, report:

```text
Changed
- concrete files and behavior

Why
- problem solved and important design choice

Validated
- exact tests/checks that passed

Not validated
- blocked or intentionally unrun checks

Risks / follow-up
- only material remaining limitations

Roadmap handoff (when implementing roadmap work)
- milestone and checklist items completed or still open
- remaining release gates and external blockers
- next actionable task and docs/roadmap-progress.md location
```

Never guess test results and never call the project production-ready solely because a local suite passed.

**Standard: small, evidence-backed, privacy-preserving changes with reproducible tests.**
