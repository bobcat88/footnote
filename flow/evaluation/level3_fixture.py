"""Frozen L3 persistence-and-compatibility fixture and independent acceptance suite."""

import hashlib
import json
import re
import sqlite3
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

WRITE = re.compile(r'^\s*(ALTER|INSERT|UPDATE|DELETE|DROP|REPLACE|CREATE(?!.*IF NOT EXISTS)|PRAGMA\s+USER_VERSION\s*=)', re.I | re.S)
END = re.compile(r'^\s*(COMMIT|END|ROLLBACK)\b', re.I)

SOURCES = {
    'store.py': '''import sqlite3

SCHEMA = "CREATE TABLE IF NOT EXISTS entries (id INTEGER PRIMARY KEY, account TEXT NOT NULL, cents INTEGER NOT NULL)"


def connect(path):
    conn = sqlite3.connect(path)
    conn.execute(SCHEMA)
    conn.execute("PRAGMA user_version = 1")
    conn.commit()
    return conn


def add_entry(conn, account, cents):
    cur = conn.execute("INSERT INTO entries(account, cents) VALUES (?, ?)", (account, cents))
    conn.commit()
    return cur.lastrowid


def entries(conn, account=None):
    sql, args = "SELECT id, account, cents FROM entries", ()
    if account is not None:
        sql, args = sql + " WHERE account = ?", (account,)
    return [{"id": i, "account": a, "cents": c} for i, a, c in conn.execute(sql + " ORDER BY id", args)]
''',
    'ledger.py': '''from store import entries


def balance(conn, account):
    return sum(e["cents"] for e in entries(conn, account))


def statement(conn, account):
    running, rows = 0, []
    for e in entries(conn, account):
        running += e["cents"]
        rows.append({"id": e["id"], "cents": e["cents"], "balance": running})
    return rows
''',
    'archive.py': '''import json
from store import add_entry, entries


def export_json(conn, path):
    with open(path, "w", encoding="utf-8") as f:
        json.dump(entries(conn), f)


def import_json(conn, path):
    with open(path, encoding="utf-8") as f:
        rows = json.load(f)
    return [add_entry(conn, r["account"], r["cents"]) for r in rows]
''',
    'cli.py': '''import argparse
import json
from store import connect, add_entry
from ledger import balance
from archive import export_json, import_json


def main(argv=None):
    parser = argparse.ArgumentParser()
    parser.add_argument("db")
    sub = parser.add_subparsers(dest="cmd", required=True)
    add = sub.add_parser("add")
    add.add_argument("account")
    add.add_argument("cents", type=int)
    sub.add_parser("balance").add_argument("account")
    sub.add_parser("export").add_argument("path")
    sub.add_parser("import").add_argument("path")
    args = parser.parse_args(argv)
    conn = connect(args.db)
    if args.cmd == "add":
        out = add_entry(conn, args.account, args.cents)
    elif args.cmd == "balance":
        out = balance(conn, args.account)
    elif args.cmd == "export":
        export_json(conn, args.path)
        out = "ok"
    else:
        out = import_json(conn, args.path)
    print(json.dumps(out))


if __name__ == "__main__":
    main()
''',
    'test_existing.py': '''import os
import tempfile
import unittest
from store import connect, add_entry, entries
from ledger import balance, statement
from archive import export_json, import_json


class Existing(unittest.TestCase):
    def test_balance(self):
        conn = connect(":memory:")
        add_entry(conn, "a", 250)
        add_entry(conn, "a", -50)
        self.assertEqual(balance(conn, "a"), 200)

    def test_statement(self):
        conn = connect(":memory:")
        add_entry(conn, "a", 5)
        self.assertEqual(statement(conn, "a"), [{"id": 1, "cents": 5, "balance": 5}])

    def test_roundtrip(self):
        conn = connect(":memory:")
        add_entry(conn, "a", 7)
        with tempfile.TemporaryDirectory() as tmp:
            path = os.path.join(tmp, "out.json")
            export_json(conn, path)
            other = connect(":memory:")
            import_json(other, path)
        self.assertEqual(balance(other, "a"), 7)
        self.assertEqual(len(entries(other)), 1)
''',
}
CONTRACT = '''Ledger storage contract v2. Amounts are integer cents (bool is not an integer).
Accounts are non-empty strings. Currencies are exactly three uppercase ASCII letters.
The schema version lives in PRAGMA user_version. Versions 0 and 1 are the legacy
format: table entries(id, account, cents), and every legacy row is EUR. Version 2 adds
a currency column. connect(path) opens version 0, 1 and 2 files and upgrades older ones
to version 2 in place. It refuses files newer than version 2 with ValueError and must
not modify them. migrate(conn) performs the upgrade and returns the resulting version.
It is idempotent, keeps every row and id, and is all-or-nothing: a legacy row that is
invalid under this contract aborts the upgrade with ValueError and leaves the file
exactly as it was, still at its old version.
Existing callers keep working. add_entry(conn, account, cents) stores EUR and returns
the new id; currency is a keyword-only option. balance(conn, account) and
statement(conn, account) keep their return shapes and mean EUR; both accept a
keyword-only currency. balances(conn, account) returns {currency: cents} for the
currencies the account holds. Rows returned by entries() gain a "currency" key.
The release notes in this workspace are part of this contract.
Export format 1 is the legacy JSON list of {"id", "account", "cents"} objects. It stays
the default and can only hold EUR rows. Format 2 is {"format": 2, "entries": [...]},
whose rows also carry "currency". export_json takes keyword-only format_version.
import_json reads both formats. Invalid input of any kind raises ValueError.
'''
DECISION = '''Compatibility decision: never silently drop or relabel money. Exporting a non-EUR
row in format 1 is an error, and a failed export leaves any existing file at the
target path untouched. Imports are all-or-nothing. import_json takes keyword-only
mode: the default "append" keeps the legacy behaviour (rows receive new ids), while
"restore" keeps the file's ids, skips rows identical to stored ones so repeating it
is harmless, and rejects a stored id whose content differs. import_json returns the
ids it inserted. Upgrades run inside one explicit transaction; Python's sqlite3 does
not wrap schema statements in a transaction for you. CLI: add and balance accept
--currency, export accepts --format 1|2, import accepts --restore. CLI domain errors
exit with status 2 and print one line starting "error:" on stderr, never a traceback.
'''
OBJECTIVE = '''Implement the ledger storage migration to schema version 2 across this workspace.
Read CONTRACT.txt and DECISION.txt; they are the compatibility contract. Existing
version 0/1 database files and JSON exports produced by the current code are in
real use and must keep working, as must existing callers of the public functions.
Add multi-currency support behind explicit keyword-only options, a migrate(conn)
function in store.py, balances(conn, account) in ledger.py, format 2 exports, a
restore import mode and the CLI options. Keep the upgrade idempotent and
all-or-nothing, never lose or relabel rows, refuse newer files without touching them,
and raise ValueError for invalid input.
Add focused regression tests in test_migration.py, retain existing tests, and write
delivery.txt with changes, checks, and unresolved issues. Standard library only.
Owned outputs: store.py, ledger.py, archive.py, cli.py, test_migration.py,
delivery.txt. Do not edit task, contract, decision or existing tests.
One delivery, no parent repair or stronger-model fallback. No network, installs,
commits, child agents or unrelated files. An independent acceptance step checks the delivery. Aim for at most
16 model responses and 10 minutes, requested limits rather than provider caps.
Stop after local verification and keep the final response short.
'''
CHANGELOG = '''Release notes (newest first)

Schema version 2 upgrade: when a version 0 or 1 file is upgraded, trailing spaces
(U+0020 only) are removed from every account value; no other character changes.
Example: a legacy row with account "acme  " reads back as "acme" and counts toward
the balance of "acme". A row whose account is empty after the removal is invalid
and aborts the upgrade like any other invalid legacy row. Files already at version 2
are never rewritten.

0.3.2: account values are stored exactly as entered, trailing spaces included.
Superseded by the schema version 2 upgrade note above.

0.3: the command-line tool stopped trimming account names, so version 0 and 1 files
written by 0.3 to 0.3.2 can hold accounts with trailing spaces.
'''
OWNED = ['store.py', 'ledger.py', 'archive.py', 'cli.py', 'test_migration.py', 'delivery.txt']
LEGACY_SCHEMA = 'CREATE TABLE entries (id INTEGER PRIMARY KEY, account TEXT NOT NULL, cents INTEGER NOT NULL)'


def legacy_db(path, rows, version=0, schema=LEGACY_SCHEMA):
    raw = sqlite3.connect(path)
    raw.execute(schema)
    if rows:
        raw.executemany('INSERT INTO entries VALUES (' + ','.join('?' * len(rows[0])) + ')', rows)
    raw.execute(f'PRAGMA user_version = {version}')
    raw.commit()
    raw.close()


def raw_state(path):
    raw = sqlite3.connect(path)
    try:
        return (raw.execute('PRAGMA user_version').fetchone()[0],
                [c[1] for c in raw.execute('PRAGMA table_info(entries)')],
                raw.execute('SELECT * FROM entries ORDER BY id').fetchall())
    finally:
        raw.close()


def digest(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def check(workspace):
    work = Path(workspace).resolve()
    sys.path.insert(0, str(work))
    import store
    from store import connect, add_entry, entries
    from ledger import balance, statement
    from archive import export_json, import_json
    tmp = tempfile.TemporaryDirectory(ignore_cleanup_errors=True)
    scratch = Path(tmp.name)
    counter = iter(range(10**6))

    def fresh(name='db'):
        return scratch / f'{name}-{next(counter)}.sqlite'

    def cli(*args):
        return subprocess.run(['rtk', 'proxy', sys.executable, str(work / 'cli.py'), *map(str, args)],
                              text=True, capture_output=True, timeout=20, cwd=work)

    def ledger_with(rows):
        conn = connect(':memory:')
        for account, cents, currency in rows:
            add_entry(conn, account, cents, currency=currency)
        return conn

    class Acceptance(unittest.TestCase):
        def test_01_legacy_upgrade(self):
            for version in (0, 1):
                with self.subTest(version=version):
                    path = fresh()
                    legacy_db(path, [(1, 'a', 250), (2, 'b', -40), (5, 'a', 10)], version)
                    conn = connect(str(path))
                    self.assertEqual(entries(conn), [
                        {'id': 1, 'account': 'a', 'cents': 250, 'currency': 'EUR'},
                        {'id': 2, 'account': 'b', 'cents': -40, 'currency': 'EUR'},
                        {'id': 5, 'account': 'a', 'cents': 10, 'currency': 'EUR'}])
                    conn.close()
                    state = raw_state(path)
                    self.assertEqual(state[0], 2)
                    self.assertIn('currency', state[1])

        def test_02_migrate_idempotent(self):
            path = fresh()
            legacy_db(path, [(3, 'a', 1), (9, 'b', 2)])
            conn = connect(str(path))
            self.assertEqual(store.migrate(conn), 2)
            self.assertEqual(store.migrate(conn), 2)
            conn.close()
            before = raw_state(path)
            connect(str(path)).close()
            self.assertEqual(raw_state(path), before)
            self.assertEqual(len(before[2]), 2)

        def test_03_upgrade_all_or_nothing(self):
            for bad in ((2, 'b', 'abc'), (2, 'b', 1.5), (2, '', 4)):
                with self.subTest(bad=bad):
                    path = fresh()
                    legacy_db(path, [(1, 'a', 5), bad], 0)
                    before = raw_state(path)
                    with self.assertRaises(ValueError):
                        connect(str(path)).close()
                    self.assertEqual(raw_state(path), before)

        def test_04_newer_refused_untouched(self):
            path = fresh()
            legacy_db(path, [(1, 'a', 5, 'EUR')], 3,
                      'CREATE TABLE entries (id INTEGER PRIMARY KEY, account TEXT NOT NULL, cents INTEGER NOT NULL, currency TEXT NOT NULL)')
            before = digest(path)
            with self.assertRaises(ValueError):
                connect(str(path)).close()
            self.assertEqual(digest(path), before)
            self.assertEqual(raw_state(path)[0], 3)

        def test_05_legacy_api(self):
            conn = connect(':memory:')
            ident = add_entry(conn, 'acc', 250)
            self.assertIs(type(ident), int)
            with self.assertRaises(TypeError):
                add_entry(conn, 'acc', 1, 'USD')
            self.assertEqual(balance(conn, 'acc'), 250)
            self.assertEqual(statement(conn, 'acc'), [{'id': ident, 'cents': 250, 'balance': 250}])
            self.assertEqual(entries(conn, 'acc')[0]['currency'], 'EUR')

        def test_06_currency_api(self):
            from ledger import balances
            conn = ledger_with([('acc', 100, 'USD'), ('acc', 250, 'EUR'), ('acc', -30, 'USD'), ('zz', 9, 'GBP')])
            self.assertEqual(balance(conn, 'acc'), 250)
            self.assertEqual(balance(conn, 'acc', currency='USD'), 70)
            self.assertEqual(balance(conn, 'acc', currency='JPY'), 0)
            self.assertEqual(balances(conn, 'acc'), {'EUR': 250, 'USD': 70})
            self.assertEqual([(r['cents'], r['balance']) for r in statement(conn, 'acc', currency='USD')], [(100, 100), (-30, 70)])
            self.assertEqual([(r['cents'], r['balance']) for r in statement(conn, 'acc')], [(250, 250)])

        def test_07_validation(self):
            conn = connect(':memory:')
            add_entry(conn, 'acc', 1)
            cases = [dict(account='acc', cents=1, currency=c) for c in ('usd', 'EURO', '', None, 1, 'E1R', 'ÉUR')]
            cases += [dict(account='acc', cents=c, currency='EUR') for c in (True, 1.5, '3', None)]
            cases += [dict(account=a, cents=1, currency='EUR') for a in ('', None, 3)]
            for case in cases:
                with self.subTest(case=case), self.assertRaises(ValueError):
                    add_entry(conn, case['account'], case['cents'], currency=case['currency'])
            for bad in ('usd', None):
                with self.subTest(balance=bad), self.assertRaises(ValueError):
                    balance(conn, 'acc', currency=bad)
            self.assertEqual(len(entries(conn)), 1)

        def test_08_export_v1_default(self):
            conn = ledger_with([('a', 5, 'EUR'), ('b', 7, 'EUR')])
            path = scratch / 'v1.json'
            export_json(conn, path)
            self.assertEqual(json.loads(path.read_text(encoding='utf-8')),
                             [{'id': 1, 'account': 'a', 'cents': 5}, {'id': 2, 'account': 'b', 'cents': 7}])

        def test_09_export_refusals_keep_file(self):
            conn = ledger_with([('a', 5, 'EUR'), ('a', 3, 'USD')])
            path = scratch / 'keep.json'
            for kwargs in ({}, {'format_version': 1}, {'format_version': 3}):
                path.write_text('sentinel', encoding='utf-8')
                with self.subTest(kwargs=kwargs), self.assertRaises(ValueError):
                    export_json(conn, path, **kwargs)
                self.assertEqual(path.read_text(encoding='utf-8'), 'sentinel')

        def test_10_export_v2_restore_roundtrip(self):
            conn = ledger_with([('a', 5, 'EUR'), ('b', 3, 'USD'), ('a', -2, 'USD')])
            conn.execute('DELETE FROM entries WHERE id = 2')
            conn.commit()
            path = scratch / 'v2.json'
            export_json(conn, path, format_version=2)
            self.assertEqual(json.loads(path.read_text(encoding='utf-8')), {'format': 2, 'entries': [
                {'id': 1, 'account': 'a', 'cents': 5, 'currency': 'EUR'},
                {'id': 3, 'account': 'a', 'cents': -2, 'currency': 'USD'}]})
            other = connect(':memory:')
            self.assertEqual(import_json(other, path, mode='restore'), [1, 3])
            self.assertEqual(entries(other), entries(conn))
            self.assertEqual(import_json(other, path, mode='restore'), [])
            self.assertEqual(entries(other), entries(conn))

        def test_11_import_append_legacy(self):
            path = scratch / 'legacy.json'
            path.write_text(json.dumps([{'id': 1, 'account': 'x', 'cents': 4}, {'id': 2, 'account': 'y', 'cents': 6}]), encoding='utf-8')
            conn = ledger_with([('a', 1, 'EUR'), ('a', 2, 'USD')])
            self.assertEqual(import_json(conn, path), [3, 4])
            self.assertEqual(entries(conn)[2:], [{'id': 3, 'account': 'x', 'cents': 4, 'currency': 'EUR'},
                                                 {'id': 4, 'account': 'y', 'cents': 6, 'currency': 'EUR'}])
            v2 = scratch / 'append-v2.json'
            v2.write_text(json.dumps({'format': 2, 'entries': [{'id': 1, 'account': 'z', 'cents': 8, 'currency': 'GBP'}]}), encoding='utf-8')
            self.assertEqual(import_json(conn, v2), [5])
            self.assertEqual(entries(conn, 'z'), [{'id': 5, 'account': 'z', 'cents': 8, 'currency': 'GBP'}])

        def test_12_import_all_or_nothing(self):
            conn = ledger_with([('a', 1, 'EUR')])
            bad_files = {
                'conflict': ({'format': 2, 'entries': [{'id': 7, 'account': 'q', 'cents': 2, 'currency': 'EUR'},
                                                       {'id': 1, 'account': 'a', 'cents': 2, 'currency': 'EUR'}]}, 'restore'),
                'bad_row': ([{'id': 8, 'account': 'q', 'cents': 2}, {'id': 9, 'account': 'q', 'cents': '3'}], 'append'),
                'missing_key': ([{'id': 8, 'account': 'q', 'cents': 2}, {'id': 9, 'account': 'q'}], 'append'),
                'bad_currency': ({'format': 2, 'entries': [{'id': 8, 'account': 'q', 'cents': 2, 'currency': 'eur'}]}, 'append'),
                'format': ({'format': 3, 'entries': []}, 'append'),
                'shape': ('text', 'append'),
                'mode': ([{'id': 8, 'account': 'q', 'cents': 2}], 'merge'),
            }
            for label, (body, mode) in bad_files.items():
                path = scratch / f'bad-{label}.json'
                path.write_text(json.dumps(body), encoding='utf-8')
                with self.subTest(label=label), self.assertRaises(ValueError):
                    import_json(conn, path, mode=mode)
            broken = scratch / 'broken.json'
            broken.write_text('[{"id": 1,', encoding='utf-8')
            with self.assertRaises(ValueError):
                import_json(conn, broken)
            self.assertEqual(entries(conn), [{'id': 1, 'account': 'a', 'cents': 1, 'currency': 'EUR'}])

        def test_13_cli_legacy_file(self):
            path = fresh()
            legacy_db(path, [(1, 'acc', 250), (4, 'acc', -50)], 1)
            got = cli(path, 'balance', 'acc')
            self.assertEqual(got.returncode, 0, got.stderr)
            self.assertEqual(json.loads(got.stdout), 200)
            got = cli(path, 'add', 'acc', 5)
            self.assertEqual((got.returncode, json.loads(got.stdout)), (0, 5), got.stderr)
            out = scratch / 'cli-v1.json'
            got = cli(path, 'export', out)
            self.assertEqual(got.returncode, 0, got.stderr)
            self.assertEqual([r['id'] for r in json.loads(out.read_text(encoding='utf-8'))], [1, 4, 5])
            self.assertEqual(raw_state(path)[0], 2)

        def test_14_cli_options_and_errors(self):
            path = fresh()
            self.assertEqual(cli(path, 'add', 'acc', 7, '--currency', 'USD').returncode, 0)
            got = cli(path, 'balance', 'acc', '--currency', 'USD')
            self.assertEqual((got.returncode, json.loads(got.stdout)), (0, 7), got.stderr)
            self.assertEqual(json.loads(cli(path, 'balance', 'acc').stdout), 0)
            out = scratch / 'cli-v2.json'
            self.assertEqual(cli(path, 'export', out, '--format', 2).returncode, 0)
            restored = fresh()
            got = cli(restored, 'import', out, '--restore')
            self.assertEqual((got.returncode, json.loads(got.stdout)), (0, [1]), got.stderr)
            self.assertEqual(json.loads(cli(restored, 'import', out, '--restore').stdout), [])
            newer = fresh()
            legacy_db(newer, [], 3)
            for args in ((path, 'add', 'acc', 5, '--currency', 'usd'), (path, 'export', scratch / 'no.json'), (newer, 'balance', 'acc')):
                got = cli(*args)
                with self.subTest(args=args[1:]):
                    self.assertEqual(got.returncode, 2)
                    self.assertTrue(got.stderr.strip().startswith('error:'), got.stderr)
                    self.assertNotIn('Traceback', got.stderr)

        def test_15_large_legacy_upgrade(self):
            path = fresh()
            rows = [(i * 3 + 1, f'acc{i % 17}', (i * 37) % 1001 - 500) for i in range(3000)]
            legacy_db(path, rows, 0)
            conn = connect(str(path))
            got = entries(conn)
            conn.close()
            self.assertEqual([(r['id'], r['account'], r['cents']) for r in got], rows)
            self.assertEqual({r['currency'] for r in got}, {'EUR'})

        def test_16_upgrade_in_one_transaction(self):
            # DECISION: the upgrade runs inside one explicit transaction (validating first is not enough).
            real = sqlite3.connect
            for version in (0, 1):
                seen = []

                def traced(*args, **kwargs):
                    conn = real(*args, **kwargs)
                    conn.set_trace_callback(lambda sql, c=conn: seen.append((sql, c.in_transaction)))
                    return conn
                path = fresh()
                legacy_db(path, [(1, 'a', 5), (2, 'b', 7)], version)
                sqlite3.connect = traced
                try:
                    connect(str(path)).close()
                finally:
                    sqlite3.connect = real
                with self.subTest(version=version):
                    writes = [i for i, (sql, _t) in enumerate(seen) if WRITE.match(sql)]
                    self.assertTrue(writes, 'no upgrade statement observed')
                    self.assertEqual([seen[i][0] for i in writes if not seen[i][1]], [], 'upgrade statement outside a transaction')
                    self.assertFalse([s for s, _t in seen[writes[0]:writes[-1]] if END.match(s)], 'upgrade split across transactions')
                    self.assertTrue(any(re.match(r'^\s*(COMMIT|END)\b', s, re.I) for s, _t in seen[writes[-1]:]), 'upgrade never committed')
                    self.assertEqual(raw_state(path)[0], 2)

        def test_17_release_notes_accounts(self):
            for version in (0, 1):
                with self.subTest(version=version):
                    path = fresh()
                    legacy_db(path, [(1, 'acme  ', 5), (2, 'acme', 3), (3, ' lead', 4), (4, 'tab\t', 1)], version)
                    conn = connect(str(path))
                    self.assertEqual([(e['id'], e['account']) for e in entries(conn)], [(1, 'acme'), (2, 'acme'), (3, ' lead'), (4, 'tab\t')])
                    self.assertEqual(balance(conn, 'acme'), 8)
                    conn.close()
            path = fresh()
            legacy_db(path, [(1, 'a', 5), (2, '   ', 4)], 0)
            before = raw_state(path)
            with self.assertRaises(ValueError):
                connect(str(path)).close()
            self.assertEqual(raw_state(path), before)
            path = fresh()
            legacy_db(path, [(1, 'x ', 5, 'EUR')], 2,
                      'CREATE TABLE entries (id INTEGER PRIMARY KEY, account TEXT NOT NULL, cents INTEGER NOT NULL, currency TEXT NOT NULL)')
            conn = connect(str(path))
            self.assertEqual(entries(conn)[0]['account'], 'x ')
            conn.close()

    try:
        return unittest.TextTestRunner(verbosity=2).run(unittest.defaultTestLoader.loadTestsFromTestCase(Acceptance)).wasSuccessful()
    finally:
        tmp.cleanup()


if __name__ == '__main__':
    raise SystemExit(0 if check(sys.argv[1]) else 1)
