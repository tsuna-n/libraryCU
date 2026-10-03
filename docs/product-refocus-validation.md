# Product refocus validation

Local Linux worktree validation, 2026-10-03. This completes the requested product
boundary deliverables and first safe implementation increment, including `learn`
and initial deterministic retrieval/reuse improvements. Later roadmap milestones
remain planned; no release publication or native macOS/Windows validation is
claimed. The version remains 0.5.0 and changes remain in the worktree.

## Deliverable audit

| Required deliverable | Current evidence |
|---|---|
| Product decision | [product-direction.md](product-direction.md): identity, ordered principles, inspected repository findings, scope. |
| Revised contributor rules | [AGENT.md](../AGENT.md): knowledge-first constraints, sources, compatibility, proportional security, workflow checks. |
| Revised roadmap | [ROADMAP.md](../ROADMAP.md): foundation → capture → retrieval → project memory → lifecycle → personal v1.0; enterprise ideas preserved in [archive](archive/enterprise-roadmap.md). |
| Revised README introduction | [README.md](../README.md): local-first engine, Save/Find/Reuse, search/explain/learn examples, optional AI; distribution details moved deeper. |
| Feature classification | Product direction table covers every current top-level command, sources, providers, recovery, security, and unimplemented enterprise ideas. |
| Proposed architecture | Product direction maps existing modules, canonical Markdown, shared retrieval/evidence, scanner support, optional AI and separate fixer. |
| Migration/refactor plan | Product direction's five incremental steps; no store/config/ID migration, old optional-field-free notes preserved, future migration requirements explicit. |
| First safe implementation | `learn` prompts and scripts use existing safe storage; optional metadata, source/ID ranking, visible reasons, bounded problem/solution/check reuse, evidence-based AI abstention. |
| Changed-behavior tests | Six learn CLI tests, two index ranking tests, capture-inference/prompt tests, Markdown section/privacy test, expanded no-provider regression, existing storage/edit/package/security/fixer regressions. |

## Commands and outcomes

`cargo fmt --check`, `cargo test --locked`, and
`cargo clippy --locked --all-targets -- -D warnings` passed on the final
implementation. Test counts: 121 library + 6 CI configuration + 65 CLI + 12
filesystem + 6 privacy = **210 passed, zero failed**. The local test log is
`target/product-refocus-tests.log`. CLI help was also inspected for `learn`
flags, core workflow messaging, optional chat, and experimental patch/recovery
labels. Relative links in the changed product documents were checked.

The filesystem sandbox maps ownership in a way that caused private-store
regressions to fail. Final full-suite and benchmark validation ran outside that
sandbox with actual local ownership and loopback sockets; the sandbox-mapped
failure is not counted as a product pass. Tests still exercise ownership/ACL,
symlink, path-boundary, private-write, and recovery checks without weakening them.

The new CLI tests prove capture → search → inspect → ask/explain → external
Markdown reload → edit → current retrieval, exact-code project preference,
explicit project isolation, framework metadata lookup, generated-ID collisions,
explicit duplicate preservation, secret redaction, historical checks never
executed, unverified capture without a check, interactive preview/confirmation,
cancellation/EOF, and symlink rejection. The provider-socket regression includes
learn/search/inspect/list/index, normal ask/explain/chat, and explicit `--ai`
requests with insufficient evidence; none contacts the configured provider.

## Retrieval evidence and practical limits

[Benchmark receipt](evidence/product-refocus-retrieval.json) records the base
revision plus `+worktree`, current implementation file hashes, tested debug binary
hash, UTC check time, immutable original dataset/corpus hashes, all original
case outcomes, new project fixture hash, and measured latency. Source/binary
hashes were compared with current files before retaining this receipt.

Reproduce after `cargo build --locked` using the current base revision:

```bash
python3 benchmarks/retrieval/run.py --binary target/debug/lbc \
  --source-sha BASE_REVISION+worktree \
  --baseline docs/evidence/retrieval-after.json \
  --output target/retrieval-benchmark/product-refocus.json
```

| Metric | Observed | Denominator / interpretation |
|---|---|---|
| Top-1 relevance | 31/31 | Specific/ambiguous original cases with relevant labels; excludes fallback-only/no-match cases. |
| Top-3 relevance | 31/31 | At least one acceptable source among the first three adequate results. |
| Expected answer status | 40/40 | Original frozen labels; no per-case regression from the prior after receipt. |
| Citation identity/locator/excerpt | 40/40 | Same existing gate, including vacuous success when no citations exist. |
| Strict citation relevance | 33/40 | All citations acceptable per case; seven original failures remain. |
| Result precision | 35/75 (46.67%) | Acceptable source hits / all emitted adequate search hits, aggregated across original queries. |
| Irrelevant citation rate | 10/46 (21.74%) | Citations outside fixed acceptable label sets / all emitted citations. |
| No-match correctness | 8/8 | Original explicitly labeled no-adequate-match cases abstain with no citations. |
| Synthetic project memory | 6/6 | Project vs personal/builtin preference, isolated projects, explicit source ID, and near-code rejection. |
| Median / P95 search + ask | 37.82 / 43.74 ms | Three repetitions per original case on this Linux host, debug binary, subprocess startup and store/index reads included. |

This is a small, mostly curated corpus plus synthetic capture scenarios, not
independent real-incident accuracy. Debug latency is not comparable to the earlier
release-binary performance table. Result precision and the ten irrelevant
citations expose remaining weak supplemental retrieval; passing Top-K tests does
not establish that every recommendation is correct. No FTS/BM25/vector engine,
provider expansion, organization source, SSO, RBAC, or service architecture was
introduced. Larger incident evaluation and measured retrieval V2 remain roadmap
work, alongside later lifecycle improvements.
