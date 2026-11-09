# Retrieval engine evaluation — 2026-11-09

The [reliability follow-up](retrieval-reliability.md) investigates the five
remaining failures below, preserves these historical results, and measures
general fixes against both exposed regression sets and a newly frozen set.

This change improves deterministic, offline retrieval using the existing Markdown
stores and shared retrieval entry point. It adds no production search backend,
dependency, vector database, provider requirement, or data migration. All original
40 labels, seven reference notes, shipped knowledge, and six project-memory
fixtures remain unchanged.

## Pipeline and failure analysis

The loader reads effective project, user, installed-package and builtin documents,
preserving qualified identities and explicit overrides. `KnowledgeIndex` scores
whole error codes, explicit IDs, titles, metadata phrases and deduplicated lexical
terms. It retains deterministic title/source tie breaking and localized excerpts.
Previously `search` and `explain` used a per-result two-thirds term-coverage gate;
`ask` additionally suppressed non-code supplements for exact-code queries.

That coverage gate counts generic words and Thai fragments as evidence. A note
can therefore cover most query terms while missing the actual symptom. Metadata
phrase ranking put a specific note first but still allowed weaker supplements.
The large additive source preference could also let a weak project overlap beat
more specific generic guidance. Citation identities were real; many extra sources
were irrelevant under the fixed conservative labels.

The shared retrieval path now applies the same evidence selection to search,
ask/chat and explain. Known complete error codes remain strongest; unsupported
Rust/TypeScript numeric codes, `ERR_` codes and longer underscore variants of
loaded codes cannot borrow a different error's message. Identifier matching is
independent of the primary ranking reason, including when an explicitly requested
note also matches a diagnostic code. Existing underscore tokenization is retained
so Python variable names do not become mandatory error codes.

Whole matched symptom phrases now outrank incidental overlap, whether recorded
in metadata, a title or ordinary Markdown. Exact titles, adequate body-only code
notes and general concept references remain usable. Where no symptom phrase or
exact code applies, supplemental lexical hits must reach two-thirds of the best
lexical score, excluding source bonuses. Source preference is bounded to
project/user/package = 30/20/10 points instead of 160/80/40; comparable exact-code
matches still explicitly prefer project → user → package → builtin. Source alone
cannot create a hit. This is a deterministic heuristic, not calibrated probability.

No adequate evidence yields explicit `no_adequate_match` with no citations. The
existing unknown-Python-error fallback remains explicitly `general_guidance`,
never a claimed specific fix. AI remains optional and is not contacted by these
benchmarks.

## Measurement protocol

The pre-change optimized binary was built and copied before any retrieval edits.
Both binaries use the same pre-existing dependency worktree and
`cargo build --locked --release`. The base revision is
`21a8ea21362b0f5502d357cd045cd972428037fb`; both measurements identify their dirty
worktree explicitly and retain binary and implementation hashes. The
[before source snapshot](evidence/retrieval-v2-source-before.json) records the
source that produced the preserved binary; the final runner validates that binary
hash before reusing its source receipt. This avoids attributing today's source
files to an older executable.

The original 40 are an exposed tuning/regression set. A separate agent froze
[12 explicitly synthetic evaluation cases](../benchmarks/retrieval/holdout.json)
before seeing ranking changes. Its hash is
`f0970ca06b061fce20baae07a40f44541866514a2ee192f9334c56509a9a8aac`.
Ranking was frozen before evaluating that set, and was not adjusted from its
failures. Related corpus documents are shared; queries and incident provenance
are checked for split leakage. This is not an independent field-incident study.

All runs isolate XDG stores, install the same packages, execute the actual CLI,
retain every result/passage/answer and three timing samples, and validate citation
identity against loaded documents. Reports are saved before regression checks.
The runner rejects formerly passing per-case metrics, precision or citation-rate
regressions; it never changes a label to obtain a pass. Empty citations cannot
inflate the emitted-citation identity denominator. Latency includes separate
search and ask startup/store reads and is not a machine-independent guarantee.

## Results

The original dataset hash remains
`6ba824757fca6994397873de76a7f71d3f57a8a0b2f0a56f0f00bf42e4b7a794`.
[Before](evidence/retrieval-v2-before.json) and
[after](evidence/retrieval-v2-after.json) use the same final runner and corpus.
The per-case baseline gates pass; neither an old passing prediction nor its
precision/citation-rate metric regressed.

| Original tuning/regression set | Before | After |
|---|---:|---:|
| Top-1 | 31/31 | 31/31 |
| Top-3 | 31/31 | 31/31 |
| Result precision | 35/75 (46.67%) | 35/40 (87.50%) |
| Strict citation relevance | 33/40 (82.50%) | 37/40 (92.50%) |
| Irrelevant cited passages | 10/46 (21.74%) | 4/40 (10.00%) |
| Actual emitted citation identity/locator/excerpt | 46/46 | 40/40 |
| Expected answer status | 40/40 | 40/40 |
| No-match accuracy | 8/8 | 8/8 |
| Synthetic project-memory scenarios | 6/6 | 6/6 |
| Median search + ask | 14.27 ms | 15.37 ms |
| P95 search + ask | 17.71 ms | 19.60 ms |

All 35 previously returned relevant search results remain. The reduction in result
count removes 35 irrelevant search hits; it is not obtained by dropping relevant
hits from this fixed set. The measured median is about 1.10 ms slower, not a
performance improvement. These are separate subprocess pairs on one Linux host;
no cross-platform or statistically significant latency claim is made.

Three strict citation failures remain under unchanged labels:

| Case | Extra citations |
|---|---|
| R17, `Python import` | `package:python-fastapi-basics:python-pytest-fixture-and-import-errors` |
| R22, `EADDRINUSE address already in use` | `package:python-fastapi-basics:fastapi-port-in-use` |
| R27, `Node หาไฟล์ไม่เจอ` | `builtin:linux-command-not-found`, `builtin:git-merge-conflict` |

These expose broad import ambiguity, a portable code shared across stacks, and
shared Thai fragments/symptom wording in unrelated tools. A related investigation
note still fails a label that does not accept it; labels were not expanded. R17
also retains one additional search hit beyond the four-passage citation bound:
`package:python-fastapi-basics:fastapi-minimal-app`. The full receipt includes it
and counts it as an irrelevant search result. `ask` retains its existing four
passage bound.

The independently frozen synthetic set is reported separately in
[holdout before](evidence/retrieval-v2-holdout-before.json) and
[holdout after](evidence/retrieval-v2-holdout-after.json). No implementation changes
were made after exposing these predictions.

| Synthetic evaluation set | Before | After |
|---|---:|---:|
| Top-1 / Top-3 | 7/9 / 7/9 | 7/9 / 7/9 |
| Result precision | 7/8 (87.50%) | 7/7 (100.00%) |
| Strict citation relevance | 11/12 (91.67%) | 11/12 (91.67%) |
| Irrelevant cited passages | 1/8 (12.50%) | 1/8 (12.50%) |
| Actual emitted citation identity/locator/excerpt | 8/8 | 8/8 |
| Expected answer status | 10/12 | 10/12 |
| No-match accuracy | 3/3 | 3/3 |
| Median / P95 search + ask | 15.14 / 22.14 ms | 16.13 / 18.03 ms |

Both versions fail H03, the multiline `sqlite3.OperationalError: database is
locked` log, and H08, the multiline `go build` unused-import log. H03 returns no
adequate search result and cites the generic Python traceback playbook as
`general_guidance`, outside its specific source label. H08 abstains despite a
relevant supplied Go note. Both are missed retrievals/status errors; H08's empty
citations make its all-citations-relevant check vacuously pass. Top-K and status
metrics expose that miss. No-match accuracy only measures the three pre-labeled
negative cases and must not be read as detecting these false abstentions.

The remaining log wrappers/path/identifier tokens can overwhelm lexical coverage.
This iteration does not resolve those two misses or claim an unseen Top-K gain.
The synthetic evaluation set is now exposed regression data; future tuning needs
another frozen, unseen evaluation collection.

## Real incidents and FTS5

No authentic error-log collection was available in the repository. The report
marks real incidents unavailable, 0/50 and incomplete. The
[dataset workflow](retrieval-datasets.md) accepts retained, anonymized UTF-8 logs,
source hashes, incident IDs and provenance; `--require-real-incidents` requires
at least 50 distinct logs and incidents. A hash validates retained bytes, not
authenticity of an asserted origin. No generated example is labeled a real log.

The [offline SQLite FTS5 experiment](retrieval-fts5.md) compares lexical retrieval,
BM25 over the same eligible candidates and unrestricted FTS candidates. Candidate
source relevance is labeled a proxy; an unimplemented candidate answer policy has
no claimed citation score. The measured candidate is not promoted because it
does not demonstrate an improvement. SQLite remains an experiment, with no
production integration or forced vector/AI service.

## Verification and scope

Regression tests cover specific-vs-broad phrases, body-only evidence, exact code
boundaries, explicit IDs combined with codes, project preference, unknown codes,
uppercase prose and Python variable names. Existing save/retrieve/project/edit/
external-Markdown/learn/explain, offline provider and recovery/security workflows
remain part of the full Rust suite. The benchmark runner and FTS prototype have
separate standard-library Python unit tests.

Final checks passed: `cargo fmt --check`, `cargo test --locked` (**240 tests**),
`cargo clippy --locked --all-targets -- -D warnings`, optimized build, and
`python3 -m unittest discover -s benchmarks/retrieval -p 'test_*.py'` (**20 tests**).
The full suite includes the existing Python package install/search/explain/fix
integration test. Private-store tests and benchmark CLI processes ran outside the
filesystem sandbox after its ownership mapping rejected `/tmp`; no ownership,
privacy or path checks were weakened to obtain a pass.

Reproduce with the preserved pre-change binary (or one built from the recorded
source snapshot) and the final binary:

```bash
cargo build --locked --release
python3 benchmarks/retrieval/run.py --binary /tmp/librarycu-retrieval-before/lbc \
  --source-sha BEFORE --source-receipt docs/evidence/retrieval-v2-source-before.json \
  --baseline docs/evidence/retrieval-after.json \
  --output target/retrieval-v2-before.json
python3 benchmarks/retrieval/run.py --binary target/release/lbc \
  --source-sha BASE_REVISION+worktree --baseline target/retrieval-v2-before.json \
  --output target/retrieval-v2-after.json
python3 benchmarks/retrieval/run.py --binary /tmp/librarycu-retrieval-before/lbc \
  --source-sha BEFORE --source-receipt docs/evidence/retrieval-v2-source-before.json \
  --dataset benchmarks/retrieval/holdout.json \
  --output target/retrieval-v2-holdout-before.json
python3 benchmarks/retrieval/run.py --binary target/release/lbc \
  --source-sha BASE_REVISION+worktree --dataset benchmarks/retrieval/holdout.json \
  --baseline target/retrieval-v2-holdout-before.json \
  --output target/retrieval-v2-holdout-after.json
```

The source-receipt flag verifies exact binary identity; an independently rebuilt
binary with different bytes needs its own matching build/source receipt. The
temporary preserved executable is a local artifact, not shipped source history;
the before receipt's per-file hashes describe the already-dirty starting tree.

Only retrieval, its benchmarks and these documents changed for this task.
Pre-existing installation/onboarding/release edits were retained. Native-platform
release testing, production incident resolution, larger-corpus scaling and a
production FTS answer policy were not evaluated.
