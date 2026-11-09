#!/usr/bin/env python3
"""Offline FTS5/BM25 experiment; never changes the production index or labels."""
import argparse
from datetime import datetime, timezone
import hashlib
import json
import os
from pathlib import Path
import sqlite3
import statistics
import subprocess
import tempfile
import time

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def fts_query(query):
    # Quoted phrases keep user operators/punctuation out of the MATCH grammar.
    return ' OR '.join('"' + token.replace('"', '""') + '"'
                       for token in dict.fromkeys(query.lower().split()))


def build_index(documents):
    db = sqlite3.connect(':memory:')
    db.execute("CREATE VIRTUAL TABLE notes USING fts5(source_id UNINDEXED, title, metadata, body, tokenize='unicode61')")
    for doc in documents:
        metadata = doc['metadata']
        fields = [metadata.get(key) or '' for key in
                  ['id', 'title_th', 'language', 'tool', 'framework', 'category', 'error_code']]
        fields += metadata.get('tags', []) + metadata.get('keywords', [])
        db.execute('INSERT INTO notes VALUES (?, ?, ?, ?)',
                   (doc['source_id'], doc['title'], ' '.join(fields), doc['body']))
    db.commit()
    return db


def search_fts(db, query):
    expression = fts_query(query)
    if not expression:
        return []
    return [dict(source_id=source_id, bm25=score) for source_id, score in db.execute(
        'SELECT source_id, bm25(notes, 0, 5, 3, 1) AS score FROM notes '
        'WHERE notes MATCH ? ORDER BY score, title, source_id', (expression,))]


def rerank(candidates, hits):
    scores = {hit['source_id']: hit['bm25'] for hit in hits}
    return sorted(candidates, key=lambda source: scores.get(source, float('inf')))


def ratio(numerator, denominator):
    return dict(numerator=numerator, denominator=denominator,
                percent=round(100 * numerator / denominator, 2) if denominator else None)


def latency(values, scope):
    ordered = sorted(values)
    return dict(unit='milliseconds', scope=scope, median=statistics.median(ordered),
                p95=ordered[max(0, (95 * len(ordered) + 99) // 100 - 1)])


def retrieval_metrics(rows, method):
    specific = [row for row in rows if row['measurable']]
    no_match = [row for row in rows if row['acceptable_statuses'] == ['no_adequate_match']]
    return dict(
        top1=ratio(sum(bool(row[method]) and row[method][0] in row['expected_sources'] for row in specific), len(specific)),
        top3=ratio(sum(bool(set(row[method][:3]) & set(row['expected_sources'])) for row in specific), len(specific)),
        result_precision=ratio(sum(source in row['expected_sources'] for row in rows for source in row[method]),
                               sum(len(row[method]) for row in rows)),
        top3_source_relevance_proxy=ratio(sum(all(source in row['expected_sources'] for source in row[method][:3]) for row in rows), len(rows)),
        no_match_candidate_abstention_proxy=ratio(sum(not row[method] for row in no_match), len(no_match)))


def run(binary, output):
    dataset_path = ROOT / 'benchmarks/retrieval/cases.json'
    cases = json.loads(dataset_path.read_text())['cases']
    corpus_hashes = {str(path.relative_to(ROOT)): digest(path)
        for base in ['knowledge', 'packages/python-fastapi-basics', 'benchmarks/retrieval/corpus']
        for path in sorted((ROOT / base).rglob('*')) if path.is_file()}
    frozen_path = ROOT / 'docs/evidence/retrieval-after.json'
    frozen = json.loads(frozen_path.read_text())
    if digest(dataset_path) != frozen['dataset_sha256'] or corpus_hashes != frozen['corpus_sha256']:
        raise ValueError('Frozen labels/corpus changed; this experiment requires the original benchmark hashes')
    rows = []
    with tempfile.TemporaryDirectory(prefix='lbc-fts5-') as temp:
        home = Path(temp)
        env = dict(os.environ)
        for key in ['LBC_CONFIG', 'OPENAI_API_KEY', 'OPENROUTER_API_KEY', 'ZAI_API_KEY', 'GLM_API_KEY', 'OLLAMA_API_KEY']:
            env.pop(key, None)
        env.update(XDG_DATA_HOME=str(home / 'data'), XDG_CONFIG_HOME=str(home / 'config'),
                   XDG_CACHE_HOME=str(home / 'cache'))

        def cli(*args):
            result = subprocess.run([str(binary), *args], cwd=home, env=env,
                                    text=True, capture_output=True, timeout=60)
            if result.returncode:
                raise RuntimeError(f'{args[0]} failed: {result.stderr}')
            return json.loads(result.stdout)

        for package in ['packages/python-fastapi-basics', 'benchmarks/retrieval/corpus']:
            # Install reports are text, unlike the retrieval commands.
            result = subprocess.run([str(binary), 'knowledge', 'install', str(ROOT / package)],
                                    cwd=home, env=env, text=True, capture_output=True, timeout=60)
            if result.returncode:
                raise RuntimeError(f'knowledge install failed: {result.stderr}')
        documents = {doc['source_id']: cli('inspect', doc['source_id'], '--json')
                     for doc in cli('list', '--json')}
        started = time.perf_counter()
        db = build_index(documents.values())
        build_ms = (time.perf_counter() - started) * 1000
        try:
            for case in cases:
                expected = set(case['expected_sources'])
                if not expected <= documents.keys():
                    raise ValueError(f"{case['id']}: expected source missing from loaded corpus")
                cli_times, fts_times = [], []
                for _ in range(3):
                    started = time.perf_counter()
                    results = cli('search', case['query'], '--json')
                    answer = cli('ask', case['query'], '--json')
                    cli_times.append((time.perf_counter() - started) * 1000)
                    ids = [result['source_id'] for result in results]
                    started = time.perf_counter()
                    hits = search_fts(db, case['query'])
                    ranked = rerank(ids, hits)
                    fts_times.append((time.perf_counter() - started) * 1000)
                passages = answer['passages']
                cited = [passage['source_id'] for passage in passages]
                rows.append(dict(
                    id=case['id'], query=case['query'], kind=case['kind'],
                    expected_sources=case['expected_sources'], acceptable_statuses=case['acceptable_statuses'],
                    measurable=bool(expected) and case['acceptable_statuses'] != ['general_guidance'],
                    lexical=ids, bm25_rerank=ranked, fts_broad_top3=[hit['source_id'] for hit in hits[:3]],
                    fts_all_hits=hits, answer_status=answer['answer_status'], cited_sources=cited,
                    status_correct=answer['answer_status'] in case['acceptable_statuses'],
                    citation_identity_correct=all(p['source_id'] in documents
                        and p['source_locator'] == documents[p['source_id']]['path']
                        and p['excerpt'] in documents[p['source_id']]['body'] for p in passages),
                    citation_relevance_correct=all(source in expected for source in cited),
                    irrelevant_citations=sum(source not in expected for source in cited),
                    median_cli_search_ask_ms=statistics.median(cli_times),
                    median_in_process_fts_rerank_ms=statistics.median(fts_times)))
        finally:
            db.close()
    methods = {method: retrieval_metrics(rows, method) for method in ['lexical', 'bm25_rerank', 'fts_broad_top3']}
    regressions = [row['id'] for row in rows if row['measurable'] and row['lexical']
                   and row['lexical'][0] in row['expected_sources']
                   and row['bm25_rerank'][0] not in row['expected_sources']]
    improvements = [row['id'] for row in rows if row['measurable'] and row['bm25_rerank']
                    and row['bm25_rerank'][0] in row['expected_sources']
                    and (not row['lexical'] or row['lexical'][0] not in row['expected_sources'])]
    report = dict(
        schema=1, checked_at=datetime.now(timezone.utc).isoformat(), binary=str(binary),
        binary_sha256=digest(binary), sqlite_version=sqlite3.sqlite_version,
        experiment_sha256=digest(Path(__file__)), dataset_sha256=digest(dataset_path),
        frozen_reference_sha256=digest(frozen_path), corpus_sha256=corpus_hashes,
        loaded_document_count=len(documents),
        loaded_documents_sha256=hashlib.sha256(json.dumps(
            [{key: doc[key] for key in ['source_id', 'title', 'metadata', 'body']}
             for _, doc in sorted(documents.items())], sort_keys=True, ensure_ascii=False).encode()).hexdigest(),
        configuration=dict(tokenizer='unicode61', query='deduplicated lowercased whitespace phrases joined with OR',
                           bm25_weights=dict(title=5, metadata=3, body=1), broad_limit=3,
                           rerank='same adequate CLI search candidates; missing/tied BM25 scores retain CLI order'),
        metrics=methods,
        actual_cli_answers=dict(
            status_correct=ratio(sum(row['status_correct'] for row in rows), len(rows)),
            no_match_correctness=ratio(sum(row['status_correct'] and not row['cited_sources'] for row in rows
                if row['acceptable_statuses'] == ['no_adequate_match']),
                sum(row['acceptable_statuses'] == ['no_adequate_match'] for row in rows)),
            strict_citation_relevance=ratio(sum(row['citation_relevance_correct'] for row in rows), len(rows)),
            citation_identity_correct=ratio(sum(row['citation_identity_correct'] for row in rows), len(rows)),
            irrelevant_citation_rate=ratio(sum(row['irrelevant_citations'] for row in rows), sum(len(row['cited_sources']) for row in rows))),
        candidate_answer_citation_metrics=None,
        limitations=[
            'FTS candidates have no answer/evidence policy; top3 source relevance and candidate abstention are proxies, not measured answer citations/statuses.',
            'Reranking preserves candidate membership and CLI adequacy; result precision and candidate abstention cannot improve by construction.',
            'Broad FTS is diagnostic only: no adequacy gate, exact-code override, context filter or project priority.',
            'FTS latency is warm in-process query plus reranking, excluding index construction and CLI/store startup; never a CLI speedup claim.',
            'Only original fixed 40 cases; project-memory, large corpus, native platforms and independent incidents are not evaluated.'],
        latency=dict(
            cli_search_ask=latency([row['median_cli_search_ask_ms'] for row in rows], 'three repetitions, separate actual search + ask subprocesses, startup/store reads included'),
            in_process_fts_rerank=latency([row['median_in_process_fts_rerank_ms'] for row in rows], 'three repetitions, warm SQLite MATCH/BM25 plus Python candidate reranking; excludes index build, CLI/store startup and answer construction'),
            in_memory_index_build_ms=build_ms),
        decision=dict(promote=False, top1_regressions=regressions, top1_improvements=improvements,
                      reason='Offline ranking experiment only; promotion requires a measured gain without case regressions and validated answer/project policies.'),
        cases=rows)
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(report, indent=2, ensure_ascii=False) + '\n')
    print(json.dumps({key: report[key] for key in ['metrics', 'actual_cli_answers', 'latency', 'decision']}, indent=2))
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    run(args.binary.resolve(), args.output)
