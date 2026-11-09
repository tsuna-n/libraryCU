# Offline FTS5/BM25 experiment

Keep FTS5 experimental. The measured BM25 rerank did not improve the existing
ranking, and unrestricted FTS candidates weakened precision and abstention. No
production index, dependency, answer policy, or knowledge format was changed.

The [runner](../benchmarks/retrieval/compare_fts.py) uses Python's standard-library
SQLite against 59 documents loaded through actual `lbc list` / `inspect` commands.
It installs the same Python and synthetic reference packages into temporary,
isolated XDG stores. The original 40 cases and all corpus file hashes must match
the frozen [retrieval receipt](evidence/retrieval-after.json), or the run fails.
No AI provider, embedding, vector database, network request, or real user store
participates.

The fixed candidate configuration is `unicode61`, OR over deduplicated quoted
whitespace phrases, and BM25 title/metadata/body weights 5/3/1. Those weights were
chosen before measurement; no parameter search was performed. Quoting treats
query operators as data; doubled quotes escape literal quotes. SQLite defines
smaller BM25 values as better matches, supports per-column weights, and describes
the tokenizer's Unicode character categories in its
[official FTS5 documentation](https://www.sqlite.org/fts5.html#the_bm25_function)
and [Unicode61 documentation](https://www.sqlite.org/fts5.html#unicode61_tokenizer).
The default tokenizer splits punctuation, including code underscores, and is not
a Thai word segmenter. Quoted identifier phrases retain token order, but this is
not equivalent to the production exact-code signal or Thai fragment matching.

Two candidates separate ranking from evidence eligibility:

- `bm25_rerank` sorts exactly the adequate candidates emitted by `lbc search`.
  Missing BM25 matches and equal scores retain CLI order. Candidate membership,
  result precision, and empty-result abstention cannot improve by construction.
- `fts_broad_top3` takes the first three unrestricted FTS matches. It intentionally
  has no production adequacy gate, exact-code override, language context filter,
  or project priority. This diagnostic shows why FTS alone is not a replacement
  retrieval policy.

The pre-change binary measured on 2026-11-09 used SQLite 3.54.1 and the unchanged
dataset hash `6ba824757fca6994397873de76a7f71d3f57a8a0b2f0a56f0f00bf42e4b7a794`.
The [pre-change receipt](evidence/retrieval-fts5-before.json) and current binary's
[full receipt](evidence/retrieval-fts5.json) retain the measured results,
including every case, every broad FTS score, binary/experiment/loaded-document
hashes, corpus hashes, actual CLI answers, and timing scope.

| Pre-change measurement | CLI lexical | Same-candidate BM25 | Broad FTS top 3 |
|---|---:|---:|---:|
| Top-1 relevance | 31/31 | 31/31 | 30/31 |
| Top-3 relevance | 31/31 | 31/31 | 31/31 |
| Relevant results / returned results | 35/75 (46.67%) | 35/75 (46.67%) | 35/99 (35.35%) |
| All first-three sources relevant (proxy) | 30/40 | 30/40 | 7/40 |
| No-match cases with no candidates (proxy) | 8/8 | 8/8 | 5/8 |

Top-K excludes the eight no-source cases and R18's general-guidance-only case,
as in the established benchmark. Precision uses all returned adequate CLI
candidates, or the explicitly capped three broad FTS candidates. The strict
first-three-source proxy counts empty source lists as passing, so it must be
read alongside Top-K and no-match results.

Broad FTS puts the Node module note above the relevant Python note for R15,
`Python หา module ไม่เจอ`. It emits candidates for unsupported TS9999 (R34),
`Go hyperwarp quantumflux` (R36), and `database quorum split brain incident`
(R40). The same-candidate rerank changes only lower-ranked ordering in R15 and
R17; no measured Top-K or strict first-three-source gain results. Exact-code and
Thai cases remain in the report; they were not removed or relabeled.

Actual pre-change CLI answers achieve 40/40 expected statuses, 40/40 citation
identity/locator/excerpt correctness, and 33/40 strict citation relevance, with
10/46 irrelevant cited passages. These measure the existing answer policy.
There is **no candidate answer policy**: candidate answer/citation metrics are
explicitly null, and the first-three-source relevance values above are proxies,
not measured citations. Reordering retrieved candidates alone does not establish
an improvement to answers or citations.

Pre-change median search + ask is 15.71 ms (P95 19.51 ms), including two subprocess
startups and store reads. Warm in-process FTS query plus candidate rerank is
0.107 ms (P95 0.206 ms), excluding startup, loading, answer construction, and
the 3.10 ms in-memory index build. These scopes differ: this is not evidence of
an end-to-end CLI speedup. Timing is host-dependent and not a promotion gate.

The experiment was also repeated against the final retrieval implementation:

| Final implementation | CLI lexical | Same-candidate BM25 | Broad FTS top 3 |
|---|---:|---:|---:|
| Top-1 relevance | 31/31 | 31/31 | 30/31 |
| Top-3 relevance | 31/31 | 31/31 | 31/31 |
| Relevant results / returned results | 35/40 (87.50%) | 35/40 (87.50%) | 35/99 (35.35%) |
| All first-three sources relevant (proxy) | 37/40 | 37/40 | 7/40 |
| No-match cases with no candidates (proxy) | 8/8 | 8/8 | 5/8 |

BM25 again demonstrates no measured benefit. Final actual CLI answers achieve
37/40 strict citation relevance, with 4/40 irrelevant passages. The FTS runner's
per-case citation identity is the legacy check (empty citation lists pass);
use the [main benchmark](retrieval-engine-improvements.md) for the additional
nonempty emitted-citation identity denominator. Final median/P95 CLI search + ask
is 15.23/18.48 ms; warm FTS query/rerank is 0.090/0.200 ms, with a 2.62 ms index
build. Those measurements still have different scopes and cannot support a
production latency claim.

Reproduce from the repository root:

```bash
python3 -m unittest discover -s benchmarks/retrieval -p test_compare_fts.py
python3 benchmarks/retrieval/compare_fts.py --binary target/release/lbc \
  --output docs/evidence/retrieval-fts5.json
```

The experiment requires a Python SQLite build with FTS5 and fails explicitly if
unavailable. There is no silent fallback to another retrieval method. A future
production proposal needs a measured gain with no case regressions, real
answer/citation evaluation, project-source priority checks, and a rebuildable
index strategy. This small curated corpus provides no large-corpus scaling,
independent incident, or native-platform evidence.
