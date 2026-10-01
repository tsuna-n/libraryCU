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
from datetime import datetime, timezone

ROOT = Path(__file__).resolve().parents[2]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def run(binary, destination, source_sha, baseline=None):
    dataset_path = ROOT / 'benchmarks/retrieval/cases.json'
    dataset = json.loads(dataset_path.read_text())
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
            ids = [r['source_id'] for r in results]
            expected = set(case['expected_sources'])
            # Fallback-only cases intentionally return no search result.
            measurable = bool(expected) and case['acceptable_statuses'] != ['general_guidance']
            citations = answer['passages']
            identity_ok = all(p['source_id'] in documents and p['source_locator'] == documents[p['source_id']]['path'] and p['excerpt'] in documents[p['source_id']]['body'] for p in citations)
            relevance_ok = all(p['source_id'] in expected for p in citations) if expected else not citations
            rows.append(dict(id=case['id'], query=case['query'], expected_sources=case['expected_sources'],
                top3=ids[:3], top1_relevant=(bool(ids) and ids[0] in expected) if measurable else None,
                top3_relevant=bool(expected.intersection(ids[:3])) if measurable else None,
                answer_status=answer['answer_status'], status_correct=answer['answer_status'] in case['acceptable_statuses'],
                cited_sources=[p['source_id'] for p in citations], citation_identity_correct=identity_ok,
                citation_relevance_correct=relevance_ok, median_search_ask_ms=statistics.median(durations)))
    def metric(field):
        values = [r[field] for r in rows if r[field] is not None]
        return dict(passed=sum(values), total=len(values), percent=round(100 * sum(values) / len(values), 2) if values else None)
    abstention = [r for r,c in zip(rows,dataset['cases']) if c['kind'] in ['insufficient','unrelated']]
    latencies = sorted(r['median_search_ask_ms'] for r in rows)
    report = dict(source_sha=source_sha, checked_at=datetime.now(timezone.utc).isoformat(),
        binary_sha256=digest(binary), dataset_sha256=digest(dataset_path),
        corpus_sha256={str(p.relative_to(ROOT)):digest(p) for base in ['knowledge','packages/python-fastapi-basics','benchmarks/retrieval/corpus'] for p in sorted((ROOT/base).rglob('*')) if p.is_file()},
        metrics={k:metric(k) for k in ['top1_relevant','top3_relevant','status_correct','citation_identity_correct','citation_relevance_correct']},
        insufficient_behavior=dict(passed=sum(r['status_correct'] for r in abstention), total=len(abstention)),
        latency=dict(unit='milliseconds',scope='three repetitions, median of separate search + ask subprocesses; includes startup/store reads',
            median=statistics.median(latencies),p95=latencies[max(0, (95*len(latencies)+99)//100-1)]),cases=rows)
    destination.parent.mkdir(parents=True,exist_ok=True)
    destination.write_text(json.dumps(report,indent=2,ensure_ascii=False)+'\n')
    print(json.dumps({k:report[k] for k in ['source_sha','dataset_sha256','metrics','insufficient_behavior','latency']},indent=2))
    if baseline:
        previous = json.loads(baseline.read_text())
        if previous['dataset_sha256'] != report['dataset_sha256'] or previous['corpus_sha256'] != report['corpus_sha256']:
            raise ValueError('Dataset/corpus changed; do not silently relabel or compare incompatible runs')
        for old, new in zip(previous['cases'], rows, strict=True):
            if old['id'] != new['id'] or old['query'] != new['query'] or old['expected_sources'] != new['expected_sources']:
                raise ValueError('Benchmark labels/order changed')
            for field in ['top1_relevant', 'top3_relevant', 'status_correct', 'citation_identity_correct', 'citation_relevance_correct']:
                if old[field] is True and new[field] is not True:
                    raise AssertionError(f"{new['id']}: regression in {field}")
    return report


if __name__ == '__main__':
    parser = argparse.ArgumentParser()
    parser.add_argument('--binary', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--source-sha', required=True, help='Exact tested source SHA; append +worktree when uncommitted')
    parser.add_argument('--baseline', type=Path, help='Reject per-case regressions and incompatible labels/corpus')
    args = parser.parse_args()
    run(args.binary.resolve(), args.output, args.source_sha, args.baseline)
