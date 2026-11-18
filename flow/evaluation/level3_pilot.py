"""Prepare five equal-source L3 arms. Never launches agents."""
import json
import random
import shutil
import subprocess
import sys
import time
from pathlib import Path

from .level3_fixture import SOURCES, CONTRACT, DECISION, OBJECTIVE, OWNED, CHANGELOG
from .measurement_adapter import MeasurementOnlyAdapter, DECISION as MEASURE_DECISION
from .live_smoke import NativeHandoff
from integration.footnote import ENGINE, PACK, Footnote, prepare, validate_context, sha
from orchestration import Governor
from orchestration.demo import task

STUDY = 'luna-medium-L3-v2'
PROJECT = 'flow-persistence-proof'
SKILL = 'sqlite-sql-performance'
RESPONSES, MINUTES = 16, 10

# Parent-only positive control, written independently of any worker output.
KNOWN_VALID = {
    'store.py': '''import re
import sqlite3

VERSION = 2
LEGACY = "CREATE TABLE IF NOT EXISTS entries (id INTEGER PRIMARY KEY, account TEXT NOT NULL, cents INTEGER NOT NULL)"


def check_currency(currency):
    if type(currency) is not str or not re.fullmatch(r"[A-Z]{3}", currency, re.ASCII):
        raise ValueError("currency must be three uppercase letters")
    return currency


def check_entry(account, cents, currency="EUR"):
    if type(account) is not str or not account:
        raise ValueError("account must be a non-empty string")
    if type(cents) is not int:
        raise ValueError("cents must be an integer")
    check_currency(currency)


def connect(path):
    conn = sqlite3.connect(path, isolation_level=None)
    try:
        migrate(conn)
    except BaseException:
        conn.close()
        raise
    return conn


def migrate(conn):
    version = conn.execute("PRAGMA user_version").fetchone()[0]
    if version > VERSION:
        raise ValueError(f"database version {version} is newer than {VERSION}")
    if version == VERSION:
        return VERSION
    conn.execute("BEGIN IMMEDIATE")
    try:
        conn.execute(LEGACY)
        conn.execute("UPDATE entries SET account = rtrim(account, ' ')")  # release notes: v2 upgrade
        for account, cents in conn.execute("SELECT account, cents FROM entries").fetchall():
            check_entry(account, cents)
        conn.execute("ALTER TABLE entries ADD COLUMN currency TEXT NOT NULL DEFAULT 'EUR'")
        conn.execute(f"PRAGMA user_version = {VERSION}")
        conn.execute("COMMIT")
    except BaseException:
        conn.execute("ROLLBACK")
        raise
    return VERSION


def add_entry(conn, account, cents, *, currency="EUR"):
    check_entry(account, cents, currency)
    cur = conn.execute("INSERT INTO entries(account, cents, currency) VALUES (?, ?, ?)", (account, cents, currency))
    return cur.lastrowid


def entries(conn, account=None):
    sql, args = "SELECT id, account, cents, currency FROM entries", ()
    if account is not None:
        sql, args = sql + " WHERE account = ?", (account,)
    return [{"id": i, "account": a, "cents": c, "currency": k} for i, a, c, k in conn.execute(sql + " ORDER BY id", args)]
''',
    'ledger.py': '''from store import entries, check_currency


def balance(conn, account, *, currency="EUR"):
    check_currency(currency)
    return sum(e["cents"] for e in entries(conn, account) if e["currency"] == currency)


def balances(conn, account):
    out = {}
    for e in entries(conn, account):
        out[e["currency"]] = out.get(e["currency"], 0) + e["cents"]
    return out


def statement(conn, account, *, currency="EUR"):
    check_currency(currency)
    running, rows = 0, []
    for e in entries(conn, account):
        if e["currency"] == currency:
            running += e["cents"]
            rows.append({"id": e["id"], "cents": e["cents"], "balance": running})
    return rows
''',
    'archive.py': '''import json
import os
from store import entries, check_entry


def export_json(conn, path, *, format_version=1):
    rows = entries(conn)
    if format_version == 1 and type(format_version) is int:
        if any(r["currency"] != "EUR" for r in rows):
            raise ValueError("format 1 can only hold EUR rows")
        data = [{"id": r["id"], "account": r["account"], "cents": r["cents"]} for r in rows]
    elif format_version == 2 and type(format_version) is int:
        data = {"format": 2, "entries": rows}
    else:
        raise ValueError("unknown export format")
    tmp = str(path) + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(data, f)
    os.replace(tmp, path)


def _read(path):
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    if isinstance(data, list):
        rows, legacy = data, True
    elif isinstance(data, dict) and data.get("format") == 2 and isinstance(data.get("entries"), list):
        rows, legacy = data["entries"], False
    else:
        raise ValueError("unknown import format")
    out = []
    for r in rows:
        if not isinstance(r, dict):
            raise ValueError("row must be an object")
        try:
            row = (r["id"], r["account"], r["cents"], "EUR" if legacy else r["currency"])
        except KeyError as exc:
            raise ValueError(f"missing {exc}") from None
        if type(row[0]) is not int:
            raise ValueError("id must be an integer")
        check_entry(*row[1:])
        out.append(row)
    return out


def import_json(conn, path, *, mode="append"):
    if mode not in ("append", "restore"):
        raise ValueError("mode must be append or restore")
    rows = _read(path)
    ids = []
    conn.execute("BEGIN IMMEDIATE")
    try:
        for ident, account, cents, currency in rows:
            if mode == "append":
                ids.append(conn.execute("INSERT INTO entries(account, cents, currency) VALUES (?, ?, ?)",
                                        (account, cents, currency)).lastrowid)
                continue
            old = conn.execute("SELECT account, cents, currency FROM entries WHERE id = ?", (ident,)).fetchone()
            if old is None:
                conn.execute("INSERT INTO entries VALUES (?, ?, ?, ?)", (ident, account, cents, currency))
                ids.append(ident)
            elif old != (account, cents, currency):
                raise ValueError(f"stored id {ident} differs")
        conn.execute("COMMIT")
    except BaseException:
        conn.execute("ROLLBACK")
        raise
    return ids
''',
    'cli.py': '''import argparse
import json
import sys
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
    add.add_argument("--currency", default="EUR")
    bal = sub.add_parser("balance")
    bal.add_argument("account")
    bal.add_argument("--currency", default="EUR")
    exp = sub.add_parser("export")
    exp.add_argument("path")
    exp.add_argument("--format", type=int, choices=(1, 2), default=1)
    imp = sub.add_parser("import")
    imp.add_argument("path")
    imp.add_argument("--restore", action="store_true")
    args = parser.parse_args(argv)
    try:
        conn = connect(args.db)
        if args.cmd == "add":
            out = add_entry(conn, args.account, args.cents, currency=args.currency)
        elif args.cmd == "balance":
            out = balance(conn, args.account, currency=args.currency)
        elif args.cmd == "export":
            export_json(conn, args.path, format_version=args.format)
            out = "ok"
        else:
            out = import_json(conn, args.path, mode="restore" if args.restore else "append")
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    print(json.dumps(out))
    return 0


if __name__ == "__main__":
    sys.exit(main())
''',
}


def run(destination, study=STUDY, model='gpt-6-luna', effort='medium', controls=True, folders=None):
    root=Path(destination).resolve()
    root.mkdir(parents=True,exist_ok=False)
    project=Path(__file__).resolve().parents[1]
    pack=PACK
    order=['baseline','skills','footnote_only','footnote','flow']
    random.Random(6119).shuffle(order)
    record={'study':study,'level':'L3','model':model,'options':{'reasoningEffort':effort},
            'order':order,'maximum_runs':5,'attempts_per_arm':1,'confirmation':False,
            'objective':OBJECTIVE,'requested_model_response_limit':RESPONSES,'requested_minutes':MINUTES,
            'hard_token_limit':None,'checker_sha256':sha(project/'evaluation/level3_fixture.py'),
            'limitations':['One synthetic persistence task, one run per arm',
                'Serial randomized order; cache and run variance prevent causal ranking',
                'Common parent preparation excluded from child usage; not production setup cost',
                'Measurement-only adapter; no predictions or dollar billing claims',
                'Shared checkout with prompt isolation, not a security sandbox',
                'Limits raised from L2 (12 responses, 8 minutes) for every arm: more modules and contract clauses'], 'arms':{}}
    for arm in order:
        began=time.perf_counter_ns()
        folder=Path(folders[arm]) if folders else root/arm
        work=folder/'workspace'
        work.mkdir(parents=True)
        for name,body in {**SOURCES,'TASK.txt':OBJECTIVE,'CONTRACT.txt':CONTRACT,'DECISION.txt':DECISION,'CHANGELOG.txt':CHANGELOG,
                          'pyproject.toml':'[project]\nname="'+PROJECT+'"\nversion="0.0.1"\n'}.items():
            (work/name).write_text(body,encoding='utf-8',newline='\n')
        item={'workspace':str(work),'initial_hashes':{p.name:sha(p) for p in work.iterdir()},
              'selected_skill':str(pack/SKILL/'SKILL.md') if arm in ('skills','footnote','flow') else None}
        # Every arm gets the same isolated setup; only assigned context delivery differs.
        (folder/'bin').mkdir()
        shutil.copy2(ENGINE,folder/'bin/footnote.exe')
        (folder/'docs').mkdir()
        (folder/'docs/contract.txt').write_text(CONTRACT+'\n'+DECISION,encoding='utf-8')
        client=Footnote(folder/'bin/footnote.exe',folder)
        client.call('remember',db=client.state/'memory.db',payload=[{'text':DECISION,'kind':'fix','scope':PROJECT,'observed_at':'2026-11-18','code':['store.py','connect']}])
        client.call('index','--root',str(folder/'docs'),'--allow',str(folder/'docs'),db=folder/'library.db')
        client.call('code-index',str(work))
        if arm in ('footnote_only','footnote','flow'):
            context=prepare(client,project=PROJECT,workspace=work,symbol='connect',query='compatibility',documents=[folder/'docs/contract.txt'])
            validate_context(context)
            (work/'footnote-context.json').write_text(json.dumps(context,indent=2),encoding='utf-8')
            item['context_bytes']=context['context_bytes']
        item['footnote_calls']=client.calls
        adapter=MeasurementOnlyAdapter(study+'-'+arm)
        if arm=='flow':
            governor=Governor(folder/'project.sqlite',work,pack)
            try:
                contract=task('implement','implementer',OWNED,[{'kind':'contains','path':'delivery.txt','value':'checks'}],skills=[SKILL],context=['TASK.txt','CONTRACT.txt','DECISION.txt','footnote-context.json'])
                contract.update(objective=OBJECTIVE,max_attempts=1,constraints=['Isolated evaluation','Independent external acceptance required'],permitted_actions=['Read assigned inputs','Edit owned files','Run local tests'],stop_condition='Return one delivery after local checks; do not delegate')
                governor.plan({'schema_version':1,'goal':OBJECTIVE,'decisions':[MEASURE_DECISION],'tasks':[contract]})
                executor=NativeHandoff(folder/'handoff.json')
                executor.engine='t3:codex:'+model+':'+effort
                try: governor.dispatch('implement',executor,adapter)
                except SystemExit: pass
                item['governor_snapshot']=governor.snapshot()
            finally: governor.close()
        else:
            adapter.activate({'mode':'real','task_id':'implement','attempt_id':arm+'-1','project_decisions':[MEASURE_DECISION],'runtime':{'adapter':'flow-evaluation:t3'},'objective':OBJECTIVE,'workspace':str(work)})
        item['measurement_activation']=adapter.snapshot()
        item['prepare_ms']=(time.perf_counter_ns()-began)/1e6
        record['arms'][arm]=item
    assert len({json.dumps(a['initial_hashes'],sort_keys=True) for a in record['arms'].values()})==1
    if not controls:
        record['controls']=None
        (root/'study.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
        return record
    # Freeze a separate positive control before any model runs.
    good=root/'known-valid'
    shutil.copytree(root/'baseline/workspace',good)
    for name,body in KNOWN_VALID.items():
        (good/name).write_text(body,encoding='utf-8',newline='\n')
    record['controls']={}
    for label,work,expected in [('broken',root/'baseline/workspace',1),('known_valid',good,0)]:
        proc=subprocess.run(['rtk','proxy',sys.executable,'-B','-m','evaluation.level3_fixture',str(work)],cwd=project,capture_output=True,text=True,timeout=120)
        (root/('control-'+label+'.txt')).write_text(proc.stdout+proc.stderr,encoding='utf-8')
        assert proc.returncode==expected,(label,proc.stderr)
        record['controls'][label]={'exit_code':proc.returncode,'expected':expected}
    existing=subprocess.run(['rtk','proxy',sys.executable,'-m','unittest','test_existing'],cwd=good,capture_output=True,text=True,timeout=60)
    assert existing.returncode==0,existing.stderr
    record['controls']['known_valid_existing_tests']={'exit_code':existing.returncode,'expected':0}
    (root/'study.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
    print(json.dumps({'prepared':order,'controls':record['controls'],'launched':False}))
    return record


if __name__=='__main__': run(sys.argv[1])
