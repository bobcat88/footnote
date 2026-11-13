import sys
import tempfile
import unittest
from pathlib import Path

import numpy as np

sys.path.insert(0, str(Path(__file__).resolve().parent))
import icm_ablation as suite


class AblationTests(unittest.TestCase):
    def setUp(self):
        self.corpus = {'a': ('', 'orchard apples apples'), 'b': ('', 'ocean coral reef'), 'c': ('', 'orchard pears')}
        self.queries = {'q1': 'apples', 'q2': 'unmatchedword'}
        self.qrels = {'q1': {'a': 1}, 'q2': {'b': 1}}
        self.docs = np.array([[1., 0.], [0., 1.], [.8, .2]], dtype=np.float32)
        self.qvec = {'apples': np.array([1., 0.], dtype=np.float32), 'unmatchedword': np.array([0., 1.], dtype=np.float32)}

    def test_both_lexical_engines_find_matching_document_and_reject_fts_syntax(self):
        with tempfile.TemporaryDirectory() as tmp:
            for backend in ('fts5', 'tantivy'):
                folder = Path(tmp) / backend
                folder.mkdir()
                e = suite.Lexical(backend, folder, suite.text_corpus(self.corpus))
                try:
                    self.assertGreater(e.scores('apples OR : *')[0], 0)
                    self.assertEqual(float(e.scores('unmatchedword').sum()), 0)
                finally:
                    e.close()

    def test_matrix_scores_same_inputs_and_dense_fallback(self):
        rows = suite.run_arms(self.corpus, self.queries, self.qrels, self.docs, self.qvec)
        self.assertEqual({(r['lexical'], r['fusion']) for r in rows},
                         {('fts5', 'cc'), ('fts5', 'rrf'), ('tantivy', 'cc'), ('tantivy', 'rrf')})
        for row in rows:
            self.assertEqual(row['rankings']['q1'][0], 'a')
            self.assertEqual(row['rankings']['q2'][0], 'b')
            self.assertEqual(row['metrics']['ndcg@10'], 1.)
            self.assertEqual(row['rankings_sha256'], suite.digest(row['rankings']))
            self.assertEqual(row['determinism'], {'queries_checked': 2, 'identical_rankings': True})

    def test_refuses_mismatched_judgments(self):
        with self.assertRaisesRegex(ValueError, 'same IDs'):
            suite.run_arms(self.corpus, self.queries, {'q1': {'a': 1}}, self.docs, self.qvec)


if __name__ == '__main__':
    unittest.main()
