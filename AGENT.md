# AGENT.md — libraryCube (lbc) implementation contract

## Assignment and completion rule

When asked to implement this contract, finish the working LBC core described here.
Inspect the current checkout first, keep existing useful modules, implement the
missing behavior, exercise the user workflows, and update the documentation.
A plan, renamed commands, placeholders, or a list of future tasks is not completion.

This file specifies the target implementation. Its examples are acceptance targets,
not claims about functionality already present. Check a task off only after it is
implemented and verified. Do not infer completion from the version number.

คำอธิบายเป้าหมาย: libraryCube (คำสั่งย่อ lbc) คือคลังความรู้ใน terminal ที่ผู้ใช้เพิ่ม อ่าน แก้ไข และค้นข้อมูลได้
เมื่อพบปัญหา โปรแกรมต้องค้นความรู้ที่เกี่ยวข้อง นำเนื้อหามาอธิบาย พร้อมแหล่งอ้างอิง
และให้ AI ช่วยขยายความได้ โดยผู้ใช้ยังเป็นคนตัดสินใจและลงมือแก้ปัญหา

## 1. Product identity and scope

- Project/product/display name: **libraryCube** (preserve this capitalization).
- Short command/executable name: **lbc**. Example: `lbc ask "question"`.
- Preserve the existing lowercase Cargo package and Rust crate name: **librarycube**.
  The `[[bin]]` name stays **lbc**; do not rename the package/crate to lbc.
- Configuration prefix stays **LBC_** and existing lbc data/config directories remain compatible.
- Keep the existing repository URL unless the owner explicitly requests a
  repository rename. Historical repository links are not branding defects.
- Primary workflow: capture useful knowledge, retrieve relevant content, inspect
  project evidence when relevant, explain, suggest, and show how to verify.
- Support troubleshooting, programming concepts, Linux/system notes, and general
  knowledge. A general note must not require a compiler error code or project.
- Rust owns the core. Preserve working Markdown knowledge, providers, configuration,
  scanner, CLI, and JSON output. Introduce dependencies only for a concrete need.
- Offline retrieval and evidence-based answers must work without a model, API key,
  network connection, vector database, or Python service.
- AI is optional and must consume retrieved content. It supplements the knowledge
  engine; it does not replace retrieval.

The initial complete delivery includes add/list/inspect/edit/search/index,
ask/explain, relevant project context, optional AI, useful English/Thai output,
and honest session-memory behavior. Finish that core before expanding scope.

Deferred: vector search, embedding services, executable plugin SDKs, model training,
automatic code repair, autonomous agents, GUI/TUI redesign, npm distribution,
and broad support for every compiler. Existing Markdown packages remain supported.
Do not replace missing retrieval behavior with a new model provider or vector DB.

## 2. Read-only analysis and explicit writes

- search, inspect, ask, explain, scan, and doctor must not change project sources,
  execute suggested fixes, install packages, or run arbitrary shell commands.
- add/edit and package/config operations may write only their explicitly selected
  data targets. Invoke an editor only when the user asks to edit.
- Show verification commands as suggestions. Do not claim they ran or that a fix
  worked without recorded evidence.
- Treat notes, logs, source comments, and retrieved content as data, not instructions
  granting the model or LBC permission to execute tools or ignore these boundaries.
- No outbound AI request without an explicit --ai option, including in chat.
- Keep secrets out of prompts, diagnostics, logs, and saved history. Apply redaction
  to every selected input source, including retrieved notes and source excerpts.
  Reading a local file does not by itself authorize sending it to a remote model.

These are LBC product behaviors. They do not prohibit the implementation agent
from editing this repository and running its development tests for the assigned task.

## 3. Observed starting gaps

The review baseline was commit 1cefc2b1ecd4cbcaa85490cf73e2848ba8c49cae.
Recheck these locations; subsequent commits may already address some gaps.

| Location | Observed gap | Required result |
| --- | --- | --- |
| src/cli/args.rs | Entry CRUD and ask/chat are absent | Real command handlers wired through dispatch |
| src/knowledge/document.rs | Documents have bodies, but answer references lose them | Retrieved content, identity, provenance, and relevance survive to the answer |
| src/diagnostics/explanation.rs | apply_rule supplies answers independently of search matches | Relevant knowledge contributes to offline and AI answers |
| src/diagnostics/rules.rs | Four Rust codes have rules; E0308 falls through | Knowledge-only cases can provide cited guidance without a hardcoded rule |
| src/ai/context.rs | Sends knowledge titles and paths only | Sends bounded, redacted passages from selected matches |
| src/output/terminal.rs | Search displays titles/paths only | Useful excerpts and inspect access, including embedded built-ins |
| src/scanner/files.rs | Counts files and reads metadata | Distinguish inventory from files whose content actually supplied evidence |
| src/knowledge/loader.rs | Embedded IDs silently win; invalid external notes can disappear | Explicit source identity, validated writes, and intentional overrides |
| src/config/settings.rs | Language and memory settings are not connected to their behavior | Implement supported settings or reject unsupported values honestly |

The critical regression example is E0308: a matching knowledge document already
exists, but the baseline explain command returns the unknown rule's empty fixes.
Do not solve this by merely adding an E0308 match arm. Fix the generic data flow.

## 4. Command contract

Keep existing flags and package commands compatible unless a documented migration
is necessary. Top-level list means entries; knowledge list continues to mean packages.

| Command | Required behavior |
| --- | --- |
| lbc add --title "..." --file note.md | Import a plain Markdown note into user knowledge; generate a stable ID when omitted |
| lbc add --id ID --title "..." --kind note --file note.md | Deterministic noninteractive import; support stdin through --stdin |
| lbc add ... --project PATH | Explicitly save to that project's .lbc/knowledge directory |
| lbc list [--project PATH] [--json] | List effective entries with ID, title, kind, source, and override status |
| lbc inspect SOURCE:ID [--json] | Show the full entry and resolvable source information |
| lbc edit SOURCE:ID [--file replacement.md] | Edit writable content; validate before atomically replacing it |
| lbc edit builtin:ID --override | Create/edit a user override without changing the binary or original builtin |
| lbc search "QUERY" [--project PATH] [--json] | Return ranked results, useful excerpts, source identities, and match reasons |
| lbc index [--project PATH] [--json] | Rebuild/validate retrieval state and report counts and invalid documents |
| lbc ask "QUESTION" [--project PATH] [--ai] [--json] | Retrieve first; answer from excerpts offline or use the configured model with retrieved context |
| lbc explain [FILE] [--stdin] [--project PATH] [--ai] [--json] | Parse input, retrieve, inspect relevant evidence, and provide a structured report |
| lbc scan [--path PATH] [--tree] [--json] | Describe project inventory accurately without implying all source contents were analyzed |
| lbc chat [--project PATH] [--ai] | Start one bounded interactive session; EOF or /exit ends it |
| lbc knowledge install/list/remove | Preserve current Markdown package functionality and data paths |
| lbc config show/set; lbc doctor | Report effective behavior and useful setup failures accurately |

Implementation details:
- No command may be a stub that always returns success or an example response.
- Plain-text ask/explain must expose useful content, not only document titles.
- inspect must work for embedded builtin IDs even when no repository is on disk.
- A bare ID is allowed only if unambiguous; otherwise display candidate source IDs.
- edit without --file uses the configured editor and preserves the original on
  cancellation or invalid input. Parse editor arguments without executing arbitrary
  shell syntax from note content.
- Never overwrite an existing entry accidentally during add.
- --json prints one documented machine-readable object/array on stdout; progress
  and warnings go to stderr. Test both text and JSON user paths.
- Errors need actionable messages and nonzero status. AI failure may preserve a
  successful offline answer, but the unavailable AI contribution must be explicit.

## 5. Knowledge model and storage

Markdown is the editable source of truth. Preserve compatibility with existing
YAML frontmatter, including id/title/language/tool/category/error_code/tags/keywords.
JSON output is an export representation, not a second competing source of truth.

Minimum new-entry requirements: nonempty stable id, title, and body.
Use kind = note, concept, or troubleshooting. Old documents remain valid with
sensible inferred/default kinds. Do not require cause/fix fields for ordinary notes.
Keep the existing technical language metadata distinct from output locale.

For troubleshooting, support structured Markdown sections for:
- Symptoms/error text and applicable environment
- Cause or explanation
- Suggested solution
- Verification steps
- References and verification status

These sections may be absent; render unknown information honestly. Do not invent
a root cause, successful outcome, command result, date, or source reference.
Verification status must distinguish unverified guidance, user-reported success,
and actual checks with recorded evidence. A retrieved score is not verification.

Storage locations:
- Built-ins: current embedded knowledge documents, inspectable by builtin:ID.
- Installed packages: preserve $XDG_DATA_HOME/lbc/knowledge and its current fallback.
- User notes: $XDG_DATA_HOME/lbc/notes, fallback ~/.local/share/lbc/notes.
- Explicit project notes: <project>/.lbc/knowledge; also read existing
  <project>/knowledge documents for compatibility.
- Generated index/cache: an XDG cache location, rebuildable from source Markdown.
- History, only when explicitly enabled: a separate user-data history location.

Use source-qualified identities. Duplicate IDs within one source are validation
errors. Across sources, show the distinction rather than silently discarding data.
Intentional overrides record their target source ID; project overrides take
precedence over user overrides in that project. An installed package must not
silently replace built-in guidance just because its ID collides.

Editing a user note or an explicit builtin override must affect the next search
and answer without recompiling. Invalidate cached content and refresh long-lived
sessions when source files change. The initial index may remain in memory; an
index command must not pretend a persistent cache exists when it does not.

Validate paths and bounded file sizes before reading/writing. Do not follow a
symlink out of the selected project/store. Use atomic replacement for edits.
Invalid documents must be reported with their paths; valid documents remain usable.
Keep all tests in isolated temporary stores, never the developer's real notes.

## 6. Retrieval must drive the answer

Use one shared retrieval path for search, ask, explain, and chat. A retrieved item
needs: source-qualified ID, title, relevant content, source locator, match reason,
and score. Optional line/section bounds should remain available for citations.

Implement this flow:
1. Normalize the question or diagnostics without removing important error tokens.
2. Retrieve candidate knowledge from the selected sources.
3. Rank and select relevant passages within an explicit context budget.
4. Collect bounded project evidence when the task needs it.
5. Build an offline answer using the passages and independently verified rules.
6. When --ai is present, send selected redacted passages and evidence to the model.
7. Render answer sections and source references that resolve to actual inputs.

Keep exact error codes as strong signals. Use useful lexical matching for titles,
metadata, and bodies; normalize punctuation and avoid ranking generic stopwords
above a matching error. Detect the producing tool where possible.
Support Thai Unicode text and curated bilingual keywords/aliases. Test Thai queries
against notes that actually contain matching Thai terms or aliases. Do not claim
cross-language semantic understanding merely because Unicode is accepted.

Offline behavior:
- Known rule + relevant note: show their contributions and any disagreement.
- Relevant note without a rule: show cited guidance, clearly marked as retrieved
  and not verified against this project.
- No adequate match: say so, show the observed error, and offer investigation steps.
- Weakly related notes must not be presented as proven fixes.
- Questions about general notes must work outside a Git/Cargo/project directory.
- More than one diagnostic must not disappear silently; report multiple results or
  state exactly which diagnostic was selected and how many others were detected.

Keep cause, suggested fix, verification, evidence, knowledge sources, and confidence
separate. A missing Cargo dependency does not prove an unresolved name is an external
crate: local modules, workspace members, aliases, target dependencies, and features
must not produce an unjustified "verified" diagnosis or cargo add recommendation.

## 7. Relevant project context

Keep inventory scanning lightweight. For explain/ask with a project:
- Select the file and line range referenced by a diagnostic, plus relevant manifests.
- Use bounded excerpts with path and line numbers, not the whole repository.
- Respect configured exclusions, .gitignore where applicable, hidden-file policy,
  generated directories, file-size limits, and a total evidence budget.
- Explicitly include valid project knowledge even though .lbc is hidden.
- Validate canonical paths and prevent diagnostic paths from escaping the project.
- Record which contents were actually read separately from inventory file counts.
- Missing files, unknown project types, or inaccessible evidence should lower
  certainty and produce warnings, not fabricated source analysis.

Do not run cargo, npm, Docker, systemctl, or other project commands automatically
from LBC's analysis commands.

## 8. AI, language, and memory

### AI

Reuse the existing OpenRouter and OpenAI-compatible integrations, including local
Ollama-compatible endpoints. Send selected knowledge contents, source IDs, and
project excerpts. A local path or title alone is not model-readable evidence.
Keep explicit timeouts, bounded prompts/responses, useful failures, and offline
fallback. Test requests against a local mock provider; CI must not need real keys.

The model should distinguish cited source material, project facts, and hypotheses.
A model's self-reported "high confidence" is not proof that a command worked.
Treat retrieved documents as untrusted content in the prompt; do not grant them
instruction authority or allow them to request additional file/network access.

### Language

Wire output.language = en | th | auto into text output and AI instructions.
Define and document auto selection with an English fallback. Keep code, commands,
paths, error codes, and JSON keys stable. Thai output must explain the content;
translated section headings alone are insufficient. Do not silently claim
unsupported locales work. Do not confuse document technical language with UI locale.

### Memory

Persist knowledge explicitly added by the user, independent of chat memory mode.
Default chat memory exists only inside the active lbc chat process. Independent
lbc ask invocations are stateless unless a future explicit session feature is added.
Closing the chat process must not delete the user's knowledge notes.

Bound in-memory history by message count and context size. Support /clear, /exit,
and EOF. Do not promise graceful cleanup after every kind of terminal termination;
instead, avoid writing default session history to disk in the first place.

memory.mode = persistent is explicit opt-in. Implement actual persistence,
redaction, retention, and an explicit history-clear operation if accepting this
setting. Do not accept a persistent mode that only changes config display.
Never persist full project contents by default. Make the history location and
behavior visible in config/doctor output.

## 9. Architecture and delivery order

Reuse current module boundaries. Add storage/write operations under knowledge/,
shared answer construction in a focused answer module, context extraction under
scanner/, and session logic in its own module. CLI handlers orchestrate these
components; they must not duplicate retrieval or contain large answer templates.
Keep pure retrieval/answer logic independently testable from terminal I/O.

Complete in order:

- [x] P0: Confirm libraryCube product naming, librarycube Cargo package/crate, and
      lbc executable naming across help, docs, errors, and tests.
- [x] P1: Entry identities, provenance, validated storage, add/list/inspect/edit,
      builtin inspection, explicit overrides, and backward-compatible loading.
- [x] P2: Useful search excerpts, index validation, shared retrieval, offline
      ask/explain, and knowledge-only answers including E0308 and custom notes.
- [x] P3: Bounded project evidence and cautious diagnostic confidence.
- [x] P4: AI consumes retrieved passages; mock-provider coverage and timeout fallback.
- [x] P5: English/Thai behavior and truthful chat/session/persistent-memory behavior.
- [x] P6: End-to-end regression coverage, working README examples, and Rust CI.

Keep each stage runnable. If work spans sessions, leave concrete completion status
and remaining blockers. Do not mark later stages complete because files exist.

## 10. Acceptance scenarios

These are required behavior tests. Use fixture content independent of the
implementation; assert meaningful answers and effects, not just help strings.

| ID | Scenario | Passing evidence |
| --- | --- | --- |
| A01 | Build/install/run libraryCube | lbc --help works; product is libraryCube; Cargo package/crate stay librarycube; binary stays lbc |
| A02 | Import an ordinary Markdown note | add generates/accepts ID; list and inspect show its exact stored content |
| A03 | Reuse a custom solution | ask/search/explain expose a unique solution phrase found only in the fixture note |
| A04 | Edit that solution | Next answer uses the new phrase and does not use the obsolete phrase |
| A05 | Explain E0308 without AI | Returns guidance from rust/E0308.md with a source ID despite absence of a special rule |
| A06 | Unknown error with no useful note | No fabricated fix, no misleading high confidence, useful next steps |
| A07 | General knowledge outside a project | A non-code question retrieves a matching concept note without Cargo or Git |
| A08 | Inspect and override a builtin | inspect works after installation; explicit override changes the effective answer without rebuilding |
| A09 | Colliding IDs and invalid edits | Ambiguity is visible; failed edit preserves original; packages cannot silently shadow builtins |
| A10 | Use actual source evidence | Excerpts have correct file/line bounds; ignored/oversized/out-of-root files are excluded |
| A11 | Provider receives relevant knowledge | Mock request contains the custom solution passage and source ID, within budget |
| A12 | Offline/privacy boundary | Without --ai there are zero provider calls; selected secrets are redacted from outgoing context |
| A13 | Provider failure | Timeout/error is visible and an available offline answer remains useful |
| A14 | Thai output and retrieval | Matching Thai note/alias is found; requested Thai explanation is meaningful; commands stay intact |
| A15 | Default session exit | Follow-up AI request has bounded session context; exit/EOF leaves no chat-history files |
| A16 | Explicit persistent history | Opt-in saves supported redacted history; a new session can restore it; clear really removes it |
| A17 | Existing package workflows | install/list/search/remove continue to work using isolated XDG directories |
| A18 | Honest diagnostics | Multiple errors are accounted for; local-module/workspace cases do not claim an unproved missing crate |
| A19 | JSON parity | JSON and text expose consistent facts, sources, and AI failure status |
| A20 | Direct file edit and index refresh | External Markdown edits are picked up by the next query or documented refresh in chat |

## 11. Target walkthrough

The following is the expected interaction after implementation. It is not output
captured from the baseline. Use it to build an automated end-to-end fixture.

Create a local fixture note titled "Port conflict in the demo service", containing:
"Change the demo service development port to 4318 in its configuration, then restart
the development process and confirm it listens on 4318." Mark this as a
user-authored suggestion, not a command LBC has run.

~~~bash
lbc add --id demo-port --title "Demo service port conflict" --kind troubleshooting --file ./port-note.md
lbc list
lbc search "demo service port conflict"
lbc inspect user:demo-port
lbc ask "How do I resolve the demo service port conflict?"
lbc edit user:demo-port --file ./updated-port-note.md
lbc ask "How do I resolve the demo service port conflict?"
printf '%s\n' 'error[E0308]: mismatched types' | lbc explain
lbc ask "How do I resolve the demo service port conflict?" --ai
lbc chat --ai
~~~

Required visible behavior:
- The first answer includes 4318, the note's source ID, and an unverified-guidance label.
- If the replacement note changes 4318 to 4429, the next answer uses 4429.
- The E0308 answer includes relevant type-mismatch guidance and its knowledge source.
- The mock AI provider receives the selected note body, not merely its filename.
- No step executes a repair command or silently changes the demo project.

Thai output target, when matching Thai material and output.language = th are used:

~~~text
พบความรู้ที่เกี่ยวข้อง: Demo service port conflict
แหล่งอ้างอิง: user:demo-port

คำแนะนำจากบันทึก:
เปลี่ยนพอร์ตของบริการเป็น 4318 ตามขั้นตอนในบันทึก แล้วตรวจว่าบริการเริ่มทำงานได้

สถานะ:
เป็นคำแนะนำจากคลังความรู้ ยังไม่ได้ตรวจยืนยันผลกับโปรเจกต์นี้
~~~

Actual formatting may differ; facts, provenance, and uncertainty must remain clear.

## 12. Validation and handoff

Run with a compatible stable Rust toolchain:

~~~bash
cargo fmt --check
cargo check --locked
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked
cargo build --locked --release
~~~

Test the resulting binary from outside the source checkout with isolated
XDG_CONFIG_HOME, XDG_DATA_HOME, and XDG_CACHE_HOME. Cover the target walkthrough,
the existing pipeline flow, and offline operation. CI should run formatting,
checking, clippy, and tests without live AI services or credentials.

If a toolchain/dependency/network limitation blocks validation, report the exact
blocker and the checks not run. Do not fabricate test results or mark the affected
gate complete. Fix concrete failures before adding more optional work.

Update README to distinguish implemented behavior from deferred features. Include
installation, entry creation/editing, plain Markdown format, source precedence,
offline/AI examples, Thai output, session memory, and troubleshooting.

The final implementation handoff must state:
- What user workflows now work
- What was changed and why
- Exact validation performed and actual outcomes
- Any remaining limitations or blockers
- A reviewable commit or pull request

Definition of done: a user can add a solution, find and inspect it, ask about it,
edit it, and receive an updated cited answer; error explanations use relevant
knowledge and project evidence; optional AI reads the same knowledge; settings
match real behavior; required tests pass. Do not finish at documentation alone
when the assigned task is to implement LBC.
