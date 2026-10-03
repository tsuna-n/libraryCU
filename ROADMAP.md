# libraryCube roadmap — Save, Find, Reuse

libraryCube (`lbc`) is a local-first developer knowledge engine. Prioritize the
loop `find previous knowledge → solve → learn → reuse next time`. AI is optional;
enterprise capabilities are Future / Post-1.0, contingent on actual demand.

[Product direction, feature classification, architecture, and migration plan](docs/product-direction.md)
define scope. [AGENT.md](AGENT.md) defines contributor constraints.
The [archived enterprise roadmap](docs/archive/enterprise-roadmap.md) preserves
previous ideas and historical checklists without making them current requirements.

Milestones below are product targets, not claims of released versions. Cargo
remains 0.5.0. Historical release/security receipts remain in the dedicated
[acceptance ledger](docs/v0.5-completion-checklist.md) and
[release runbook](docs/release-0.5.0.md); a passing repository test does not prove
production signing, administrator controls, native trust, or publication.

| Milestone | Focus | Developer value |
|---|---|---|
| v0.5 | Stable local knowledge foundation | Reliable storage, retrieval, sources, citations, CLI |
| v0.6 | Knowledge capture | Save today's solution in under a minute |
| v0.7 | Retrieval V2 | Find a few strongly relevant solutions with understandable reasons |
| v0.8 | Project memory | Remember how this project solved it before |
| v0.9 | Knowledge lifecycle | Maintain, move, and improve a growing personal library |
| v1.0 | Reliable personal developer knowledge engine | Portable, fast, offline, cross-platform everyday use |

## v0.5 — Stable local knowledge foundation

Existing capabilities to retain and continuously test:

- Markdown/YAML as canonical knowledge; stable source-qualified IDs.
- User, explicit project, installed package, and builtin sources.
- Safe non-overwriting capture and atomic edits, locking, boundary validation.
- Offline search/ask/explain, adequate evidence, locators, truthful verification.
- Explicit overrides, duplicate-ID rejection, external Markdown reload.
- Clear no-match behavior, bounded project context, optional AI fallback.
- Existing Linux/macOS/Windows validation and proportional distribution safety.

Acceptance: save → search → inspect → edit → retrieve uses current content with
no AI credentials or network; invalid documents are reported without hiding valid
knowledge. Source collisions remain distinct and no migration loses notes.
Release readiness is evaluated separately through existing distribution runbooks.

## v0.6 — Knowledge capture

First capture increment is implemented in this refocus:

- Interactive `lbc learn`: problem, cause, solution, recorded check, destination,
  preview, confirmation; cancellation and EOF preserve stores.
- Scriptable capture with required problem/solution and explicit `--yes`.
- Portable troubleshooting Markdown, generated IDs/titles/tags, conservative
  language/tool/error-code assistance, creation/update timestamps.
- Project capture by default; explicit project/user selection.
- Recorded verification is historical and never executed during capture.

Remaining milestone work:

- [ ] Observe actual developer sessions and improve the under-one-minute path.
- [ ] Improve metadata hints based on ambiguous real captures without overguessing.
- [ ] Offer optional AI capture assistance only after the deterministic flow is excellent.

Acceptance: capture a solved incident and retrieve its problem, previous solution,
and historical check on the next command, entirely offline. Do not require a
Markdown template, provider setup, or schema tutorial.

## v0.7 — Retrieval V2

First increment: exact error-code/entry-ID precedence, bounded source preference,
and exposed ranking reasons using the existing weighted lexical implementation.

- [ ] Expand incident evaluation before choosing FTS5/BM25.
- [ ] Add a rebuildable FTS5/BM25 index if measured quality/latency justifies it.
- [ ] Improve title, metadata, identifier, and keyword ranking without weak result padding.
- [ ] Reduce unrelated supplemental citations and document abstention decisions.
- [ ] Track Top-1, Top-3, precision, irrelevant citation rate, no-match correctness,
  project-specific retrieval, and latency with fixed labels and distinct denominators.
- [ ] Optional embedded semantic retrieval only after deterministic quality gates.

Acceptance: exact codes and IDs beat weaker overlap, comparably relevant project
knowledge gets preference, unrelated queries abstain, and evidence identity and
ranking reasons are understandable. A curated 100% Top-1 score is not field accuracy.

## v0.8 — Project memory

- [ ] Improve context-aware ranking with bounded manifests/diagnostic excerpts.
- [ ] Evaluate previous project solutions across similar incidents and repositories.
- [ ] Add portable knowledge links and related-entry navigation.
- [ ] Capture useful framework/version context without introducing mandatory schemas.
- [ ] Keep explicit project scope and test isolation between repositories.

Acceptance: explain retrieves how THIS project solved a comparable problem before;
project preference never turns unrelated material into evidence or proof of cause.

## v0.9 — Knowledge lifecycle

- [ ] Portable import/export and backup/restore workflows.
- [ ] Update/merge tools that preserve stable IDs and user edits.
- [ ] Duplicate detection with explicit, reviewable merges.
- [ ] Stale knowledge and broken-reference detection.
- [ ] Lightweight quality checks and supported-version metadata.
- [ ] Interrupted/read-only/disk-full failure and concurrent-store regressions.

Acceptance: a growing library remains understandable, movable, editable, and
recoverable; health hints do not silently rewrite or discard developer knowledge.

## v1.0 — Reliable personal developer knowledge engine

- [ ] Excellent local Save/Find/Reuse workflow proven through real use.
- [ ] Stable storage/config/CLI formats, compatibility and deprecation policy.
- [ ] Good retrieval with relevant citations and reliable abstention.
- [ ] Portable developer-owned data with tested recovery and migration.
- [ ] Fast CLI measured on realistic libraries, with median and P95 latency.
- [ ] Supported Linux/macOS/Windows behavior verified on native platforms.
- [ ] Clear installation, capture, source, maintenance, and offline documentation.
- [ ] Safe optional AI integration grounded in selected evidence.
- [ ] Proportional security and distribution checks required by the chosen release path.

Ready when a developer can capture, find, inspect, edit, understand, and reuse
personal/project knowledge reliably offline, then move that knowledge to another
supported machine. No SSO, organization server, or coding-agent identity is required.

## Future / Post-1.0

Preserved ideas, contingent on demonstrated demand; do not expand them now:

- Team/organization sources and offline synchronization, trusted package registries.
- Enterprise policy, centralized audit, controlled verification sandbox.
- SSO/OIDC, RBAC, device management, organization control plane.
- Managed server/SaaS, multi-tenant administration, organization deployment/pilots.
- Organization analytics and productivity dashboards.
- GUI/TUI extensions, editor integrations, marketplace, executable plugin SDK.
- Advanced semantic recommendations and automatic capture suggestions.
- Autonomous coding/multi-agent tooling and distributed vector services.

Existing fix/proposal/apply/verification/rollback stays experimental with its safety
checks and command compatibility. Existing chat/history stays optional supporting
knowledge Q&A. Neither drives milestones. More providers, model routing, and
enterprise abstractions wait behind useful local capture and retrieval.
