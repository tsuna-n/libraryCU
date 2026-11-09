# Retrieval dataset policy

The benchmark measures retrieval of supplied local evidence, not whether a fix
resolved a production incident. No authentic incident-log collection was available
for this change. The real-incident gate is therefore **unavailable / incomplete**;
synthetic cases must never be counted as authentic logs.

The existing [40 cases](../benchmarks/retrieval/cases.json), corpus and
[six project-memory cases](../benchmarks/retrieval/project-memory.json) remain
unchanged. The original cases are an exposed tuning/regression set. The project
cases remain synthetic regressions for project/user preference, isolation and
qualified IDs. Original case SHA-256:
`6ba824757fca6994397873de76a7f71d3f57a8a0b2f0a56f0f00bf42e4b7a794`.

The separate [12-case holdout](../benchmarks/retrieval/holdout.json) is explicitly
synthetic, frozen before the ranking implementation by an independent agent using
existing supplied notes and known boundary failure modes. It contains multiline
compiler/runtime wrappers, additional Python package errors, qualified source IDs,
near-code negatives and unrelated input. SHA-256:
`f0970ca06b061fce20baae07a40f44541866514a2ee192f9334c56509a9a8aac`.
Its labels must not be changed to improve scores. Do not tune this iteration from
its failures. After results are exposed, it becomes a regression set for future
work; select and freeze another unseen evaluation set before the next tuning run.
Shared relevant document IDs are allowed across splits; identical normalized
queries and incident provenance are not. This small related synthetic set does
not establish generalization to real incidents.

Use the same binary build, dataset, corpus and runner for before/after comparison:

```bash
python3 benchmarks/retrieval/run.py --binary target/debug/lbc \
  --source-sha SOURCE+worktree \
  --output target/retrieval/original.json
python3 benchmarks/retrieval/run.py --binary target/debug/lbc \
  --dataset benchmarks/retrieval/holdout.json \
  --source-sha SOURCE+worktree \
  --output target/retrieval/holdout.json
python3 -m unittest discover -s benchmarks/retrieval -p test_run.py
```

`--baseline PATH` rejects incompatible dataset/corpus hashes, changed case labels,
formerly passing case metrics, and per-case result-precision or irrelevant-citation
rate regressions when the old report contains those counters. Reports are written
before these regression gates run, so failing predictions remain visible. The
unchanged project-memory fixtures still run for every dataset. Latency is reported
without a machine-dependent acceptance threshold.

When testing a preserved old binary after editing the workspace, use
`--source-receipt path/to/original-report.json`. The runner first checks that the
receipt's binary hash matches the tested binary, then reuses its captured source
SHA and implementation-file hashes. The report identifies the receipt and its
hash. Without this option, implementation hashes explicitly describe workspace
files at run time; the build process must establish correspondence to the binary.
The runner hash always describes the current benchmark code.

## Adding authentic logs

Collect at least 50 distinct developer error logs with permission to retain them.
Anonymize secrets, personal data and private paths before freezing the retained
files. Record the actual origin and stable incident IDs; a hash proves that a
retained file did not change, not that its reported origin is authentic. Do not
manufacture incidents, relabel generated examples as real, duplicate one log with
many query paraphrases, or claim a resolved incident from a retrieval hit.

Create a separate JSON manifest with this shape (the example below is a schema
illustration, **not an incident or an executable dataset**):

```json
{
  "schema": 1,
  "split": "evaluation",
  "provenance": {
    "kind": "real_incidents",
    "id": "collection-and-frozen-version",
    "origin": "Actual collection source, dates and anonymization process"
  },
  "cases": [
    {
      "id": "stable-case-id",
      "language": "actual-language",
      "kind": "exact",
      "query": "An actual retained log excerpt",
      "expected_sources": ["qualified:existing-source-id"],
      "acceptable_statuses": ["retrieved_guidance"],
      "rationale": "Why this evidence supports the observed diagnostic",
      "source": {
        "path": "logs/actual-log.txt",
        "sha256": "SHA-256 of the retained anonymized UTF-8 file",
        "incident_id": "stable-original-incident-id"
      }
    }
  ]
}
```

Source paths resolve relative to the manifest. Each real case must refer to an
existing UTF-8 log with the exact SHA-256, and its query must occur in the log after
Unicode NFKC normalization, case-folding and whitespace collapse. Expected source
IDs must exist in the benchmark's loaded builtin and installed-package corpus;
use an empty list and `no_adequate_match` when evidence is absent. Arbitrary new
project corpora require separately reviewed fixtures rather than changing the
frozen original corpus.

Keep tuning and evaluation manifests separate and freeze labels before ranking
work. Pass `--tuning-dataset path/to/tuning.json` to check against an additional
collection as well as the original 40. That manifest must declare
`split: "tuning"`. The original 40 are always checked. The runner rejects overlap in normalized query, collection ID,
source hash or incident ID; human review must also reject semantic duplicates and
related exports that these exact comparisons cannot detect.

```bash
python3 benchmarks/retrieval/run.py --binary target/debug/lbc \
  --dataset path/to/evaluation.json --tuning-dataset path/to/tuning.json \
  --require-real-incidents --source-sha SOURCE \
  --output target/retrieval/real-evaluation.json
```

The gate requires at least 50 distinct source hashes **and** 50 distinct incident
IDs, all backed by validated retained logs. Smaller real collections can be run
without `--require-real-incidents`, but reports explicitly mark them incomplete.
Synthetic/curated datasets report zero available real logs. No empty real manifest
is shipped because there is no actual collection to represent.

## Metrics and retained evidence

Every report retains input/corpus/binary/runner hashes, split and provenance, each query
and frozen label, full search results and passages, all three duration samples,
and every prediction including failures. The metrics are:

- Top-1 and Top-3 relevance over labeled retrieval cases, excluding fallback-only
  and no-source cases.
- Result precision: relevant returned results / all returned results.
- Strict citation relevance: cases where every emitted citation belongs to the
  pre-labeled acceptable source set; irrelevant-citation rate counts individual
  emitted sources outside that set.
- Citation identity: loaded source ID, exact locator, nonempty excerpt contained
  in the loaded document. A retrieved/general answer with no passages fails the
  per-case identity check. `emitted_citation_identity` counts only actual emitted
  passages and has a null percentage when none exist; no-match cases cannot
  inflate its denominator.
- Expected answer status, insufficient/unrelated behavior, and no-match correctness
  (expected abstentions return `no_adequate_match` with no citations or search results).
- Project-memory pass count, and median/P95 over each case's median of three
  search+ask subprocess pairs, including startup and store reads.

Do not pool synthetic and authentic results into a claim of real-world accuracy.
Keep dated before/after reports, include failures, and distinguish ranking tuning
from fresh evaluation in any reported conclusion.
