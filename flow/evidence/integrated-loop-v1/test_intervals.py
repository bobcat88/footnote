import copy
import unittest

from intervals import merge_intervals


class MergeIntervalsTests(unittest.TestCase):
    def test_merges_overlaps_and_returns_sorted_tuples(self):
        self.assertEqual(
            merge_intervals([[8, 10], [2, 4], [-3, 0], [1, 3], [2, 3], [1, 3]]),
            [(-3, 0), (1, 4), (8, 10)],
        )

    def test_keeps_touching_billing_segments_separate(self):
        self.assertEqual(
            merge_intervals([(4, 7), (0, 2), (2, 4)]),
            [(0, 2), (2, 4), (4, 7)],
        )

    def test_overlap_chain_can_bridge_a_boundary(self):
        self.assertEqual(merge_intervals([(0, 2), (2, 4), (1, 3)]), [(0, 4)])

    def test_explicit_false_keeps_touching_billing_segments_separate(self):
        intervals = [(4, 7), (0, 2), (2, 4)]
        self.assertEqual(
            merge_intervals(intervals, join_touching=False),
            [(0, 2), (2, 4), (4, 7)],
        )

    def test_joins_touching_intervals_transitively_when_enabled(self):
        self.assertEqual(
            merge_intervals(
                ((4, 7), (-3, 0), (2, 4), (0, 2), (1, 3), (2, 3), (9, 11)),
                join_touching=True,
            ),
            [(-3, 7), (9, 11)],
        )

    def test_joins_a_chain_of_only_touching_intervals(self):
        self.assertEqual(
            merge_intervals([(4, 7), (0, 2), (2, 4)], join_touching=True),
            [(0, 7)],
        )

    def test_preserves_input(self):
        intervals = [[5, 8], [0, 3], [1, 6]]
        original = copy.deepcopy(intervals)
        self.assertEqual(merge_intervals(intervals), [(0, 8)])
        self.assertEqual(intervals, original)

    def test_preserves_input_when_joining_touching_intervals(self):
        intervals = [[4, 7], [0, 2], [2, 4]]
        original = copy.deepcopy(intervals)
        self.assertEqual(merge_intervals(intervals, join_touching=True), [(0, 7)])
        self.assertEqual(intervals, original)

    def test_accepts_tuple_input_and_empty_sequences(self):
        self.assertEqual(merge_intervals(((2, 5), (0, 3))), [(0, 5)])
        self.assertEqual(merge_intervals([]), [])
        self.assertEqual(merge_intervals(()), [])

    def test_empty_sequences_with_explicit_join_touching(self):
        for intervals in ([], ()):
            for join_touching in (False, True):
                with self.subTest(intervals=intervals, join_touching=join_touching):
                    self.assertEqual(
                        merge_intervals(intervals, join_touching=join_touching), []
                    )

    def test_join_touching_requires_a_boolean(self):
        for join_touching in (None, 0, 1, 1.0, "true", [], {}, object()):
            for intervals in ([], [(0, 2)]):
                with self.subTest(join_touching=join_touching, intervals=intervals):
                    with self.assertRaises(ValueError):
                        merge_intervals(intervals, join_touching=join_touching)

    def test_join_touching_is_keyword_only(self):
        with self.assertRaises(TypeError):
            merge_intervals([(0, 2)], True)

    def test_rejects_malformed_input_with_value_error(self):
        invalid_inputs = [
            None, 42, "12", {"start": 1, "end": 2}, {1, 2},
            iter([(1, 2)]), [None], [42], ["12"], [{1, 2}],
            [()], [(1,)], [(1, 2, 3)], [(2, 2)], [(3, 1)],
            [(True, 2)], [(0, False)], [(1.0, 2)], [(1, 2.0)],
            [("1", 2)], [(1, None)], [(0, 2), (3, False)],
        ]
        for intervals in invalid_inputs:
            with self.subTest(intervals=intervals):
                with self.assertRaises(ValueError):
                    merge_intervals(intervals)
                for join_touching in (False, True):
                    with self.subTest(join_touching=join_touching):
                        with self.assertRaises(ValueError):
                            merge_intervals(intervals, join_touching=join_touching)

    def test_rejected_input_stays_unchanged(self):
        for join_touching in (False, True):
            intervals = [[4, 7], [0, 2], [2, False]]
            original = copy.deepcopy(intervals)
            with self.subTest(join_touching=join_touching):
                with self.assertRaises(ValueError):
                    merge_intervals(intervals, join_touching=join_touching)
                self.assertEqual(intervals, original)


if __name__ == "__main__":
    unittest.main()
