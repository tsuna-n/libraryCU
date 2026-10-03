# Product direction: Save, Find, Reuse

Decision: refocus libraryCube (`lbc`) on a small, reliable, local-first developer
knowledge engine. This supersedes the enterprise sequence in the
[archived roadmap](archive/enterprise-roadmap.md). The version remains 0.5.0;
this decision is not a release or a claim that historical production gates passed.

Save solutions, notes, debugging knowledge, and project-specific fixes as local
Markdown. Find evidence deterministically. Reuse previous solutions, then capture
the next result. The product is useful without AI or a network connection.

## Principles, in priority order

1. Knowledge-first
2. Local-first
3. Deterministic-first
4. Developer-owned
5. Offline-capable
6. Simple CLI UX
7. Evidence-based retrieval
8. AI-optional
9. Safe by default
10. Enterprise features only when real demand exists

The primary loop is `search / ask / explain → solve → learn → retrieve next time`.
Capture, search, inspection, editing, and diagnostic reuse must work without model
APIs, Ollama, external vector databases, Python services, or cloud services.

## Repository findings and feature classification

Classification follows the actual dispatch, storage, retrieval, scanner, AI,
history, and fixer modules, rather than the previous roadmap's labels.

| Feature / command | Status | Implementation and reason |
|---|---|---|
| `add` | CORE | Imports Markdown bodies through a locked, non-overwriting writer. |
| `learn` | CORE | Captures a solved problem with context and a recorded check; uses the same writer. |
| `search` | CORE | Deterministic local retrieval with excerpts, qualified IDs, and ranking reasons. |
| `ask` | CORE | Answers from adequate retrieved local passages; optional `--ai` enhancement. |
| `inspect` | CORE | Full document, source, writability, and verification transparency. |
| `edit` | CORE | Atomic maintenance; changed bodies invalidate recorded verification status. |
| `explain` | CORE | Diagnostic parsing plus the shared knowledge index and bounded project evidence. |
| Project/user stores | CORE | Developer-owned memory; explicit project scope and portable Markdown. |
| Builtin/installed package retrieval | CORE | Offline evidence sources with stable qualified identities. |
| `list` | SUPPORTING | Browse effective entries. |
| `index` | SUPPORTING | Validates/rebuilds retrieval state in memory; no persistent cache is claimed. |
| `knowledge install/list/remove` | SUPPORTING | Explicit local package management, integrity checks, and atomic publication. |
| `doctor`, `config` | SUPPORTING | Diagnose local health and configure the CLI; connectivity is opt-in. |
| `scan` / scanner | SUPPORTING | Detect stack/manifests and bounded diagnostic context; inventory is not analysis of every file. |
| AI providers | SUPPORTING, OPTIONAL | Existing providers enhance retrieved evidence; provider expansion is not a priority. |
| `chat` | SUPPORTING, OPTIONAL | Repeated knowledge Q&A already uses shared retrieval; no autonomous tools. |
| `history` | SUPPORTING, OPTIONAL | Manages explicitly persisted, redacted chat history; session history is not canonical knowledge. |
| `fix` | EXPERIMENTAL | AI patch preview and explicit application modify source; secondary to knowledge. |
| `apply-proposal` | EXPERIMENTAL SUPPORT | Applies reviewed saved bytes offline with stale-target/integrity checks. |
| `rollback`, explicit verification | EXPERIMENTAL SUPPORT | Recovery and honest executed-check outcomes needed by source changes. |
| Safe writes, locks, boundaries, redaction | SUPPORTING | Protect real local operations; retain security regressions. |
| Release signing/SBOM/provenance/native trust | SUPPORTING | Distribution safeguards; dedicated runbooks, separate production gates. |
| SSO, RBAC, enterprise policy/audit/control plane | FUTURE / POST-1.0 | Roadmap ideas, not implemented CLI commands or current dependencies. |
| Organization/team/cloud knowledge server | FUTURE / POST-1.0 | Not part of the existing source loader; no new source hierarchy. |
| Legacy/remove candidates | NONE YET | No existing command is removed merely because it is secondary. |

The unnecessary complexity was principally the enterprise-first milestone
sequence and infrastructure-heavy introduction. Source-changing tooling is large
but already separated in `fixer` modules and valuable for recovery. Keep its
safety engineering and compatibility; do not rewrite the core around it.

## Architecture: existing boundaries and intended evolution

```text
add / learn / edit
       ↓
validated Markdown + YAML → locked atomic store writes
       ↓
project → user → installed package → builtin
       ↓
loader: validate / qualified identities / explicit overrides
       ↓
retrieval: normalize → exact code / ID → title / metadata / lexical ranking
       ↓
adequate evidence + reasons + locators + verification status
       ↓
search / ask / explain → offline output → optional AI explanation
       ↑                                     ↓
scanner: bounded local context       separate experimental fixer
```

`src/knowledge/storage.rs` owns safe publication; `document.rs` describes optional
metadata and views of ordinary Markdown sections; `loader.rs` loads all four
sources and handles validation/overrides; `index.rs` performs deterministic
ranking; `retrieval.rs` is shared by search, ask, chat, and diagnostics.
`answer.rs` selects adequate passages. `diagnostics` parses logs and adds local
rules and bounded scanner evidence. CLI modules handle prompts and presentation.
`ai` receives deliberately bounded, redacted evidence. `fixer` owns proposals,
application, executed verification, and recovery independently of capture.

Current retrieval is an in-memory weighted lexical index, **not FTS or BM25**.
SQLite is already used for opt-in history, not canonical knowledge. Normalizing
queries, exact-code precedence, complete metadata phrases, word boundaries,
deduplicated terms, language filtering, and abstention already exist.
The refocus adds exact entry-ID matching and a bounded source preference only
after a real match. Exact codes and IDs remain ahead of weaker matches; full
title/metadata-phrase signals retain their existing precedence for lexical matches.
Among exact-code hits, source preference precedes generic diagnostic wording.
Project then user
then package preference applies among comparably relevant matches; it cannot
create relevance. Ranking reasons are visible in text and JSON.

Next: measure lexical precision on larger incident corpora before introducing a
rebuildable SQLite FTS5/BM25 index. Optional embedded semantic retrieval comes
after deterministic quality, never as a normal-usage requirement. Evidence
selection should improve precision rather than fill result quotas.

## Capture and project memory

`lbc learn` asks for problem, optional cause, solution, optional historical check,
project/user destination, and confirmation. It previews the content and
destination; cancellation or EOF does not save. The default destination is the
current explicit directory's `.lbc/knowledge`; no upward project discovery occurs.
Use `--project PATH` or `--user` to choose a destination explicitly.

For scripts:

```bash
lbc learn --yes --problem 'Rust E0308 String to &str mismatch' \
  --cause 'Function expected &str but database value was String' \
  --solution 'Borrowed the database value using &value' \
  --verification 'cargo test' --project ./backend
lbc search E0308 --project ./backend
lbc explain build.log --project ./backend
```

Capture generates a collision-safe ID, title, troubleshooting kind, conservative
language/tool/error-code hints, tags, and UTC epoch-second creation/update times.
Flags override hints, and `--framework` records explicit framework metadata.
Problem, symptoms, context, cause, solution, verification,
and references live in ordinary Markdown sections; only problem and solution
are required. Unknown or ambiguous languages need not be guessed.
`ask` and `explain` expose bounded problem/solution/check sections when present;
other hand-written notes still use their existing excerpts.

`learn` never executes the recorded check. A nonempty check is `recorded-check`;
otherwise it is `unverified`. Existing `user-reported` notes remain supported.
These describe an author's record, not a freshly tested fix. Source preference
does not establish a root cause or correctness. No adequate evidence means an
explicit no-match or labeled general investigation guidance.
`ask --ai` also abstains without adequate knowledge, and `explain --ai` abstains
when neither retrieved knowledge nor a known diagnostic rule supplies evidence.

## Migration and incremental refactor plan

1. **Product boundaries (implemented):** revise AGENT, README, roadmap, and this
   decision; label optional/experimental commands without renaming them. Archive
   enterprise ideas and preserve dedicated security/release runbooks.
2. **Capture (implemented here):** complete `learn` using existing safe storage;
   add optional timestamps and structured Markdown, project/user capture,
   confirmation, scripting, and workflow/security regressions.
3. **Retrieval (small first step here, V2 planned):** exact entry IDs, source
   preference, visible reasons, and learned solution reuse. FTS/BM25 and broader
   evaluation are future measured changes, not claimed implementation.
4. **CLI/documentation:** keep compatibility aliases/top-level commands;
   reconsider an `experimental` group only with a documented deprecation path.
   Keep ordinary knowledge workflows first. Do not add provider abstractions.
5. **Evaluation:** retain frozen baseline cases and gradually add anonymized real
   incidents, project memory, external edits, no-match and collision cases.

No user-store, package, project-directory, ID, config, or command migration is
needed for these changes. Older Markdown lacks the new optional timestamps and
remains valid. `project:ID` stays the existing identity; project scope supplies
the namespace. Do not invent incompatible `project:repo:ID` keys. Explicit
overrides, duplicate-ID rejection, and the existing identical-content deduplication
remain compatible. A future format change must detect old data, migrate safely,
preserve originals on failure, document recovery, and test it before shipping.

## Acceptance and evaluation

The [validation record](product-refocus-validation.md) audits all requested
deliverables and records current tests, benchmark results, and remaining limits.

Changed-behavior evidence belongs in `tests/cli/learn.rs`, index unit tests,
and the existing offline/provider, edit, filesystem, privacy, and fixer tests.
Verify capture → search → inspect → ask/explain → edit → retrieve, project
isolation, exact code/ID precedence, unrelated abstention, external Markdown
reload, cancellation/EOF, duplicate preservation, historical-check semantics,
secret redaction, and no AI connections from normal capture/retrieval.

Keep `benchmarks/retrieval/cases.json` labels frozen when comparing runs. Track
Top-1, Top-3, result precision, irrelevant citation rate, no-match correctness,
project-specific retrieval, and latency. Existing 40-case metrics are small-corpus
regressions, not production accuracy or evidence that a solution resolved an
incident. The current benchmark report and historical release receipts remain
dated evidence; they do not certify this worktree or production readiness.

Decision test for every new feature: **Does this make it easier to capture, find,
understand, or reuse a developer's own technical knowledge?** If it does not,
keep it supporting, experimental, future, or omit it.
