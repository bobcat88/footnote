"""Frozen dependency-sensitive development fixture and independent acceptance suite."""

import json
import subprocess
import sys
import unittest
from pathlib import Path

SOURCES = {
    'intervals.py': '''def merge_intervals(intervals):
    result = []
    for start, end in sorted(intervals):
        if result and start < result[-1][1]:
            result[-1] = (result[-1][0], max(end, result[-1][1]))
        else:
            result.append((start, end))
    return result
''',
    'availability.py': '''from intervals import merge_intervals

def available_windows(bookings, start, end):
    merged = merge_intervals(bookings)
    result, cursor = [], start
    for left, right in merged:
        left, right = max(start, left), min(end, right)
        if left >= right:
            continue
        if cursor < left:
            result.append((cursor, left))
        cursor = max(cursor, right)
    if cursor < end:
        result.append((cursor, end))
    return result

def busy_windows(bookings):
    return merge_intervals(bookings)
''',
    'billing.py': '''from intervals import merge_intervals

def invoice(bookings, rate=2, segment_fee=5):
    segments = merge_intervals(bookings)
    return {"segments": segments, "units": sum(b-a for a,b in segments),
            "total": sum(b-a for a,b in segments)*rate + len(segments)*segment_fee}
''',
    'api.py': '''from availability import available_windows, busy_windows
from billing import invoice

def handle(payload):
    bookings = payload["bookings"]
    return {"busy": busy_windows(bookings),
            "free": available_windows(bookings, payload["start"], payload["end"]),
            "invoice": invoice(bookings)}
''',
    'cli.py': '''import json
import sys
from api import handle

if __name__ == "__main__":
    print(json.dumps(handle(json.load(sys.stdin))))
''',
    'test_existing.py': '''import unittest
from availability import available_windows
from billing import invoice
from api import handle

class Existing(unittest.TestCase):
    def test_gap(self):
        self.assertEqual(available_windows([(2,4),(6,8)],0,10), [(0,2),(4,6),(8,10)])
    def test_bill(self):
        self.assertEqual(invoice([(0,2),(2,4)])["total"],18)
    def test_api(self):
        self.assertEqual(handle({"bookings":[],"start":0,"end":10})["free"],[(0,10)])
''',
}
CONTRACT = '''Interval service contract v1. Half-open integer intervals [start,end), start < end.
Booleans are not integers for this contract. Bad intervals raise ValueError. Inputs are
not mutated, generators are supported, and output intervals are sorted tuple pairs.
merge_intervals defaults to preserving touching boundaries. Overlaps always merge.
Billing charges a fixed fee PER merged segment, in addition to duration times rate.
Thus touching billing segments must remain separate. Availability may join boundaries.
API currently returns busy, free and invoice. CLI accepts one JSON object on stdin.
'''
DECISION = '''merge_intervals migration decision: billing boundary policy must remain strict
even when display/availability joins touching windows. API fan-out must materialize a
one-shot bookings iterable once before multiple consumers. Never let one consumer
exhaust data needed by billing. Domain failures use ValueError, not leaking TypeError.
'''
OBJECTIVE = '''Implement the interval-service policy migration across this workspace.
Read CONTRACT.txt and DECISION.txt. Add keyword-only join_touching=False to
merge_intervals and busy_windows. True merges touching chains; False preserves
touching boundaries. Reject non-boolean policies and malformed intervals with
ValueError, preserve inputs, support one-shot iterables, and use O(n log n) merging.
available_windows must use joined busy windows and reject invalid bounds with ValueError.
Keep invoice's public signature and strict boundary billing policy unchanged.
handle(payload) accepts optional join_touching default False, applies it to busy only,
and computes free and invoice from the same bookings even for a generator. Preserve
the response's existing keys and JSON shape. CLI must honor the same policy.
Add focused regression tests in test_migration.py, retain existing tests, and write
delivery.txt with changes, checks, and unresolved issues. Standard library only.
Owned outputs: intervals.py, availability.py, billing.py, api.py, cli.py,
test_migration.py, delivery.txt. Do not edit task, contract, decision or existing tests.
One delivery, no parent repair or stronger-model fallback. No network, installs,
commits, child agents, other arms, hidden checks, or unrelated files. Aim for at most
12 model responses and 8 minutes, requested limits rather than provider caps.
Stop after local verification and keep the final response short.
'''
OWNED = ['intervals.py','availability.py','billing.py','api.py','cli.py','test_migration.py','delivery.txt']


def check(workspace):
    work = Path(workspace).resolve()
    sys.path.insert(0, str(work))
    from intervals import merge_intervals
    from availability import available_windows, busy_windows
    from billing import invoice
    from api import handle

    class Acceptance(unittest.TestCase):
        def test_01_legacy_boundaries(self):
            self.assertEqual(merge_intervals([(2,4),(0,2),(1,2)]), [(0,2),(2,4)])
        def test_02_join_transitive(self):
            self.assertEqual(merge_intervals([(4,8),(0,2),(2,4)], join_touching=True), [(0,8)])
        def test_03_bad_policy(self):
            for flag in (None,0,1,'yes',[]):
                with self.subTest(flag=flag), self.assertRaises(ValueError):
                    merge_intervals([],join_touching=flag)
        def test_04_bad_intervals(self):
            for value in (None,1,[(True,2)],[(0,False)],[(0,1.5)],[(2,2)],[(3,1)],[(0,1,2)],[1],['12']):
                with self.subTest(value=value), self.assertRaises(ValueError):
                    merge_intervals(value)
        def test_05_purity_generator(self):
            data = [[4,6],[0,2],[1,3]]
            copy = [r[:] for r in data]
            self.assertEqual(merge_intervals(iter(data)),[(0,3),(4,6)])
            self.assertEqual(data,copy)
        def test_06_busy_policy(self):
            self.assertEqual(busy_windows([(0,2),(2,4)],join_touching=True),[(0,4)])
        def test_07_free_clip(self):
            self.assertEqual(available_windows(iter([(-5,2),(2,4),(6,20)]),0,10),[(4,6)])
        def test_08_bounds(self):
            for a,b in ((True,5),(0,False),(2,2),(3,1),(0,3.5)):
                with self.subTest(a=a,b=b), self.assertRaises(ValueError):
                    available_windows([],a,b)
        def test_09_billing(self):
            self.assertEqual(invoice([(0,2),(2,4)]),{'segments':[(0,2),(2,4)],'units':4,'total':18})
        def test_10_api_join_and_billing(self):
            got=handle({'bookings':iter([(0,2),(2,4)]),'start':0,'end':6,'join_touching':True})
            self.assertEqual(got,{'busy':[(0,4)],'free':[(4,6)],'invoice':{'segments':[(0,2),(2,4)],'units':4,'total':18}})
        def test_11_api_default_and_purity(self):
            data=[[0,2],[2,4]]
            got=handle({'bookings':data,'start':0,'end':6})
            self.assertEqual(got['busy'],[(0,2),(2,4)])
            self.assertEqual(data,[[0,2],[2,4]])
        def test_12_api_policy(self):
            with self.assertRaises(ValueError):
                handle({'bookings':[],'start':0,'end':6,'join_touching':1})
        def test_13_cli(self):
            proc=subprocess.run(['rtk','proxy',sys.executable,str(work/'cli.py')],input=json.dumps({'bookings':[[0,2],[2,4]],'start':0,'end':6,'join_touching':True}),text=True,capture_output=True,timeout=15,cwd=work)
            self.assertEqual(proc.returncode,0,proc.stderr)
            got=json.loads(proc.stdout)
            self.assertEqual(got['busy'],[[0,4]])
            self.assertEqual(got['invoice']['total'],18)
        def test_14_large_chain(self):
            self.assertEqual(merge_intervals(((i,i+1) for i in range(5000,-1,-1)),join_touching=True),[(0,5001)])

    return unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Acceptance)).wasSuccessful()


if __name__ == '__main__':
    raise SystemExit(0 if check(sys.argv[1]) else 1)
