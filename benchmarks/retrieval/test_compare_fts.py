"""Small behavioral checks for the isolated FTS experiment (stdlib only)."""
import unittest

from compare_fts import build_index, fts_query, rerank, search_fts


def document(source_id, title, body='', **metadata):
    return dict(source_id=source_id, title=title, body=body, metadata=metadata)


class FtsExperimentTests(unittest.TestCase):
    def test_query_treats_operators_quotes_and_repeated_terms_as_data(self):
        self.assertEqual(fts_query('port PORT " OR x:y'), '"port" OR """" OR "or" OR "x:y"')
        with build_index([document('a', 'port'), document('b', 'quorum')]) as db:
            self.assertEqual([r['source_id'] for r in search_fts(db, 'port " OR x:y')], ['a'])
            self.assertEqual(search_fts(db, '   '), [])
            self.assertEqual(search_fts(db, '!!!'), [])

    def test_code_and_thai_use_sqlite_tokenization_without_english_translation(self):
        with build_index([document('a', 'ERR_MODULE_NOT_FOUND หาไฟล์ไม่เจอ')]) as db:
            self.assertEqual(search_fts(db, 'ERR_MODULE_NOT_FOUND')[0]['source_id'], 'a')
            self.assertEqual(search_fts(db, 'หาไฟล์ไม่เจอ')[0]['source_id'], 'a')
            self.assertEqual(search_fts(db, 'quantumflux'), [])

    def test_rerank_preserves_candidate_set_and_lexical_order_on_missing_or_tied_scores(self):
        candidates = ['missing', 'b', 'a', 'other']
        hits = [dict(source_id='a', bm25=-2), dict(source_id='b', bm25=-2),
                dict(source_id='unapproved', bm25=-10)]
        self.assertEqual(rerank(candidates, hits), ['b', 'a', 'missing', 'other'])
        self.assertEqual(rerank(candidates, []), candidates)


if __name__ == '__main__':
    unittest.main()
