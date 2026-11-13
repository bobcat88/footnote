import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent))
import longmem_score as scorer
import longmem_footnote as runner
import longmem_icm as icm
import icm_ablation_compare as comparator
import footprint_inventory as footprint
from benchmark_checkpoint import Checkpoint


class LongmemTests(unittest.TestCase):
    def setUp(self):
        self.questions = [
            {'question_id': 'one', 'question_type': 'single', 'haystack_session_ids': ['a', 'b'],
             'answer_session_ids': ['b'], 'haystack_sessions': [[{'role': 'user', 'content': 'alpha'}], [{'role': 'user', 'content': 'bravo'}]]},
            {'question_id': 'two_abs', 'question_type': 'multi', 'haystack_session_ids': ['c', 'd'],
             'answer_session_ids': ['c', 'd'], 'haystack_sessions': [[{'role': 'user', 'content': 'charlie'}], [{'role': 'user', 'content': 'delta'}]]},
        ]

    def test_vendor_prefix_and_abstention_are_scored(self):
        ranks = {'one': ['one_a', 'one_b'], 'two_abs': ['two_abs_c', 'two_abs_d']}
        result = scorer.score(ranks, self.questions, gold={'one': ['one_b'], 'two_abs': ['two_abs_c', 'two_abs_d']})
        self.assertEqual(result['metrics']['all']['any@5'], 1.)
        self.assertEqual(result['metrics']['abstention_tagged']['questions'], 1)
        self.assertEqual(result['metrics']['answerable']['questions'], 1)

    def test_raw_duplicates_consume_rank_but_ndcg_is_distinct(self):
        ranked = {'two_abs': ['c'] * 5 + ['d']}
        result = scorer.score(ranked, [self.questions[1]])
        self.assertEqual(result['metrics']['all']['all@5'], 0.)
        self.assertEqual(result['metrics']['all']['ndcg@10_distinct'], 1.)

    def test_refuses_missing_unknown_foreign_and_wrong_gold(self):
        with self.assertRaisesRegex(ValueError, 'coverage mismatch'):
            scorer.score({'one': ['a']}, self.questions)
        with self.assertRaisesRegex(ValueError, 'foreign session'):
            scorer.score({'one': ['x']}, [self.questions[0]])
        with self.assertRaisesRegex(ValueError, 'gold mismatch'):
            scorer.score({'one': ['a']}, [self.questions[0]], gold={'one': ['a']})
        with self.assertRaisesRegex(ValueError, 'duplicate query'):
            scorer.rankings_and_metadata({'dataset': 'longmemeval', 'mode': 'recall', 'results': [
                {'query_id': 'one', 'ranked_ids': [], 'gold_ids': ['b']},
                {'query_id': 'one', 'ranked_ids': [], 'gold_ids': ['b']}]})

    def test_paired_rejects_different_queries(self):
        with self.assertRaisesRegex(ValueError, 'identical'):
            scorer.paired({'a': {}}, {'b': {}})

    def test_footprint_reports_bytes_and_rejects_overlapping_inputs(self):
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp)
            (path / 'data').write_bytes(b'abc')
            row = footprint.measure([('store', path)])[0]
            self.assertEqual((row['files'], row['bytes']), (1, 3))
            with self.assertRaisesRegex(ValueError, 'overlapping'):
                footprint.measure([('directory', path), ('same', path / 'data')])

    def test_checkpoint_requires_explicit_matching_resume(self):
        with tempfile.TemporaryDirectory() as tmp:
            output = Path(tmp) / 'result.json'
            identity = {'question_ids': ['one', 'two'], 'binary': 'abc'}
            c = Checkpoint(output, identity)
            c.save('one', ['a'], {'sessions': 1})
            with self.assertRaisesRegex(ValueError, 'use --resume'):
                Checkpoint(output, identity)
            resumed = Checkpoint(output, identity, resume=True)
            self.assertEqual(resumed.rankings, {'one': ['a']})
            with self.assertRaisesRegex(ValueError, 'identity differs'):
                Checkpoint(output, {'question_ids': ['one'], 'binary': 'other'}, resume=True)
            with self.assertRaisesRegex(ValueError, 'duplicate'):
                resumed.save('one', [], {})

    def test_utf8_splitting_is_lossless_and_bounded(self):
        text = 'été 🐱' * 100
        parts = list(icm.split_utf8(text, 17))
        self.assertEqual(''.join(parts), text)
        self.assertTrue(all(len(p.encode('utf8')) <= 17 for p in parts))

    def test_ablation_paired_comparison_detects_reversal(self):
        a = {'q': ['a', 'b']}
        b = {'q': ['b', 'a']}
        result = comparator.paired(a, b, {'q': {'b': 1}}, resamples=100)
        self.assertGreater(result['ndcg@10']['b_minus_a'], 0)
        self.assertGreater(result['ndcg@10']['bootstrap_95'][0], 0)
        with self.assertRaisesRegex(ValueError, 'same nonempty'):
            comparator.paired(a, {}, {'q': {'b': 1}}, resamples=100)

    def test_user_sessions_drop_assistant_and_reject_duplicate_conflicts(self):
        q = dict(self.questions[0]); q['haystack_sessions'] = [
            [{'role': 'assistant', 'content': 'secret'}, {'role': 'user', 'content': 'alpha'}],
            [{'role': 'user', 'content': 'bravo'}]]
        self.assertEqual(runner.user_sessions(q)['a'], ('', 'alpha'))
        q['haystack_session_ids'] = ['a', 'a']
        with self.assertRaisesRegex(ValueError, 'different contents'):
            runner.user_sessions(q)


if __name__ == '__main__':
    unittest.main()
