# Retrieval reliability follow-up — 2026-11-09

This follows the [previous evaluation](retrieval-engine-improvements.md).
It fixes text parsing and evidence selection in the existing offline engine.
No dependency, persistent index, knowledge format, provider, schema or CLI command
was added. The previous 40 cases, 12-case holdout, project-memory fixtures, corpus
and all expected labels remain unchanged. The previous holdout is now exposed
regression/tuning data, not unseen evaluation evidence.

## Investigated failures

| Case | Root cause and classification | Decision |
|---|---|---|
| R17 — `Python import` | Both query terms are explicit pytest-note tags, and its import section gives legitimate Python import investigation guidance. The query does not distinguish ordinary imports from test collection. This is query ambiguity under conservative labels, not a fabricated citation. | Preserve the supporting note and the strict-label failure. Excluding pytest or lowering the passage count would guess missing context. |
| R22 — `EADDRINUSE address already in use` | Node and FastAPI notes both record the exact code and symptom. The query contains no stack. This is ambiguous shared-code evidence, not a ranking defect establishing that FastAPI is wrong. | Preserve both and the strict-label failure. Resolving it needs additional stack context and a separately validated supporting-source policy; no stack is inferred from fixture labels. |
| R27 — `Node หาไฟล์ไม่เจอ` | The tokenizer split at Thai combining marks, turning one symptom into disconnected fragments. Linux/Git notes covered 4/6 fragments, meeting the existing two-thirds adequacy gate. This is a retrieval/tokenization defect. | Preserve Thai combining marks within tokens. The correct note covers 11/11 terms; partial Linux/Git notes cover only 5/11 and fail the unchanged gate. |
| H03 — SQLite exception plus frame | The correct supplied note explicitly records both the exception and locked-database symptom. Frame paths, function names, line numbers and generic frame text inflated the query denominator. A header before its frame was not recognized as a Python diagnostic. This is query construction/recognition failure, not missing knowledge or wrong ordering among adequate results. | Recognize an exception with validated Python frame context, project its code/source/message, and retain code evidence checks. Do not invent a file location for a frame printed after the exception. |
| H08 — Go build prelude plus unused import | The relevant supplied Go note exists. Build-command text, paths and identifiers diluted term coverage until it was rejected before final evidence selection. This is retrieval/query construction failure, not insufficient evidence or ranking alone. | Use the existing compiler parser to retrieve from the Go diagnostic message and source, omitting build and location wrappers. |

The Thai mark ranges follow the
[Unicode Thai chart](https://www.unicode.org/charts/PDF/U0E00.pdf): U+0E31,
U+0E34–U+0E3A, and U+0E47–U+0E4E remain attached to words; punctuation still
separates tokens. Scalar trigrams remain the existing approximation, not a new
Thai word segmenter. No score, adequacy threshold or source-preference weight was
changed in this follow-up.

## Shared behavior and safeguards

Search and ask parse recognized diagnostics, rank each focused message with the
same index, and merge sources without duplicates in diagnostic input order.
Unstructured questions retain their complete text. Explicit source identifiers
retain their direct-lookup behavior. Explain supplies its already parsed primary
diagnostic to the same retrieval logic, preserving its existing first-diagnostic
report and count of additional diagnostics. A single diagnostic therefore uses
the same evidence in all three commands; the documented multiple-diagnostic scope
and passage limits remain different.

Focusing a log must not turn an unsupported exception into a known fix. Its code
must be recorded somewhere in the retrieved evidence. For Python, a qualified
exception can use its short class alias; for example, a namespaced decoder error
can reuse a note indexed by the short exception class. Unknown classes still
abstain or provide the existing explicitly general Python playbook. Availability
of a known class permits more specific symptom notes that do not repeat that
class literally. A broad exception's complete metadata symptom can support an
untyped note, but cannot admit a note declaring a conflicting diagnostic code.

Tests specifically protect circular-import guidance beside a recognized import
exception, unknown exceptions with identical symptom words, typed conflicting
errors, shared codes across stacks, and short body-only supporting notes.
No top-one-only citation policy or stack-specific blacklist was introduced.

Public search/ask queries keep their 8 KB raw input bound. Explain retains its
2 MB log reader and validates the focused retrieval query at 8 KB, so long logs
with short diagnostics continue to work. Private-store permissions, path checks,
secret redaction, optional-provider guards and recovery checks remain unchanged.

## Evaluation protocol

An independent agent froze
[24 new synthetic cases](../benchmarks/retrieval/reliability-evaluation.json)
before seeing implementation changes. SHA-256:
`40f05e4bcc6b757d4b868237c1996b58d274d803b3c5a74b15adec32d3473512`.
Queries are disjoint under NFKC/case-fold/whitespace normalization from both exposed
sets; those source manifest hashes are recorded in the new manifest. The same
existing corpus is shared, so this is a small synthetic generalization check,
not independent incident accuracy. No authentic log collection was available.

The pre-change release executable was copied before edits and verified against
the prior [source/binary receipt](evidence/retrieval-v2-after.json). Both binaries
use `cargo build --locked --release` with the same dependency worktree. The final
runner is unchanged. It retains complete answers, results, passages, labels,
latency samples, corpus/source/binary/runner hashes and failing predictions, and
saves reports before regression gates. Source was frozen before opening the new
evaluation predictions. No ranking decisions were made from those predictions.

## Measured results

[Implementation freeze](evidence/retrieval-reliability-freeze.json) records the
final binary, implementation, CLI test, runner and fresh manifest hashes. All
three after reports match that release binary. Per-case baseline gates pass on
all three datasets. No previously returned, label-accepted source was removed
from any case in any dataset; supporting-source preservation was checked
separately from Top-K scores. Six synthetic project-memory scenarios pass in
every run. Citation identities/locators/nonempty excerpts resolve correctly for
every emitted passage.

### Original 40-case regression set

[Before receipt](evidence/retrieval-reliability-before.json) ·
[After receipt](evidence/retrieval-reliability-after.json)

| Metric | Before | After |
|---|---:|---:|
| Top-1 / Top-3 | 31/31 / 31/31 | 31/31 / 31/31 |
| Result precision | 35/40 (87.50%) | 35/38 (92.11%) |
| Strict citation relevance | 37/40 (92.50%) | 38/40 (95.00%) |
| Irrelevant cited passages | 4/40 (10.00%) | 2/38 (5.26%) |
| Expected answer status | 40/40 | 40/40 |
| No-match accuracy | 8/8 | 8/8 |
| Median / P95 search + ask | 14.39 / 18.61 ms | 15.34 / 19.32 ms |

The two remaining strict citation failures are R17's pytest import note and R22's
FastAPI port note, for the reasons above. R27's two unrelated citations disappear
through corrected token coverage, not a blacklist or a weaker label. R17's
search-only FastAPI minimal-app hit remains irrelevant under its label and is
still included in result precision. No failure is hidden from the receipts.

### Exposed 12-case former holdout

[Before receipt](evidence/retrieval-reliability-regression-before.json) ·
[After receipt](evidence/retrieval-reliability-regression-after.json)

| Metric | Before | After |
|---|---:|---:|
| Top-1 / Top-3 | 7/9 / 7/9 | 9/9 / 9/9 |
| Result precision | 7/7 (100%) | 9/9 (100%) |
| Strict citation relevance | 11/12 | 12/12 |
| Irrelevant cited passages | 1/8 | 0/9 |
| Expected answer status | 10/12 | 12/12 |
| No-match accuracy | 3/3 | 3/3 |
| Median / P95 search + ask | 15.34 / 17.86 ms | 17.13 / 21.93 ms |

H03 now retrieves/cites `package:python-fastapi-basics:python-sqlite-database-locked`
and H08 retrieves/cites `package:retrieval-benchmark:go-unused`, each with
`retrieved_guidance`. The other ten cases retain their formerly passing checks.
In particular, the response-validation case does not gain a request-validation
citation merely because both contain a generic validation phrase.

### Fresh 24-case synthetic evaluation

[Before receipt](evidence/retrieval-reliability-evaluation-before.json) ·
[After receipt](evidence/retrieval-reliability-evaluation-after.json)

| Metric | Before | After |
|---|---:|---:|
| Top-1 / Top-3 | 13/20 / 13/20 | 19/20 / 19/20 |
| Result precision | 16/16 (100%) | 23/23 (100%) |
| Strict citation relevance | 20/24 | 24/24 |
| Irrelevant cited passages | 4/21 | 0/24 |
| Expected answer status | 17/24 | 23/24 |
| No-match accuracy | 3/3 | 3/3 |
| Median / P95 search + ask | 16.77 / 20.75 ms | 17.39 / 19.98 ms |

F07 remains a miss in both versions: the multiline Git merge-conflict log
(`Auto-merging…`, `CONFLICT (content)…`, `Automatic merge failed…`) returns no
adequate result despite an appropriate builtin note. This is another unsupported
tool-log normalization/coverage pattern, not missing corpus knowledge. It was
not used to adjust this implementation. Its empty citations pass the strict
all-citations-relevant check vacuously, while Top-K and expected-status metrics
correctly expose the false abstention. Neither 24/24 strict citation relevance nor
3/3 labeled negative-case accuracy means all positive queries are answered.

The measured median increases are approximately 0.94, 1.79 and 0.62 ms across the
three sets. These are medians of three actual search+ask subprocess pairs per
case, including startup and store reads on one Linux host. P95 is nearest-rank
over case medians. The extra parsing and focusing do not establish a speedup;
these small samples also do not isolate code cost from host noise. No large-corpus
or cross-platform performance claim is made.

## Validation and limitations

Regression tests cover combining marks and punctuation, Thai question affixes,
complete Thai/English body support, error aliases, frame/command wrappers,
unsupported classes, conflicting codes, explicit IDs, unstructured multiline
questions and multi-diagnostic preservation. The CLI tests compare actual search,
ask and explain sources and protect the different input size contracts.

Validation passed:

- `cargo test --locked`: **253 tests**, including 142 library, 75 existing CLI,
  four new CLI reliability tests, and the existing filesystem/privacy/install/
  release-configuration/user-path suites. Nine new unit tests and four new CLI
  tests were added; existing cases/assertions were preserved.
- `cargo fmt --check`.
- `cargo clippy --locked --all-targets -- -D warnings`.
- `cargo build --locked --release`.
- `python3 -m unittest discover -s benchmarks/retrieval -p 'test_*.py'`:
  **20 tests**.
- The unchanged Python CLI evaluator on all three datasets, including per-case
  source/status/citation/precision regression gates and unchanged project fixtures.

Store tests and CLI benchmarks use isolated temporary XDG directories. They ran
with actual host ownership after sandbox ownership mapping rejected the private
stores; no filesystem/security policy was relaxed.

Before committing, the exact retrieval-only staged snapshot was also exported
and checked without the unrelated local onboarding, release and dependency edits.
Its **233 Rust tests**, **20 Python tests**, formatting and strict Clippy checks
passed. The 253-test count and benchmark fingerprints above describe the earlier
complete worktree measurement; those historical receipts remain unchanged.

Reproduce the evaluations with the locally preserved baseline executable:

```bash
cargo build --locked --release
for dataset in cases holdout reliability-evaluation; do
  python3 benchmarks/retrieval/run.py \
    --binary /tmp/librarycu-reliability-before/lbc \
    --source-sha BEFORE --source-receipt docs/evidence/retrieval-v2-after.json \
    --dataset "benchmarks/retrieval/$dataset.json" \
    --output "target/reliability-$dataset-before.json"
  python3 benchmarks/retrieval/run.py --binary target/release/lbc \
    --source-sha 21a8ea21362b0f5502d357cd045cd972428037fb+worktree-reliability \
    --dataset "benchmarks/retrieval/$dataset.json" \
    --baseline "target/reliability-$dataset-before.json" \
    --output "target/reliability-$dataset-after.json"
done
```

The baseline source receipt verifies exact executable bytes before attributing
source hashes. The temporary executable is a local artifact; rebuilding a prior
dirty worktree elsewhere requires its matching source/build receipt. All raw
predictions and fingerprints are retained in the linked JSON files even without
that executable.

The remaining ambiguity cannot be resolved reliably by deleting labeled failures.
The measured fresh-set gain supports targeted diagnostic normalization as useful
work. The next useful investigation is actual tool logs (including Git), with
hard negatives, explicit context and supporting-source judgments, frozen before
implementation. More score/threshold tuning on these now-exposed synthetic sets
is unlikely to provide reliable generalization evidence. No production FTS, vector engine,
external AI dependency, corpus relabeling or real-incident claim is part of this
change. Native-platform and large-corpus performance remain unmeasured.
