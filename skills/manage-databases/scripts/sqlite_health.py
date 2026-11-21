#!/usr/bin/env python3
"""sqlite-health: read-only checkup of a SQLite database, plus query plans for the queries you care about.

usage: python sqlite_health.py <db> [--query "SELECT ..."]... [--json]
Opens the file read-only (never changes it). Exit 1 if integrity or foreign-key checks fail.
"""
import argparse
import json
import os
import sqlite3
import sys


def health(path, queries=()):
    con = sqlite3.connect(f"file:{path}?mode=ro", uri=True)
    q = lambda sql: con.execute(sql).fetchall()
    r = {"sqlite": sqlite3.sqlite_version, "bytes": os.path.getsize(path), "findings": []}
    add = lambda sev, msg: r["findings"].append({"severity": sev, "msg": msg})
    wal = path + "-wal"
    if os.path.exists(wal):
        r["wal_bytes"] = os.path.getsize(wal)
    r["journal_mode"] = q("PRAGMA journal_mode")[0][0]
    pages, free = q("PRAGMA page_count")[0][0], q("PRAGMA freelist_count")[0][0]
    r["free_ratio"] = round(free / pages, 3) if pages else 0
    if pages and free / pages > 0.25:
        add("WARN", f"{free / pages:.0%} of pages are free; VACUUM (or VACUUM INTO a copy) would reclaim them")
    ok = q("PRAGMA quick_check")
    if ok != [("ok",)]:
        add("FAIL", f"quick_check: {ok[:5]}")
    fk = q("PRAGMA foreign_key_check")
    if fk:
        add("FAIL", f"{len(fk)} foreign-key violations, first: {fk[0]}")
    r["tables"] = {}
    for (name,) in q("SELECT name FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'"):
        # Virtual tables (FTS5 and friends) can need modules this build lacks; count what we can.
        try:
            rows = q(f'SELECT count(*) FROM "{name}"')[0][0]
        except sqlite3.Error as e:
            r["tables"][name] = {"rows": None, "note": str(e)}
            continue
        idx = [i[1] for i in q(f'PRAGMA index_list("{name}")')]
        r["tables"][name] = {"rows": rows, "indexes": idx}
        if rows > 10_000 and not idx and q(f'PRAGMA table_info("{name}")') and not any(c[5] for c in q(f'PRAGMA table_info("{name}")')):
            add("WARN", f'"{name}" has {rows} rows, no primary key and no index')
    if r["journal_mode"] != "wal" and r["tables"]:
        add("INFO", f"journal_mode={r['journal_mode']}; WAL lets readers run during a write (not on network filesystems)")
    r["plans"] = []
    for sql in queries:
        try:
            plan = [row[3] for row in q("EXPLAIN QUERY PLAN " + sql)]
        except sqlite3.Error as e:
            add("FAIL", f"query failed to plan: {e}: {sql}")
            continue
        r["plans"].append({"sql": sql, "plan": plan})
        for step in plan:
            if step.startswith("SCAN ") and "USING" not in step:
                add("WARN", f"full scan ({step}) in: {sql}; fine for small tables, otherwise index the filter or join column")
            if "USE TEMP B-TREE" in step:
                add("INFO", f"sort or group without an index ({step}) in: {sql}")
    con.close()
    return r


def main():
    ap = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    ap.add_argument("db")
    ap.add_argument("--query", action="append", default=[])
    ap.add_argument("--json", action="store_true")
    a = ap.parse_args()
    r = health(a.db, a.query)
    if a.json:
        print(json.dumps(r, indent=2))
    else:
        print(f"SQLite {r['sqlite']}, {r['bytes']:,} bytes, journal {r['journal_mode']}, free {r['free_ratio']:.0%}")
        for name, t in r["tables"].items():
            print(f"  {name}: {t['rows']} rows, indexes {t.get('indexes', '-')}")
        for p in r["plans"]:
            print(f"\n{p['sql']}\n  " + "\n  ".join(p["plan"]))
        for f in r["findings"]:
            print(f"{f['severity']:4} {f['msg']}")
    sys.exit(1 if any(f["severity"] == "FAIL" for f in r["findings"]) else 0)


if __name__ == "__main__":
    main()
