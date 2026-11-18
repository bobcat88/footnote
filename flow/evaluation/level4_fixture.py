"""Frozen L4 cross-layer diagnosis-and-recovery fixture and independent acceptance suite."""

import json
import random
import shutil
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

SOURCES = {
    'db.py': '''import sqlite3

SCHEMA = """
CREATE TABLE IF NOT EXISTS stock (sku TEXT PRIMARY KEY, qty INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
CREATE TABLE IF NOT EXISTS orders (order_id TEXT PRIMARY KEY, lines TEXT NOT NULL, status TEXT NOT NULL);
CREATE TABLE IF NOT EXISTS events (id INTEGER PRIMARY KEY, order_id TEXT, line_no INTEGER,
                                   sku TEXT NOT NULL, delta INTEGER NOT NULL);
INSERT OR IGNORE INTO meta VALUES ('applied_through', 0);
"""


def connect(path):
    conn = sqlite3.connect(path)
    conn.executescript(SCHEMA)
    return conn


def applied_through(conn):
    return conn.execute("SELECT value FROM meta WHERE key = 'applied_through'").fetchone()[0]


def iter_events(conn, after=0, batch=100):
    """Yield (id, order_id, line_no, sku, delta) rows newer than `after`, oldest first."""
    last = after
    while True:
        rows = conn.execute(
            "SELECT id, order_id, line_no, sku, delta FROM events WHERE id > ? ORDER BY id LIMIT ?",
            (last, batch)).fetchall()
        if not rows:
            return
        yield from rows
        # continue from the row after the last one of this page
        last = rows[-1][0] + 1
''',
    'projection.py': '''from db import applied_through, iter_events


def catch_up(conn):
    """Apply events newer than the projection to the stock table."""
    last = applied_through(conn)
    for event_id, _order, _line, sku, delta in iter_events(conn, after=last):
        conn.execute("INSERT INTO stock(sku, qty) VALUES (?, ?) "
                     "ON CONFLICT(sku) DO UPDATE SET qty = qty + excluded.qty", (sku, delta))
        last = event_id
        if event_id % 50 == 0:
            conn.commit()  # keep write transactions short on long catch-ups
    conn.execute("UPDATE meta SET value = ? WHERE key = 'applied_through'", (last,))
    conn.commit()
    return last


def stock(conn):
    return dict(conn.execute("SELECT sku, qty FROM stock ORDER BY sku"))
''',
    'service.py': '''import json


def receive(conn, sku, qty):
    conn.execute("INSERT INTO events(order_id, line_no, sku, delta) VALUES (NULL, NULL, ?, ?)", (sku, qty))
    conn.commit()


def ship(conn, order_id, lines):
    """Record an outgoing order; lines is a list of [sku, qty]."""
    conn.execute("INSERT INTO orders VALUES (?, ?, 'pending')", (order_id, json.dumps(lines)))
    conn.commit()
    for line_no, (sku, qty) in enumerate(lines):
        conn.execute("INSERT INTO events(order_id, line_no, sku, delta) VALUES (?, ?, ?, ?)",
                     (order_id, line_no, sku, -qty))
        conn.commit()
    conn.execute("UPDATE orders SET status = 'done' WHERE order_id = ?", (order_id,))
    conn.commit()
''',
    'recovery.py': '''import json
from projection import catch_up


def recover(conn):
    """Finish orders interrupted by a crash, then refresh the stock projection."""
    done = []
    for order_id, lines in conn.execute(
            "SELECT order_id, lines FROM orders WHERE status = 'pending' ORDER BY order_id").fetchall():
        for line_no, (sku, qty) in enumerate(json.loads(lines)):
            conn.execute("INSERT INTO events(order_id, line_no, sku, delta) VALUES (?, ?, ?, ?)",
                         (order_id, line_no, sku, -qty))
        conn.execute("UPDATE orders SET status = 'done' WHERE order_id = ?", (order_id,))
        done.append(order_id)
    conn.commit()
    catch_up(conn)
    return done
''',
    'report.py': '''from db import iter_events


def shipped(conn):
    """Units shipped per SKU over the stored event log."""
    totals = {}
    for _id, _order, _line, sku, delta in iter_events(conn):
        if delta < 0:
            totals[sku] = totals.get(sku, 0) - delta
    return dict(sorted(totals.items()))
''',
    'cli.py': '''import json
import sys
from db import connect
from projection import catch_up, stock
from recovery import recover
from report import shipped
from service import ship


def main(argv):
    path, command, *rest = argv
    conn = connect(path)
    if command == "stock":
        out = stock(conn)
    elif command == "report":
        out = shipped(conn)
    elif command == "refresh":
        out = catch_up(conn)
    elif command == "recover":
        out = recover(conn)
    elif command == "ship":
        order_id, *items = rest
        lines = [[sku, int(qty)] for sku, qty in (item.split(":") for item in items)]
        ship(conn, order_id, lines)
        out = order_id
    else:
        raise SystemExit("unknown command " + command)
    print(json.dumps(out))


if __name__ == "__main__":
    main(sys.argv[1:])
''',
    'test_existing.py': '''import unittest
from db import connect
from projection import catch_up, stock
from report import shipped
from service import receive, ship


class Existing(unittest.TestCase):
    def test_flow(self):
        conn = connect(":memory:")
        receive(conn, "A", 10)
        receive(conn, "B", 4)
        ship(conn, "o1", [["A", 3], ["B", 1]])
        self.assertEqual(catch_up(conn), 4)
        self.assertEqual(stock(conn), {"A": 7, "B": 3})
        self.assertEqual(shipped(conn), {"A": 3, "B": 1})

    def test_refresh_is_incremental(self):
        conn = connect(":memory:")
        receive(conn, "A", 5)
        catch_up(conn)
        ship(conn, "o1", [["A", 2]])
        catch_up(conn)
        self.assertEqual(stock(conn), {"A": 3})
''',
}
CONTRACT = '''Warehouse service contract v1. The events table is the append-only source of truth.
Each event changes one SKU by delta: receipts are positive, shipment lines negative.
Events up to meta.compacted_through were archived; their effect survives only in the
stock snapshot, so stock can never be rebuilt from the event log alone.
stock is a projection: it equals the snapshot plus every event with id up to
meta.applied_through, each applied exactly once. projection.catch_up(conn) applies every
newer event exactly once, also when an earlier catch-up was interrupted part way, moves
applied_through to the newest event id and returns it; running it again changes nothing.
projection.stock(conn) and `cli.py DB stock` only read. Every reader of the event log sees
every event newer than its starting point exactly once, in id order, whatever page size
it reads with.
service.ship(conn, order_id, lines): order_id is a non-empty string; lines is a non-empty
list of [sku, qty] pairs with a non-empty string sku and a positive int qty (bool is not
int). Line n becomes one event (order_id, n, sku, -qty). Recording an order is
all-or-nothing. Repeating ship with the same order_id and the same lines never writes a
line twice: it completes an order left pending and changes nothing for a done one. The
same order_id with different lines raises ValueError. Invalid input raises ValueError
and writes nothing. service.receive is unchanged.
report.shipped(conn) returns {sku: units shipped} over every stored event, sorted by sku.
recovery.recover(conn) returns the sorted ids of the orders it completed.
CLI: python cli.py DB stock|report|refresh|recover|ship ORDER SKU:QTY... prints JSON.
Domain errors exit with status 2 and one line starting "error:" on stderr, no traceback.
'''
DECISION = '''Recovery decision: an order row with status "pending" is a durable intent written
before its lines. Recovery completes each pending order by writing only the lines that
are missing for that (order_id, line_no), marks it done, then catches the projection
up. It never deletes or rewrites existing events and never rebuilds stock from zero.
recover(conn) is all-or-nothing and idempotent: a pending order whose stored lines are
invalid aborts the whole recovery with ValueError and leaves the database unchanged.
warehouse.db is production data with no backup. Diagnose on copies.
'''
OBJECTIVE = '''Incident: warehouse service in this workspace. Read CONTRACT.txt and DECISION.txt.
Last night the shipping process crashed while it was recording order ord-1-crash,
and the nightly stock refresh had not run for several days. Since then:
1. `python cli.py warehouse.db report` shows shipped totals lower than the order
   lines add up to. Operations see it on busy weeks only; quiet weeks look right.
2. On a scratch copy, `python cli.py COPY recover` left stock that disagrees with the
   event log: some SKUs dropped further than the crashed order explains.
3. Retrying the crashed order with `python cli.py warehouse.db ship ...` fails with a
   traceback instead of finishing the order.
Diagnose and fix the causes so the service meets the contract, then bring
warehouse.db to the correct recovered state: no recorded effect may be lost or
applied twice. Add focused regression tests in test_regression.py, retain existing
tests, and write delivery.txt with the root cause of each symptom, changes, checks,
and unresolved issues. Standard library only.
Owned outputs: db.py, projection.py, service.py, recovery.py, report.py, cli.py,
warehouse.db, test_regression.py, delivery.txt. Do not edit task, contract, decision
or existing tests.
One delivery, no parent repair or stronger-model fallback. No network, installs,
commits, child agents or unrelated files. An independent acceptance step checks the delivery. Aim for at most
20 model responses and 12 minutes, requested limits rather than provider caps.
Stop after local verification and keep the final response short.
'''
OWNED = ['db.py', 'projection.py', 'service.py', 'recovery.py', 'report.py', 'cli.py',
         'warehouse.db', 'test_regression.py', 'delivery.txt']
RAW_SCHEMA = '''CREATE TABLE stock (sku TEXT PRIMARY KEY, qty INTEGER NOT NULL);
CREATE TABLE meta (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
CREATE TABLE orders (order_id TEXT PRIMARY KEY, lines TEXT NOT NULL, status TEXT NOT NULL);
CREATE TABLE events (id INTEGER PRIMARY KEY, order_id TEXT, line_no INTEGER,
                     sku TEXT NOT NULL, delta INTEGER NOT NULL);'''
SKUS = ['AX-1', 'AX-2', 'BX-7', 'CX-3', 'DX-9']
COMPACTED = 1000


def scenario(seed, events=260, lag=170):
    """Interrupted state: compacted snapshot, stale projection, one half-written order."""
    rng = random.Random(seed)
    base = {s: rng.randrange(300, 600) for s in SKUS}
    rows, orders, n = [], [], 0
    while len(rows) < events:
        ident = COMPACTED + 1 + len(rows)
        if rng.random() < 0.15:
            rows.append((ident, None, None, rng.choice(SKUS), rng.randrange(20, 60)))
            continue
        n += 1
        order_id = f'ord-{seed}-{n:04d}'
        lines = [[s, rng.randrange(1, 6)] for s in rng.sample(SKUS, rng.randrange(1, 4))]
        orders.append((order_id, lines, 'done'))
        rows += [(ident + i, order_id, i, s, -q) for i, (s, q) in enumerate(lines)]
    applied = rows[-1][0] - lag
    snapshot = dict(base)
    for ident, _o, _l, sku, delta in rows:
        if ident <= applied:
            snapshot[sku] += delta
    crash = (f'ord-{seed}-crash', [['BX-7', 4], ['AX-1', 2], ['CX-3', 3]])
    start = rows[-1][0] + 1
    rows += [(start + i, crash[0], i, s, -q) for i, (s, q) in enumerate(crash[1][:2])]
    final = dict(base)
    for _i, _o, _l, sku, delta in rows:
        final[sku] += delta
    final[crash[1][2][0]] -= crash[1][2][1]
    return {'snapshot': snapshot, 'applied': applied, 'rows': rows, 'orders': orders, 'crash': crash,
            'logged': {s: base[s] + sum(r[4] for r in rows if r[3] == s) for s in SKUS},
            'final': final, 'shipped': shipped_from(rows + [(0, crash[0], 2, *crash[1][2][:1], -crash[1][2][1])])}


def shipped_from(rows):
    out = {}
    for row in rows:
        if row[4] < 0:
            out[row[3]] = out.get(row[3], 0) - row[4]
    return dict(sorted(out.items()))


def build(path, state, extra_pending=()):
    raw = sqlite3.connect(path)
    raw.executescript(RAW_SCHEMA)
    raw.executemany('INSERT INTO stock VALUES (?, ?)', sorted(state['snapshot'].items()))
    raw.executemany('INSERT INTO meta VALUES (?, ?)', [('applied_through', state['applied']), ('compacted_through', COMPACTED)])
    raw.executemany('INSERT INTO orders VALUES (?, ?, ?)', [(o, json.dumps(l), s) for o, l, s in state['orders']])
    raw.execute('INSERT INTO orders VALUES (?, ?, ?)', (state['crash'][0], json.dumps(state['crash'][1]), 'pending'))
    raw.executemany('INSERT INTO orders VALUES (?, ?, ?)', extra_pending)
    raw.executemany('INSERT INTO events VALUES (?, ?, ?, ?, ?)', state['rows'])
    raw.commit()
    raw.close()


def content(path):
    raw = sqlite3.connect(path)
    try:
        return {t: raw.execute(f'SELECT * FROM {t} ORDER BY 1, 2').fetchall() for t in ('stock', 'meta', 'orders', 'events')}
    finally:
        raw.close()


def check(workspace):
    work = Path(workspace).resolve()
    sys.path.insert(0, str(work))
    from db import connect, iter_events
    from projection import catch_up, stock
    from recovery import recover
    from report import shipped
    from service import receive, ship
    tmp = tempfile.TemporaryDirectory(ignore_cleanup_errors=True)
    scratch = Path(tmp.name)
    counter = iter(range(10**6))

    def fresh(state=None, extra_pending=()):
        path = scratch / f'w-{next(counter)}.db'
        if state is None:
            raw = sqlite3.connect(path)
            raw.executescript(RAW_SCHEMA + "\nINSERT INTO meta VALUES ('applied_through', 0);")
            raw.close()
        else:
            build(path, state, extra_pending)
        return path

    def cli(*args):
        return subprocess.run(['rtk', 'proxy', sys.executable, str(work / 'cli.py'), *map(str, args)],
                              text=True, capture_output=True, timeout=20, cwd=work)

    def events(path):
        return content(path)['events']

    def assert_recovered(test, path, state):
        got = content(path)
        test.assertEqual(dict(got['stock']), state['final'])
        test.assertEqual(got['events'][:len(state['rows'])], state['rows'])
        tail = got['events'][len(state['rows']):]
        crash_id, lines = state['crash']
        test.assertEqual([r[1:] for r in tail], [(crash_id, 2, lines[2][0], -lines[2][1])])
        meta = dict(got['meta'])
        test.assertEqual((meta['applied_through'], meta['compacted_through']), (got['events'][-1][0], COMPACTED))
        test.assertEqual({s for _o, _l, s in got['orders']}, {'done'})

    class Acceptance(unittest.TestCase):
        def test_01_iter_events_pages(self):
            path = fresh()
            raw = sqlite3.connect(path)
            raw.executemany('INSERT INTO events(id, sku, delta) VALUES (?, ?, ?)', [(i, 'A', 1) for i in range(1, 251)])
            raw.execute('DELETE FROM events WHERE id IN (5, 100, 101, 202)')
            raw.commit()
            raw.close()
            ids = [i for i in range(1, 251) if i not in (5, 100, 101, 202)]
            conn = connect(str(path))
            for batch in (1, 7, 99, 100, 1000):
                for after in (0, 50, 101):
                    with self.subTest(batch=batch, after=after):
                        self.assertEqual([r[0] for r in iter_events(conn, after=after, batch=batch)], [i for i in ids if i > after])
            conn.close()

        def test_02_report_busy_and_quiet(self):
            for count in (12, 340):
                with self.subTest(count=count):
                    path = fresh()
                    raw = sqlite3.connect(path)
                    raw.executemany('INSERT INTO events(order_id, line_no, sku, delta) VALUES (?, 0, ?, ?)',
                                    [(f'o{i}', SKUS[i % 5], -(i % 4 + 1)) for i in range(count)] + [(None, 'AX-1', 50)])
                    raw.commit()
                    raw.close()
                    conn = connect(str(path))
                    self.assertEqual(shipped(conn), shipped_from(events(path)))
                    conn.close()

        def test_03_cli_report(self):
            state = scenario(3)
            path = fresh(state)
            got = cli(path, 'report')
            self.assertEqual(got.returncode, 0, got.stderr)
            self.assertEqual(json.loads(got.stdout), shipped_from(state['rows']))

        def test_04_catch_up_stale_projection(self):
            state = scenario(7)
            path = fresh(state)
            conn = connect(str(path))
            self.assertEqual(catch_up(conn), state['rows'][-1][0])
            conn.close()
            got = content(path)
            self.assertEqual(dict(got['stock']), state['logged'])
            self.assertEqual(dict(got['meta'])['applied_through'], state['rows'][-1][0])
            self.assertEqual(got['events'], state['rows'])

        def test_05_catch_up_idempotent(self):
            state = scenario(8)
            path = fresh(state)
            conn = connect(str(path))
            catch_up(conn)
            before = content(path)
            self.assertEqual(catch_up(conn), state['rows'][-1][0])
            conn.close()
            self.assertEqual(content(path), before)

        def test_06_recover_completes_missing_lines_only(self):
            state = scenario(11)
            path = fresh(state)
            conn = connect(str(path))
            self.assertEqual(recover(conn), [state['crash'][0]])
            conn.close()
            assert_recovered(self, path, state)

        def test_07_recover_idempotent(self):
            state = scenario(12)
            path = fresh(state)
            conn = connect(str(path))
            recover(conn)
            before = content(path)
            self.assertEqual(recover(conn), [])
            conn.close()
            self.assertEqual(content(path), before)

        def test_08_recover_all_or_nothing(self):
            for label, lines in (('json', '[["AX-1", 2'), ('qty', '[["AX-1", 0]]'), ('shape', '{"AX-1": 2}')):
                with self.subTest(label=label):
                    path = fresh(scenario(13), [('zzz-bad', lines, 'pending')])
                    before = content(path)
                    conn = connect(str(path))
                    with self.assertRaises(ValueError):
                        recover(conn)
                    conn.close()
                    self.assertEqual(content(path), before)

        def test_09_ship_retry_done_order(self):
            path = fresh()
            conn = connect(str(path))
            receive(conn, 'AX-1', 10)
            ship(conn, 'o1', [['AX-1', 2], ['BX-7', 1]])
            before = content(path)
            ship(conn, 'o1', [('AX-1', 2), ('BX-7', 1)])
            conn.close()
            self.assertEqual(content(path), before)
            self.assertEqual([r[1:] for r in before['events']], [(None, None, 'AX-1', 10), ('o1', 0, 'AX-1', -2), ('o1', 1, 'BX-7', -1)])
            self.assertEqual([(o, json.loads(l), st) for o, l, st in before['orders']], [('o1', [['AX-1', 2], ['BX-7', 1]], 'done')])

        def test_10_ship_conflict(self):
            path = fresh()
            conn = connect(str(path))
            ship(conn, 'o1', [['AX-1', 2]])
            before = content(path)
            for lines in ([['AX-1', 3]], [['AX-1', 2], ['BX-7', 1]]):
                with self.subTest(lines=lines), self.assertRaises(ValueError):
                    ship(conn, 'o1', lines)
            conn.close()
            self.assertEqual(content(path), before)

        def test_11_ship_validation_writes_nothing(self):
            path = fresh()
            conn = connect(str(path))
            bad = [('o2', []), ('o2', [['AX-1', 0]]), ('o2', [['AX-1', True]]), ('o2', [['AX-1', '2']]),
                   ('o2', [['', 1]]), ('o2', [['AX-1', 1], ['BX-7', -1]]), ('o2', 'AX-1:1'), ('o2', [['AX-1', 1, 2]]),
                   ('o2', [['AX-1', 1.0]]), ('', [['AX-1', 1]]), (None, [['AX-1', 1]]), (7, [['AX-1', 1]])]
            for order_id, lines in bad:
                with self.subTest(order_id=order_id, lines=lines), self.assertRaises(ValueError):
                    ship(conn, order_id, lines)
            conn.close()
            got = content(path)
            self.assertEqual((got['orders'], got['events']), ([], []))

        def test_12_ship_retry_completes_crashed_order(self):
            state = scenario(14)
            path = fresh(state)
            conn = connect(str(path))
            ship(conn, *state['crash'])
            ship(conn, *state['crash'])
            catch_up(conn)
            conn.close()
            assert_recovered(self, path, state)

        def test_13_stock_reads_only(self):
            state = scenario(15)
            path = fresh(state)
            before = content(path)
            conn = connect(str(path))
            self.assertEqual(stock(conn), dict(sorted(state['snapshot'].items())))
            conn.close()
            got = cli(path, 'stock')
            self.assertEqual((got.returncode, json.loads(got.stdout)), (0, state['snapshot']), got.stderr)
            self.assertEqual(content(path), before)

        def test_14_cli_errors(self):
            path = fresh()
            self.assertEqual(cli(path, 'ship', 'o1', 'AX-1:2').returncode, 0)
            for args in (('ship', 'o1', 'AX-1:3'), ('ship', 'o2', 'AX-1:0'), ('ship', 'o3', 'AX-1:x'), ('ship', 'o4', 'AX-1')):
                got = cli(path, *args)
                with self.subTest(args=args):
                    self.assertEqual(got.returncode, 2)
                    self.assertTrue(got.stderr.strip().startswith('error:'), got.stderr)
                    self.assertNotIn('Traceback', got.stderr)
            self.assertEqual([r[1:] for r in events(path)], [('o1', 0, 'AX-1', -2)])

        def test_15_cli_recover(self):
            state = scenario(16)
            path = fresh(state)
            got = cli(path, 'recover')
            self.assertEqual((got.returncode, json.loads(got.stdout)), (0, [state['crash'][0]]), got.stderr)
            got = cli(path, 'recover')
            self.assertEqual((got.returncode, json.loads(got.stdout)), (0, []), got.stderr)
            assert_recovered(self, path, state)
            self.assertEqual(json.loads(cli(path, 'report').stdout), state['shipped'])

        def test_16_delivered_warehouse_recovered(self):
            state = scenario(1)
            assert_recovered(self, work / 'warehouse.db', state)

        def test_17_regression_receive_ship_refresh(self):
            conn = connect(':memory:')
            receive(conn, 'A', 10)
            ship(conn, 'o', [['A', 3]])
            self.assertEqual(catch_up(conn), 2)
            receive(conn, 'B', 1)
            self.assertEqual(catch_up(conn), 3)
            self.assertEqual((stock(conn), shipped(conn)), ({'A': 7, 'B': 1}, {'A': 3}))

        def test_18_catch_up_interrupted(self):
            state = scenario(19)
            path = fresh(state)
            raw = sqlite3.connect(path)
            raw.executescript("""CREATE TABLE fuse (n INTEGER); INSERT INTO fuse VALUES (120);
                CREATE TRIGGER fuse_i BEFORE INSERT ON stock BEGIN UPDATE fuse SET n = n - 1;
                  SELECT RAISE(ABORT, 'injected failure') WHERE (SELECT n FROM fuse) <= 0; END;
                CREATE TRIGGER fuse_u BEFORE UPDATE ON stock BEGIN UPDATE fuse SET n = n - 1;
                  SELECT RAISE(ABORT, 'injected failure') WHERE (SELECT n FROM fuse) <= 0; END;""")
            raw.close()
            conn = connect(str(path))
            try:
                catch_up(conn)
            except Exception:
                pass  # the injected failure; an implementation that writes few rows may not reach it
            conn.close()
            raw = sqlite3.connect(path)
            raw.executescript('DROP TRIGGER fuse_i; DROP TRIGGER fuse_u; DROP TABLE fuse;')
            raw.close()
            conn = connect(str(path))
            self.assertEqual(catch_up(conn), state['rows'][-1][0])
            conn.close()
            got = content(path)
            self.assertEqual(dict(got['stock']), state['logged'])
            self.assertEqual(dict(got['meta'])['applied_through'], state['rows'][-1][0])

        def test_19_ship_failure_between_lines_writes_nothing(self):
            path = fresh()
            raw = sqlite3.connect(path)
            raw.execute("CREATE TRIGGER boom BEFORE INSERT ON events WHEN NEW.line_no = 1 "
                        "BEGIN SELECT RAISE(ABORT, 'injected failure'); END")
            raw.commit()
            raw.close()
            conn = connect(str(path))
            with self.assertRaises(Exception):
                ship(conn, 'o9', [['AX-1', 1], ['BX-7', 2], ['CX-3', 1]])
            conn.close()
            got = content(path)
            self.assertEqual((got['orders'], got['events']), ([], []))

        def test_20_recover_fills_gap_by_line_number(self):
            state = scenario(21)
            gap = ('gap-order', [['AX-1', 1], ['BX-7', 2], ['CX-3', 3]])
            path = fresh(state, [(gap[0], json.dumps(gap[1]), 'pending')])
            n = state['rows'][-1][0] + 1
            raw = sqlite3.connect(path)
            raw.executemany('INSERT INTO events VALUES (?, ?, ?, ?, ?)', [(n, gap[0], 0, 'AX-1', -1), (n + 1, gap[0], 2, 'CX-3', -3)])
            raw.commit()
            raw.close()
            conn = connect(str(path))
            self.assertEqual(recover(conn), [gap[0], state['crash'][0]])
            conn.close()
            got = content(path)
            self.assertEqual(sorted(r[2:] for r in got['events'] if r[1] == gap[0]), [(0, 'AX-1', -1), (1, 'BX-7', -2), (2, 'CX-3', -3)])
            self.assertEqual(sorted(r[2] for r in got['events'] if r[1] == state['crash'][0]), [0, 1, 2])
            expected = dict(state['final'])
            for sku, qty in gap[1]:
                expected[sku] -= qty
            self.assertEqual(dict(got['stock']), expected)

    try:
        return unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Acceptance)).wasSuccessful()
    finally:
        tmp.cleanup()


def seed_warehouse(path):
    """Production snapshot for every arm: scenario 1 in its interrupted state."""
    build(path, scenario(1))


if __name__ == '__main__':
    raise SystemExit(0 if check(sys.argv[1]) else 1)
