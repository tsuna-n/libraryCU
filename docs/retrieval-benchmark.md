# Retrieval benchmark — v0.5, 2026-10-01

Labels were fixed before ranking changes in [cases.json](../benchmarks/retrieval/cases.json).
Dataset SHA-256: `6ba824757fca6994397873de76a7f71d3f57a8a0b2f0a56f0f00bf42e4b7a794`. The 40 cases cover Rust, Python,
TypeScript/Node and Go with English/Thai, exact codes, paraphrases, ambiguity,
unrelated input, insufficient evidence and one repeated-keyword adversary.
Every case includes expected source IDs/acceptable status sets and a rationale.
No hard case was removed and no expected result was changed after measurement.

Corpus: 12 embedded notes + 40 explicitly installed shipped Python/FastAPI notes
+ seven [synthetic reference notes](../benchmarks/retrieval/corpus/). TS/Node/Go
fixtures measure retrieval of supplied knowledge; they are not embedded product
coverage or independently verified fixes. Each fixture is marked unverified.
Both reports retain every input and corpus file hash, observed top-3/citations/
status, source revision, binary SHA-256, check date and three-run latency.

Baseline source: `08bcba665087c788a80649045fe83a887ed270a3`, built from an isolated `git archive`.
Final implementation source: `12277921b67f947a2ffbffdc525b45d11d225f91`.
[Baseline JSON](evidence/retrieval-baseline.json) and
[after JSON](evidence/retrieval-after.json) use identical dataset/corpus hashes.

| Metric | Baseline | After | Denominator / meaning |
|---|---|---|---|
| Top-1 relevance | 28/31 (90.32%) | 31/31 (100%) | 31 specific/ambiguous retrieval cases with relevant expected sources; fallback-only and no-source cases excluded |
| Top-3 relevance | 30/31 (96.77%) | 31/31 (100%) | At least one acceptable source in first three adequate search results |
| Citation identity/locator/excerpt correctness | 40/40 | 40/40 | Every emitted citation resolves to loaded identity/locator/excerpt; vacuous success for empty citations is included |
| Strict citation relevance | 24/40 (60%) | 33/40 (82.5%) | Every emitted source must belong to pre-labeled acceptable set; extra sources fail the whole case |
| Expected answer status | 40/40 | 40/40 | retrieved_guidance / general_guidance / no_adequate_match per pre-labeled case |
| Insufficient/unrelated behavior | 9/9 | 9/9 | Eight no-evidence cases abstain and one unknown Python failure explicitly general-guides |
| Median search + ask | 14.88 ms | 14.66 ms | Median of each case's three separate search+ask subprocess pairs, including startup/index/store reads |
| P95 search + ask | 18.49 ms | 18.47 ms | Nearest-rank P95 over case medians on this Linux host; not a portable performance guarantee |

Implemented corrections: deduplicate terms; use Latin/underscore word boundaries
while retaining Thai fragments; structurally prioritize exact codes, full titles
and complete metadata phrases; retain deterministic score/title/source ties;
exclude weak cross-language matches only when context is explicit (mixed language
queries stay eligible; exact codes and unclassified notes stay eligible); cite
exact-code documents and adequate code-titled user notes before weaker overlap.
No provider, LLM or vector dependency participates in evaluation.

Strict citation failures remain:

- **R13**: `coroutine was never awaited` — extra sources: `package:python-fastapi-basics:python-task-exception-not-retrieved`.

- **R14**: `cannot import name from partially initialized module` — extra sources: `package:python-fastapi-basics:python-module-not-found`.

- **R15**: `Python หา module ไม่เจอ` — extra sources: `package:python-fastapi-basics:fastapi-uvicorn-import-error`, `package:python-fastapi-basics:python-attribute-error`, `package:python-fastapi-basics:python-pytest-fixture-and-import-errors`.

- **R17**: `Python import` — extra sources: `package:python-fastapi-basics:python-pytest-fixture-and-import-errors`.

- **R22**: `EADDRINUSE address already in use` — extra sources: `package:python-fastapi-basics:fastapi-port-in-use`.

- **R27**: `Node หาไฟล์ไม่เจอ` — extra sources: `builtin:git-merge-conflict`, `builtin:linux-command-not-found`.

- **R31**: `module resolution` — extra sources: `builtin:rust-e0433`.

Several extras are related investigation notes or the same portable error code
in another stack (EADDRINUSE). The conservative labels still count them as
failures; they were not expanded to raise the score. This exposes residual
lexical ambiguity and overly broad supplemental citations, not fabricated source
identity. R17 is deliberately ambiguous and not proof of a specific fix.
A generic traceback is investigation guidance, never adequate evidence for AI
patching. Neither Top-K percentages nor passing tests imply 100% product
accuracy, actual incident resolution, code correctness or production readiness.
This small mostly curated corpus is not an independent field-incident study.

Reproduce outside the real user stores (the runner isolates projects/XDG/env):

```bash
cargo build --locked --release
python3 benchmarks/retrieval/run.py --binary target/release/lbc \
  --source-sha "$(git rev-parse HEAD)" \
  --baseline docs/evidence/retrieval-baseline.json \
  --output target/retrieval-benchmark/results.json
```

Linux candidate CI runs the same command and retains its JSON as a separate
retrieval-benchmark artifact, with exact CIRCLE_SHA1. It rejects changed dataset/
corpus hashes and each formerly passing case/metric that regresses, not merely
an improving aggregate. Baseline comparisons never silently relabel inputs.
Latency is measured/reported and is not used as a noisy machine-dependent gate.
