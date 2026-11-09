#!/usr/bin/env python3
"""Evaluate fixed labels with isolated stores through the actual CLI; never contacts AI."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import statistics
import subprocess
import tempfile
import time
import unicodedata
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[2]
DEFAULT_DATASET = ROOT / "benchmarks/retrieval/cases.json"
CHECK_FIELDS = ["top1_relevant", "top3_relevant", "status_correct",
    "citation_identity_correct", "citation_relevance_correct", "no_match_correct"]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def source_evidence(binary, source_sha, receipt_path=None):
    binary_hash = digest(binary)
    if receipt_path:
        receipt = json.loads(receipt_path.read_text())
        if receipt.get('binary_sha256') != binary_hash:
            raise ValueError('Source receipt binary hash does not match the tested binary')
        if not receipt.get('source_sha') or not isinstance(receipt.get('implementation_sha256'), dict) or not receipt['implementation_sha256']:
            raise ValueError('Source receipt requires source_sha and implementation_sha256')
        return dict(source_sha=receipt['source_sha'], binary_sha256=binary_hash,
            implementation_sha256=receipt['implementation_sha256'],
            implementation_snapshot_origin=dict(kind='receipt', path=str(receipt_path), sha256=digest(receipt_path)))
    return dict(source_sha=source_sha, binary_sha256=binary_hash,
        implementation_sha256={str(p.relative_to(ROOT)): digest(p) for p in sorted((ROOT / 'src').rglob('*.rs'))},
        implementation_snapshot_origin=dict(kind='workspace', path=str(ROOT),
            scope='Source files at run time; correspondence to the binary must be established by the build receipt'))


def normalized_query(query):
    return ' '.join(unicodedata.normalize('NFKC', query).casefold().split())


def load_dataset(path):
    path = path.resolve()
    dataset = json.loads(path.read_text())
    if path == DEFAULT_DATASET.resolve():
        # Existing labels remain byte-for-byte frozen; metadata is report-only.
        dataset.setdefault('split', 'tuning')
        dataset.setdefault('provenance', {'kind': 'curated', 'id': 'original-40'})
    provenance = dataset.get('provenance', {})
    if not isinstance(provenance, dict):
        raise ValueError('Dataset provenance must be an object')
    if dataset.get('schema') != 1 or dataset.get('split') not in ['evaluation', 'tuning']:
        raise ValueError('Dataset requires schema=1 and explicit evaluation/tuning split')
    if provenance.get('kind') not in ['synthetic', 'curated', 'real_incidents'] or not provenance.get('id'):
        raise ValueError('Dataset requires explicit provenance kind and id')
    if not isinstance(dataset.get('cases'), list) or not dataset['cases']:
        raise ValueError('Dataset must contain at least one case')
    seen_ids, seen_queries = set(), set()
    for case in dataset['cases']:
        if not isinstance(case, dict):
            raise ValueError('Case must be an object')
        query = case.get('query')
        if not isinstance(query, str) or not query.strip() or not isinstance(case.get('id'), str) or not case['id']:
            raise ValueError('Case requires nonempty id and query')
        normalized = normalized_query(query)
        if case['id'] in seen_ids or normalized in seen_queries:
            raise ValueError('Duplicate case id or normalized query')
        seen_ids.add(case['id'])
        seen_queries.add(normalized)
        if not isinstance(case.get('expected_sources'), list) or not all(isinstance(x, str) and x for x in case['expected_sources']):
            raise ValueError(f"{case['id']}: expected_sources must be source IDs")
        statuses = case.get('acceptable_statuses')
        if not isinstance(statuses, list) or not statuses or not set(statuses) <= {'retrieved_guidance', 'general_guidance', 'no_adequate_match'}:
            raise ValueError(f"{case['id']}: invalid acceptable_statuses")
        if provenance['kind'] == 'real_incidents':
            source = case.get('source', {})
            if not isinstance(source, dict):
                raise ValueError(f"{case['id']}: source must be an object")
            if not provenance.get('origin') or not all(isinstance(source.get(k), str) and source[k].strip() for k in ['path', 'sha256', 'incident_id']):
                raise ValueError(f"{case['id']}: real incidents require origin and source path/sha256/incident_id")
            source_path = path.parent / source['path']
            if not source_path.is_file() or digest(source_path) != source['sha256']:
                raise ValueError(f"{case['id']}: real log source is absent or hash differs")
            if normalized not in normalized_query(source_path.read_text()):
                raise ValueError(f"{case['id']}: query is absent from retained real log")
    return dataset


def validate_disjoint(evaluation, tuning):
    queries = {normalized_query(c['query']) for c in evaluation['cases']}
    other_queries = {normalized_query(c['query']) for c in tuning['cases']}
    def origins(dataset):
        return {(key, c['source'][key]) for c in dataset['cases'] if 'source' in c
            for key in ['sha256', 'incident_id'] if key in c['source']}
    if (queries & other_queries or origins(evaluation) & origins(tuning)
            or evaluation['provenance']['id'] == tuning['provenance']['id']):
        raise ValueError('Tuning/evaluation overlap in normalized query or provenance')


def incident_evidence(dataset):
    real = dataset['provenance']['kind'] == 'real_incidents'
    logs = {c['source']['sha256'] for c in dataset['cases']} if real else set()
    incidents = {c['source']['incident_id'] for c in dataset['cases']} if real else set()
    count = min(len(logs), len(incidents))
    return dict(available=real, verified_logs=count, required_logs=50, complete=count >= 50,
        scope='Local source existence, SHA-256 and query inclusion checked; authenticity is asserted by the supplied provenance, not independently established')


def score_case(case, results, answer, documents, durations):
    ids = [r['source_id'] for r in results]
    expected = set(case['expected_sources'])
    measurable = bool(expected) and case['acceptable_statuses'] != ['general_guidance']
    citations = answer['passages']
    identities = [
        p['source_id'] in documents and p['source_locator'] == documents[p['source_id']]['path']
        and bool(p['excerpt'].strip()) and p['excerpt'] in documents[p['source_id']]['body'] for p in citations]
    identity_ok = (bool(citations) or answer['answer_status'] == 'no_adequate_match') and all(identities)
    relevance_ok = all(p['source_id'] in expected for p in citations) if expected else not citations
    no_match = (answer['answer_status'] == 'no_adequate_match' and not citations and not ids
        if case['acceptable_statuses'] == ['no_adequate_match'] else None)
    return dict(id=case['id'], query=case['query'], expected_sources=case['expected_sources'],
        acceptable_statuses=case['acceptable_statuses'], rationale=case.get('rationale'), source=case.get('source'),
        retrieved_sources=ids, results=results, relevant_results=sum(source in expected for source in ids), result_count=len(ids),
        irrelevant_citations=sum(p['source_id'] not in expected for p in citations), citation_count=len(citations),
        top3=ids[:3], top1_relevant=(bool(ids) and ids[0] in expected) if measurable else None,
        top3_relevant=bool(expected.intersection(ids[:3])) if measurable else None,
        answer_status=answer['answer_status'], status_correct=answer['answer_status'] in case['acceptable_statuses'],
        no_match_correct=no_match, answer=answer, valid_citation_identities=sum(identities), citations=citations, cited_sources=[p['source_id'] for p in citations], citation_identity_correct=identity_ok,
        citation_relevance_correct=relevance_ok, search_ask_ms=durations, median_search_ask_ms=statistics.median(durations))


def check_regressions(previous, report):
    if previous['dataset_sha256'] != report['dataset_sha256'] or previous['corpus_sha256'] != report['corpus_sha256']:
        raise ValueError('Dataset/corpus changed; do not silently relabel or compare incompatible runs')
    for old, new in zip(previous['cases'], report['cases'], strict=True):
        if any(old.get(k) != new.get(k) for k in ['id', 'query', 'expected_sources']):
            raise ValueError('Benchmark labels/order changed')
        if 'acceptable_statuses' in old and old['acceptable_statuses'] != new['acceptable_statuses']:
            raise ValueError('Benchmark status labels changed')
        for field in CHECK_FIELDS:
            if old.get(field) is True and new.get(field) is not True:
                raise AssertionError(f"{new['id']}: regression in {field}")
        if old.get('result_count', 0) and new.get('result_count', 0):
            if new['relevant_results'] * old['result_count'] < old['relevant_results'] * new['result_count']:
                raise AssertionError(f"{new['id']}: regression in result precision")
        if old.get('citation_count', 0) and new.get('citation_count', 0):
            if new['irrelevant_citations'] * old['citation_count'] > old['irrelevant_citations'] * new['citation_count']:
                raise AssertionError(f"{new['id']}: regression in irrelevant citation rate")
    if 'project_memory' in previous and previous['project_memory']['dataset_sha256'] != report['project_memory']['dataset_sha256']:
        raise ValueError('Project memory labels changed')


def run(binary, destination, source_sha, baseline=None, dataset_path=None, tuning_dataset=None, require_real_incidents=False, source_receipt=None):
    dataset_path = (dataset_path or DEFAULT_DATASET).resolve()
    dataset = load_dataset(dataset_path)
    tuning_path = tuning_dataset or DEFAULT_DATASET
    tuning_paths = [DEFAULT_DATASET] + ([tuning_dataset] if tuning_dataset and tuning_dataset.resolve() != DEFAULT_DATASET.resolve() else [])
    if dataset['split'] == 'evaluation':
        for path in tuning_paths:
            tuning = load_dataset(path)
            if tuning['split'] != 'tuning':
                raise ValueError('--tuning-dataset must declare split=tuning')
            validate_disjoint(dataset, tuning)
    elif tuning_dataset:
        raise ValueError('--tuning-dataset requires an evaluation dataset')
    incident_summary = incident_evidence(dataset)
    if require_real_incidents and not incident_summary['complete']:
        raise ValueError('A complete real-incident benchmark requires at least 50 distinct retained logs and incident IDs')
    source = source_evidence(binary, source_sha, source_receipt)
    rows = []
    with tempfile.TemporaryDirectory(prefix='lbc-retrieval-') as temp:
        home = Path(temp)
        env = dict(os.environ)
        for key in ['LBC_CONFIG', 'OPENAI_API_KEY', 'OPENROUTER_API_KEY', 'ZAI_API_KEY', 'GLM_API_KEY', 'OLLAMA_API_KEY']:
            env.pop(key, None)
        env.update(XDG_DATA_HOME=str(home / 'data'), XDG_CONFIG_HOME=str(home / 'config'), XDG_CACHE_HOME=str(home / 'cache'))
        def cli(*args):
            result = subprocess.run([str(binary), *args], cwd=home, env=env, text=True, capture_output=True, timeout=60)
            if result.returncode:
                raise RuntimeError(f'{args[0]} failed: {result.stderr}')
            return result.stdout
        for package in ['packages/python-fastapi-basics', 'benchmarks/retrieval/corpus']:
            cli('knowledge', 'install', str(ROOT / package))
        documents = {d['source_id']: json.loads(cli('inspect', d['source_id'], '--json')) for d in json.loads(cli('list', '--json'))}
        for case in dataset['cases']:
            unknown = set(case['expected_sources']) - documents.keys()
            if unknown:
                raise ValueError(f"{case['id']}: labels reference absent sources: {unknown}")
            durations = []
            for _ in range(3):
                started = time.perf_counter()
                results = json.loads(cli('search', case['query'], '--json'))
                answer = json.loads(cli('ask', case['query'], '--json'))
                durations.append((time.perf_counter() - started) * 1000)
            rows.append(score_case(case, results, answer, documents, durations))
        # Separate frozen synthetic project fixtures; never relabel the original corpus.
        memory_path = ROOT / 'benchmarks/retrieval/project-memory.json'
        memory = json.loads(memory_path.read_text())
        for name in ['backend', 'other']:
            (home / name).mkdir()
        for capture in memory['captures']:
            args = ['learn', '--yes', '--id', capture['id'], '--problem', capture['problem'],
                '--solution', capture['solution'], '--verification', capture['verification'], '--json']
            args += ['--user'] if capture['destination'] == 'user' else ['--project', str(home / capture['destination'])]
            cli(*args)
        memory_rows = []
        for case in memory['cases']:
            durations = []
            for _ in range(3):
                started = time.perf_counter()
                results = json.loads(cli('search', case['query'], '--project', str(home / case['project']), '--json'))
                answer = json.loads(cli('ask', case['query'], '--project', str(home / case['project']), '--json'))
                durations.append((time.perf_counter() - started) * 1000)
            top1 = results[0]['source_id'] if results else None
            passed = top1 == case['expected_top1'] and (
                bool(answer['passages']) and answer['passages'][0]['source_id'] == top1
                if top1 else answer['answer_status'] == 'no_adequate_match' and not answer['passages'])
            memory_rows.append(dict(**case, observed_top1=top1, passed=passed,
                median_search_ask_ms=statistics.median(durations)))
    def metric(field):
        values = [r[field] for r in rows if r[field] is not None]
        return dict(passed=sum(values), total=len(values), percent=round(100 * sum(values) / len(values), 2) if values else None)
    abstention = [r for r,c in zip(rows,dataset['cases']) if c.get('kind') in ['insufficient','unrelated']]
    latencies = sorted(r['median_search_ask_ms'] for r in rows)
    no_matches = [r for r,c in zip(rows,dataset['cases']) if c['acceptable_statuses'] == ['no_adequate_match']]
    def ratio(numerator, denominator):
        return dict(numerator=numerator, denominator=denominator,
            percent=round(100 * numerator / denominator, 2) if denominator else None)
    report = dict(**source, checked_at=datetime.now(timezone.utc).isoformat(),
        runner_sha256=digest(Path(__file__)), dataset_sha256=digest(dataset_path),
        dataset=dict(split=dataset['split'], provenance=dataset['provenance'], description=dataset.get('description'),
            tuning_dataset_sha256=digest(tuning_path) if dataset['split'] == 'evaluation' else None,
            excluded_tuning_datasets=[dict(path=str(p), sha256=digest(p)) for p in tuning_paths] if dataset['split'] == 'evaluation' else []),
        real_incidents=incident_summary,
        corpus_sha256={str(p.relative_to(ROOT)):digest(p) for base in ['knowledge','packages/python-fastapi-basics','benchmarks/retrieval/corpus'] for p in sorted((ROOT/base).rglob('*')) if p.is_file()},
        metrics={k:metric(k) for k in ['top1_relevant','top3_relevant','status_correct','citation_identity_correct','citation_relevance_correct']},
        insufficient_behavior=dict(passed=sum(r['status_correct'] for r in abstention), total=len(abstention)),
        result_precision=ratio(sum(r['relevant_results'] for r in rows), sum(r['result_count'] for r in rows)),
        emitted_citation_identity=ratio(sum(r['valid_citation_identities'] for r in rows), sum(r['citation_count'] for r in rows)),
        irrelevant_citation_rate=ratio(sum(r['irrelevant_citations'] for r in rows), sum(r['citation_count'] for r in rows)),
        no_match_correctness=ratio(sum(r['no_match_correct'] for r in no_matches), len(no_matches)),
        project_memory=dict(dataset_sha256=digest(memory_path), passed=sum(r['passed'] for r in memory_rows),
            total=len(memory_rows), cases=memory_rows),
        latency=dict(unit='milliseconds',scope='three repetitions, median of separate search + ask subprocesses; includes startup/store reads',
            median=statistics.median(latencies),p95=latencies[max(0, (95*len(latencies)+99)//100-1)]),cases=rows)
    destination.parent.mkdir(parents=True,exist_ok=True)
    destination.write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps({k:report[k] for k in ['source_sha','dataset_sha256','metrics','insufficient_behavior',
        'result_precision','emitted_citation_identity','irrelevant_citation_rate','no_match_correctness','real_incidents','latency']},indent=2))
    print(f"Synthetic project memory: {report['project_memory']['passed']}/{report['project_memory']['total']}")
    if not all(row['passed'] for row in memory_rows):
        raise AssertionError('Project memory regression; inspect the retained case report')
    if baseline:
        check_regressions(json.loads(baseline.read_text()), report)
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--source-sha', required=True, help='Exact tested source SHA; append +worktree when uncommitted')
    parser.add_argument('--baseline', type=Path, help='Reject per-case regressions and incompatible labels/corpus')
    parser.add_argument('--dataset', type=Path, help='Alternate frozen labeled dataset with explicit split/provenance')
    parser.add_argument('--tuning-dataset', type=Path, help='Additional tuning manifest to exclude from evaluation (original 40 always checked)')
    parser.add_argument('--require-real-incidents', action='store_true', help='Require at least 50 distinct source-checked real logs')
    parser.add_argument('--source-receipt', type=Path, help='Prior report for this exact binary; reuse its captured source SHA and source-file hashes')
    args = parser.parse_args()
    run(args.binary.resolve(), args.output, args.source_sha, args.baseline, args.dataset, args.tuning_dataset, args.require_real_incidents, args.source_receipt)
