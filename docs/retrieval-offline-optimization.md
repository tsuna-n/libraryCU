# Offline retrieval optimization

libraryCU now runs its knowledge workflows locally. The optimization keeps
Markdown and YAML as the source of truth and builds the lexical index in
memory from the effective documents in the selected scope.

## Audit and changes

- The baseline sent prompts through OpenAI-compatible and OpenRouter adapters.
  `ask`, `explain`, `chat`, and `fix` enabled generation with `--ai`. Setup
  and config managed providers, and `doctor` could probe their endpoints.
  The refactor removes `src/ai/`, the CLI and config paths, and the provider
  fixtures. Cargo drops `reqwest` and its HTTP/TLS dependencies, plus the
  unused `rusqlite` and `dialoguer` dependencies. Credential redaction uses
  `form_urlencoded` for encoded query keys. Tokio remains for the bounded,
  explicitly requested verification subprocess used by `apply-proposal`.
- Retrieval detects project context, resolves document scope and overrides,
  normalizes the query or diagnostic, ranks code, ID, title, metadata, and body
  matches, then applies evidence thresholds. The index now reuses its
  normalized code lookup, stops counting body occurrences at the existing
  score cap, and extracts excerpts only for selected results. Source ID and
  locator provide deterministic tie-breakers. [Index](../src/knowledge/index.rs),
  [retrieval](../src/knowledge/retrieval.rs).
- Broad runtime queries now require stronger support for tool-specific
  supplements. Exact codes, qualified IDs, complete symptom phrases, and
  comparable project matches retain their priority. The diagnostic parser
  recognizes Git content-conflict logs and focuses retrieval on the conflict.
  [Diagnostic parser](../src/diagnostics/parser.rs).
- Excerpt selection prefers the matched code and coverage of distinct query
  terms. Quotes preserve the source's internal whitespace and remain bounded
  and redacted. `ask` and `explain` omit repeated source bodies while keeping
  distinct solutions that share a problem excerpt. Inadequate matches still
  abstain. [Answer](../src/answer.rs), [explanation](../src/diagnostics/explanation.rs).
- The baseline verified checksummed packages and then read and parsed the
  Markdown again for indexing. Retrieval now parses the verified contents.
  It also avoids copying note bodies when identifying identical documents.
  The loader still validates package integrity and accepts legacy packages.
  [Loader](../src/knowledge/loader.rs), [package verification](../src/knowledge/packages.rs).

## Accuracy

Both release builds were evaluated on the same 59-document frozen corpus and
unchanged labels. The baseline was built before edits from `0dafccd` with
the existing completion and TUI work in the checkout. The benchmark runner
now passes `--new` when creating its deliberate duplicate capture fixtures
and reports p99. Each final run passed the measured baseline's per-case
gate. The original suite also passed the historical CI gate.

| Metric | Original 40 before | Original 40 after |
| --- | ---: | ---: |
| Top-1 relevance | 31/31 | 31/31 |
| Top-3 relevance | 31/31 | 31/31 |
| Returned result precision | 35/38 (92.11%) | 35/36 (97.22%) |
| Strict citation relevance | 38/40 (95.00%) | 39/40 (97.50%) |
| Irrelevant citations | 2/38 | 1/37 |
| Emitted citation identity and excerpt | 38/38 | 37/37 |
| Correct answer status | 40/40 | 40/40 |
| Correct no-match | 8/8 | 8/8 |
| Project memory | 6/6 | 6/6 |

The 12-case holdout retained 9/9 Top-1 and Top-3, 9/9 result precision, 12/12
strict citation relevance, and 3/3 correct no-matches. The 24-case reliability
evaluation rose from 19/20 to 20/20 for both Top-1 and Top-3 after recovering
a Git content conflict. It ended with 24/24 correct statuses, 24/24 strict
citation relevance, and 3/3 correct no-matches.

The original suite's remaining citation miss is an unqualified `EADDRINUSE`
query. Its label accepts the Node note, while the search also returns a
FastAPI note that covers the same error code. Exact shared codes remain
eligible across stacks when the query supplies no stack.

## Performance

Release builds used Rust 1.100.0 on Linux x86-64. The original runner starts
separate `search` and `ask` subprocesses for each repetition. The table reports
percentiles over the median of three paired repetitions per query, including
startup and store loading. With 40 cases, p99 is the slowest case median.

| Original 40, search + ask | Baseline | Offline |
| --- | ---: | ---: |
| p50 | 15.77 ms | 11.32 ms |
| p95 | 22.40 ms | 15.29 ms |
| p99 | 25.52 ms | 18.30 ms |

For per-command timing, 120 runs per command and binary were interleaved
on a fixed store. Peak RSS is the largest Linux `wait4` high-water mark
across 120 separate invocations per command and binary.

| Command | p50 before → after | p95 before → after | p99 before → after | Peak RSS before → after |
| --- | ---: | ---: | ---: | ---: |
| `search` | 10.05 → 5.64 ms | 15.28 → 9.08 ms | 19.31 → 10.69 ms | 9,380 → 6,272 KiB |
| `ask` | 9.85 → 6.05 ms | 12.74 → 9.23 ms | 14.84 → 10.90 ms | 9,352 → 6,316 KiB |

In-process profiles used 100 runs over the same installed packages and
five representative queries. The direct index search emits every scored
candidate, so this phase has a different scope from the CLI.

| Profile phase | Baseline median | Offline median |
| --- | ---: | ---: |
| Store load and validation | 2.578 ms | 1.469 ms |
| In-memory index build | 0.574 ms | 0.560 ms |
| Direct lexical search | 0.962 ms | 0.881 ms |
| Release executable size | 10,351,936 bytes | 6,960,904 bytes |

There is no on-disk retrieval index. Store loading remains the largest
measured in-process phase. The paired 10 ms median target remains open.

## Experiments and verification

- Reusing the verified package contents reduced the median store load to
  1.496 ms in an isolated experiment with the baseline dependencies.
- FTS5/BM25 reranking of the same candidates did not change Top-1, Top-3,
  or precision. Broad FTS5 returned 35/99 relevant results, regressed
  Top-1 to 30/31, and left 5/8 no-match cases empty. The production path
  remains lexical. [FTS5 experiment](retrieval-fts5.md).
- Excerpt-only deduplication was discarded during review because it could
  hide different recorded solutions with the same problem paragraph.
- Final checks passed with 342 Rust tests and 20 Python benchmark tests,
  formatting, Clippy with warnings denied, and the locked offline release
  build. The initial baseline test run hit sandbox ownership checks. With
  normal ownership, a temporary directory beneath the checkout caused the
  non-Git project test to fail. The original release binary passed the
  equivalent doctor and setup fixture outside the checkout. The final suite
  used isolated stores outside the checkout.

The datasets are small and curated or synthetic. Citation identity validates
source ID, locator, and quoted text. Strict relevance checks the sources that
were emitted. The measurements do not cover large stores or real incident
logs at scale. Native macOS and Windows execution and live external agent
sessions were outside this run. The stdio MCP and editor integration tests
passed on Linux. Existing knowledge needs no migration. The upgrade notes
list the removed flags and schema fields. [CLI upgrade notes](cli-reference.md#upgrading-to-the-offline-cli).

To repeat the measurements with a release binary and an isolated store:

```bash
cargo build --release --locked --offline
python3 benchmarks/retrieval/run.py --binary target/release/lbc \
  --source-sha "$(git rev-parse HEAD)+worktree" --output target/retrieval-benchmark/offline.json
```

Pass `--dataset benchmarks/retrieval/holdout.json` or
`--dataset benchmarks/retrieval/reliability-evaluation.json` for the other
suites. Compare matching datasets and corpus files with `--baseline`.
The runner records the tested binary and input hashes.
