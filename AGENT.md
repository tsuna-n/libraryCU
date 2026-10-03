# libraryCube Product Rules

These rules are architectural and product-level constraints.

All agents, contributors, and automated coding systems working on libraryCube MUST follow them.

Read [docs/product-direction.md](docs/product-direction.md) for the inspected feature
classification, current architecture, and incremental migration/refactor plan.
[ROADMAP.md](ROADMAP.md) is the product roadmap. Archived enterprise and release
ledgers remain historical evidence, not blockers for knowledge capture work.

Apply principles in this order: knowledge-first, local-first, deterministic-first,
developer-owned, offline-capable, simple CLI UX, evidence-based retrieval,
AI-optional, safe by default, enterprise only when real demand exists.

Keep the source model project → user → installed package → builtin when relevance
is comparable. Preserve qualified IDs and explicit override semantics. Do not
introduce organization/team/cloud sources during the refocus.

The primary CLI is add, learn, search, ask, inspect, edit, explain. List, index,
doctor, config, knowledge packages, and scanner support it. Chat/history is optional
knowledge Q&A. Fix/apply-proposal/rollback is experimental; preserve compatibility
and its recovery/security checks. Do not silently rename or remove commands.

For knowledge changes, verify actual save/retrieve/project/exact-code/no-match/
edit/external-Markdown/learn/explain workflows offline. Run `cargo fmt --check`,
`cargo test --locked`, and `cargo clippy --locked --all-targets -- -D warnings`.
Keep fixed benchmark labels; measure precision and irrelevant citations as well
as Top-1/Top-3, no-match, project retrieval, and latency. Native platform/release
evidence remains a separate gate; do not infer it from local Linux tests.

---

## 1. Product Identity

libraryCube is a:

> Local-first developer knowledge engine.

Its primary job is to help developers:

```text
capture
find
understand
reuse
```

technical knowledge from their own work.

libraryCube is NOT primarily:

- an AI coding agent
- an autonomous software engineer
- a generic chatbot
- an IDE replacement
- an enterprise control plane
- an SSO/RBAC platform
- a centralized knowledge SaaS

Do not change the product identity without an explicit architectural decision.

---

## 2. Knowledge Is the Core

The following concepts are Core:

```text
knowledge storage
knowledge capture
knowledge retrieval
knowledge inspection
knowledge editing
project knowledge
user knowledge
knowledge packages
retrieval evidence
diagnostic-to-knowledge retrieval
```

Development effort should prioritize these areas.

---

## 3. AI Is Optional

All essential libraryCube functionality must work without:

```text
API keys
LLMs
cloud services
Ollama
vector databases
remote servers
```

AI may improve results.

AI must not be required to access the user's own knowledge.

Correct architecture:

```text
local retrieval
    ↓
evidence
    ↓
optional AI
```

Incorrect architecture:

```text
user query
    ↓
AI provider
    ↓
answer
```

---

## 4. Retrieval Before Generation

Always prefer existing knowledge over generated content.

Before asking an AI model for an answer:

1. search local knowledge
2. rank evidence
3. determine whether evidence is adequate
4. expose relevant sources
5. only then optionally use AI

Never fabricate a local source.

Never imply generated knowledge already existed in the local library.

---

## 5. Developer-Owned Data

Knowledge belongs to the user.

Prefer:

```text
Markdown
YAML metadata
SQLite indexes that can be rebuilt
portable files
transparent formats
```

Avoid making critical user knowledge depend on opaque databases.

Derived indexes may be rebuilt.

Canonical knowledge should remain inspectable and portable whenever practical.

---

## 6. Project Memory Is Important

Project knowledge should capture how a specific project solved a problem.

When relevance is comparable:

```text
project knowledge
```

should normally outrank generic knowledge.

libraryCube should answer:

> How did this project solve this before?

not just:

> What does generic documentation say?

---

## 7. Capture Must Be Easy

Knowledge capture should require minimal ceremony.

The preferred workflow is:

```text
solve problem
    ↓
lbc learn
    ↓
save useful knowledge
```

Users should not need to understand the entire internal schema to save useful knowledge.

The system should assist with:

```text
ID generation
metadata
title
error code
language/tool detection
structure
```

while keeping the stored result human-readable.

---

## 8. Evidence Must Be Honest

Distinguish between:

```text
unverified
user-reported
recorded-check
freshly verified
```

Do not describe a recorded historical verification as a current verification.

Do not claim a root cause unless evidence supports it.

Do not convert retrieval confidence into factual certainty.

---

## 9. Simple CLI First

The common workflow should require as few steps as practical.

Prioritize:

```bash
lbc learn
lbc search
lbc ask
lbc explain
```

Avoid forcing users through multiple commands when one coherent command can safely perform the workflow.

Do not add new top-level commands without strong justification.

---

## 10. Features Must Strengthen the Knowledge Loop

Before implementing any significant feature, answer:

```text
Does this improve:

- capture?
- retrieval?
- understanding?
- reuse?
- knowledge quality?
- knowledge safety?
```

If no, it should normally NOT become Core.

Classify it instead as:

```text
Supporting
Experimental
Future
```

---

## 11. Do Not Compete With Coding Agents

Do not redesign libraryCube into a clone of:

```text
Codex
Claude Code
Cursor
Copilot
Aider
Cline
```

Code generation and automated patching may exist as optional tooling.

The differentiator of libraryCube must remain:

```text
persistent local developer knowledge
```

---

## 12. Code Modification Is Experimental

Functions that modify project source code, including AI-generated patches, must remain secondary to knowledge functionality.

They must:

- require explicit user intent
- preserve recovery where appropriate
- avoid silent modification
- expose verification state
- remain separable from the knowledge core

Do not design the entire architecture around autonomous code changes.

---

## 13. Enterprise Scope Is Deferred

Do not implement or expand the following as current Core functionality unless explicitly approved:

```text
SSO
RBAC
multi-tenant control plane
organization policy server
enterprise governance platform
centralized audit platform
complex organization deployment
```

Keep future designs documented if useful.

Do not let future enterprise requirements distort the local-first architecture.

---

## 14. Avoid Premature Infrastructure

Do not introduce:

```text
microservices
message queues
distributed databases
remote control planes
mandatory cloud APIs
external vector databases
```

for problems that can reasonably be solved locally.

Prefer the smallest reliable architecture.

---

## 15. Retrieval Quality Beats Feature Count

Improving retrieval from:

```text
good → excellent
```

has higher priority than adding another unrelated feature.

Prioritize:

```text
exact error matching
metadata matching
FTS/BM25
ranking
project-aware retrieval
precision
citation quality
abstention
```

before provider expansion or enterprise features.

---

## 16. Prefer Precision Over Result Count

Do not return weak results merely to fill a result list.

Prefer:

```text
3 relevant results
```

over:

```text
10 loosely related results
```

When evidence is inadequate, say so.

Correct abstention is a feature.

---

## 17. Benchmarks Must Reflect Reality

Benchmarks are regression tools, not marketing claims.

Never interpret:

```text
100% benchmark result
```

as:

```text
100% real-world correctness
```

Keep benchmark datasets versioned.

Do not modify expected labels simply to improve scores.

Add real incidents when possible.

---

## 18. Backward Compatibility Matters

Protect existing:

```text
knowledge files
user stores
project stores
configuration
stable CLI workflows
```

Do not destroy user knowledge during migrations.

Any migration affecting user data requires:

- compatibility analysis
- migration path
- tests
- failure handling

---

## 19. Do Not Rewrite Without Need

A cleaner architecture alone is not sufficient justification for a rewrite.

Prefer incremental changes.

Refactor when it improves:

```text
product boundaries
correctness
maintainability
retrieval
UX
testability
```

Do not refactor stable code purely for aesthetic consistency.

---

## 20. Security Must Be Proportional

Maintain security measures that protect real local operations:

```text
path validation
safe file handling
atomic writes
permissions
secret redaction
safe recovery
dependency checks
release integrity
```

Do not add enterprise security infrastructure without a concrete requirement.

---

## 21. Documentation Must Lead With User Value

README ordering should prioritize:

```text
What is libraryCube?
Why use it?
How do I install it?
How do I use it?
How does knowledge work?
```

Deep details about:

```text
SBOM
provenance
signing
notarization
CI internals
```

belong in dedicated documentation.

Do not let infrastructure dominate the product introduction.

---

## 22. Core Architecture Boundary

The conceptual architecture should remain close to:

```text
            libraryCube
                 │
          Knowledge Engine
                 │
      ┌──────────┼──────────┐
      ↓          ↓          ↓
   Capture     Search     Retrieve
      │          │          │
      └──────────┼──────────┘
                 ↓
              Explain
                 │
          ┌──────┴──────┐
          ↓             ↓
       Offline      Optional AI
```

New modules should have a clear place within or around this architecture.

---

## 23. Preferred Product Loop

The most important end-to-end workflow is:

```text
Developer encounters problem
        ↓
lbc search / ask / explain
        ↓
Existing local solution found
        ↓
Developer applies solution
        ↓
Problem resolved
        ↓
lbc learn
        ↓
Knowledge becomes available next time
```

Optimize libraryCube around this loop.

---

## 24. Decision Rule

For every new feature, ask:

> Does this help developers capture, find, understand, or reuse their technical knowledge?

If yes:

Consider it for Core.

If indirectly:

Supporting.

If experimental:

Isolate it.

If future-facing:

Document it without forcing the current architecture to support it.

If no:

Do not add it.

---

## 25. Product Motto

All development decisions should preserve this principle:

> Knowledge first. Local first. AI optional.

Or, in operational terms:

```text
Save.
Find.
Reuse.
```
