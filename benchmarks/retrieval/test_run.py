"""Offline checks for benchmark evidence validation and regression gates."""
import copy
import importlib.util
import json
from pathlib import Path
import tempfile
import unittest

SPEC = importlib.util.spec_from_file_location('retrieval_runner', Path(__file__).with_name('run.py'))
runner = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(runner)


class DatasetTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.dataset = {'schema': 1, 'split': 'evaluation',
            'provenance': {'kind': 'synthetic', 'id': 'held-out'},
            'cases': [{'id': 'X1', 'query': 'Unique diagnostic', 'kind': 'exact',
                'expected_sources': ['builtin:one'], 'acceptable_statuses': ['retrieved_guidance']}]}

    def save(self, data, name='cases.json'):
        path = self.root / name
        path.write_text(json.dumps(data))
        return path

    def test_frozen_original_and_holdout_load_without_overlap(self):
        original = runner.load_dataset(runner.DEFAULT_DATASET)
        holdout = runner.load_dataset(Path(__file__).with_name('holdout.json'))
        runner.validate_disjoint(holdout, original)
        self.assertEqual(len(original['cases']), 40)
        self.assertEqual(holdout['split'], 'evaluation')
        self.assertEqual(runner.digest(runner.DEFAULT_DATASET), '6ba824757fca6994397873de76a7f71d3f57a8a0b2f0a56f0f00bf42e4b7a794')
        self.assertEqual(runner.digest(Path(__file__).with_name('holdout.json')), 'f0970ca06b061fce20baae07a40f44541866514a2ee192f9334c56509a9a8aac')

    def test_alternate_dataset_requires_explicit_provenance(self):
        del self.dataset['provenance']
        with self.assertRaises(ValueError):
            runner.load_dataset(self.save(self.dataset))

    def test_empty_cases_duplicate_ids_and_empty_query_are_rejected(self):
        for cases in [[], self.dataset['cases'] * 2, [dict(self.dataset['cases'][0], query=' ')]]:
            with self.subTest(cases=cases), self.assertRaises(ValueError):
                runner.load_dataset(self.save(dict(self.dataset, cases=cases)))

    def test_normalized_query_overlap_is_rejected(self):
        other = copy.deepcopy(self.dataset)
        other['provenance']['id'] = 'tuning'
        other['cases'][0]['query'] = ' UNIQUE\n diagnostic  '
        with self.assertRaisesRegex(ValueError, 'overlap'):
            runner.validate_disjoint(self.dataset, other)

    def real_dataset(self):
        log = self.root / 'incident.log'
        log.write_text('Unique diagnostic\n')
        self.dataset['provenance'] = {'kind': 'real_incidents', 'id': 'recorded-logs', 'origin': 'Anonymized local log export'}
        self.dataset['cases'][0]['source'] = {'path': log.name, 'sha256': runner.digest(log), 'incident_id': 'incident-1'}
        return self.dataset

    def test_real_log_must_exist_match_hash_and_contain_query(self):
        real = self.real_dataset()
        loaded = runner.load_dataset(self.save(real))
        self.assertEqual(runner.incident_evidence(loaded)['verified_logs'], 1)
        self.assertFalse(runner.incident_evidence(loaded)['complete'])
        for source in [dict(real['cases'][0]['source'], path='missing.log'), dict(real['cases'][0]['source'], sha256='0' * 64)]:
            with self.subTest(source=source), self.assertRaises(ValueError):
                runner.load_dataset(self.save(dict(real, cases=[dict(real['cases'][0], source=source)])))
        real['cases'][0]['query'] = 'Invented error absent from log'
        with self.assertRaisesRegex(ValueError, 'query'):
            runner.load_dataset(self.save(real))

    def test_same_incident_or_source_cannot_cross_splits(self):
        real = self.real_dataset()
        other = copy.deepcopy(real)
        other['provenance']['id'] = 'other-split'
        other['cases'][0]['query'] = 'Different excerpt'
        with self.assertRaisesRegex(ValueError, 'overlap'):
            runner.validate_disjoint(real, other)

    def test_fifty_distinct_source_checked_logs_complete_the_real_set(self):
        real = self.real_dataset()
        real['cases'] = []
        for n in range(50):
            log = self.root / f'incident-{n}.log'
            log.write_text(f'Observed diagnostic {n}\n')
            real['cases'].append(dict(id=f'I{n}', query=f'Observed diagnostic {n}',
                expected_sources=[], acceptable_statuses=['no_adequate_match'],
                source=dict(path=log.name, sha256=runner.digest(log), incident_id=f'original-{n}')))
        loaded = runner.load_dataset(self.save(real))
        self.assertEqual(runner.incident_evidence(loaded)['verified_logs'], 50)
        self.assertTrue(runner.incident_evidence(loaded)['complete'])

    def test_fifty_rephrasings_do_not_count_as_fifty_logs(self):
        real = self.real_dataset()
        real['cases'] = [dict(real['cases'][0], id=str(n)) for n in range(50)]
        self.assertEqual(runner.incident_evidence(real)['verified_logs'], 1)
        self.assertFalse(runner.incident_evidence(real)['complete'])


class SourceReceiptTests(unittest.TestCase):
    def test_receipt_requires_matching_tested_binary(self):
        with tempfile.TemporaryDirectory() as temp:
            binary = Path(temp) / 'lbc'
            binary.write_text('one binary')
            receipt = Path(temp) / 'source.json'
            receipt.write_text(json.dumps(dict(source_sha='original+worktree',
                binary_sha256='wrong', implementation_sha256={'src/index.rs': 'f' * 64})))
            with self.assertRaisesRegex(ValueError, 'binary'):
                runner.source_evidence(binary, 'current+worktree', receipt)

    def test_matching_receipt_preserves_original_source_snapshot(self):
        with tempfile.TemporaryDirectory() as temp:
            binary = Path(temp) / 'lbc'
            binary.write_text('one binary')
            receipt = Path(temp) / 'source.json'
            hashes = {'src/index.rs': 'f' * 64}
            receipt.write_text(json.dumps(dict(source_sha='original+worktree',
                binary_sha256=runner.digest(binary), implementation_sha256=hashes)))
            source = runner.source_evidence(binary, 'current+worktree', receipt)
            self.assertEqual(source['source_sha'], 'original+worktree')
            self.assertEqual(source['implementation_sha256'], hashes)
            self.assertEqual(source['implementation_snapshot_origin']['kind'], 'receipt')
            self.assertEqual(source['implementation_snapshot_origin']['sha256'], runner.digest(receipt))


class MetricsTests(unittest.TestCase):
    def case(self):
        return {'id': 'X', 'query': 'error', 'expected_sources': ['builtin:one'], 'acceptable_statuses': ['retrieved_guidance']}

    def test_empty_excerpt_and_missing_citation_do_not_pass_identity(self):
        docs = {'builtin:one': {'path': 'builtin/one', 'body': 'Actual evidence'}}
        for passages in [[], [{'source_id': 'builtin:one', 'source_locator': 'builtin/one', 'excerpt': ''}]]:
            with self.subTest(passages=passages):
                row = runner.score_case(self.case(), [{'source_id': 'builtin:one'}],
                    {'answer_status': 'retrieved_guidance', 'passages': passages}, docs, [1, 2, 3])
                self.assertFalse(row['citation_identity_correct'])

    def test_extra_results_and_citations_lower_precision(self):
        docs = {'builtin:one': {'path': 'one', 'body': 'Evidence'}, 'builtin:two': {'path': 'two', 'body': 'Other'}}
        passages = [{'source_id': source, 'source_locator': doc['path'], 'excerpt': doc['body']} for source, doc in docs.items()]
        row = runner.score_case(self.case(), [{'source_id': s} for s in docs],
            {'answer_status': 'retrieved_guidance', 'passages': passages}, docs, [1, 2, 3])
        self.assertEqual(row['relevant_results'], 1)
        self.assertEqual(row['result_count'], 2)
        self.assertEqual(row['irrelevant_citations'], 1)
        self.assertFalse(row['citation_relevance_correct'])
        self.assertEqual(row['citations'], passages)

    def test_case_regression_is_rejected_even_when_aggregate_improves(self):
        old = {'dataset_sha256': 'same', 'corpus_sha256': {}, 'cases': [dict(self.case(), top1_relevant=True)]}
        new = copy.deepcopy(old)
        new['cases'][0]['top1_relevant'] = False
        with self.assertRaisesRegex(AssertionError, 'top1_relevant'):
            runner.check_regressions(old, new)

    def test_valid_and_invalid_citations_have_a_nonvacuous_identity_count(self):
        docs = {'builtin:one': {'path': 'one', 'body': 'Evidence'}}
        passages = [{'source_id': 'builtin:one', 'source_locator': 'one', 'excerpt': 'Evidence'},
            {'source_id': 'builtin:one', 'source_locator': 'one', 'excerpt': 'Fabricated'}]
        row = runner.score_case(self.case(), [], {'answer_status': 'retrieved_guidance', 'passages': passages}, docs, [1])
        self.assertEqual(row['valid_citation_identities'], 1)
        self.assertFalse(row['citation_identity_correct'])

    def test_abstention_with_irrelevant_search_results_fails_no_match(self):
        case = dict(self.case(), expected_sources=[], acceptable_statuses=['no_adequate_match'])
        row = runner.score_case(case, [{'source_id': 'builtin:one'}],
            {'answer_status': 'no_adequate_match', 'passages': []}, {}, [1])
        self.assertFalse(row['no_match_correct'])

    def test_precision_regression_rejected_when_top1_still_passes(self):
        old = {'dataset_sha256': 'same', 'corpus_sha256': {},
            'cases': [dict(self.case(), top1_relevant=True, result_count=1, relevant_results=1)]}
        new = copy.deepcopy(old)
        new['cases'][0]['result_count'] = 2
        with self.assertRaisesRegex(AssertionError, 'precision'):
            runner.check_regressions(old, new)

    def test_changed_status_labels_are_not_comparable(self):
        old = {'dataset_sha256': 'same', 'corpus_sha256': {}, 'cases': [dict(self.case(), top1_relevant=True)]}
        new = copy.deepcopy(old)
        new['cases'][0]['acceptable_statuses'] = ['general_guidance']
        with self.assertRaisesRegex(ValueError, 'labels'):
            runner.check_regressions(old, new)


if __name__ == '__main__':
    unittest.main()
