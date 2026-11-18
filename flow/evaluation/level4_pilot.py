"""Prepare five equal-source L4 arms. Never launches agents."""
import json
import random
import shutil
import subprocess
import sys
import time
from pathlib import Path

from .level4_fixture import SOURCES, CONTRACT, DECISION, OBJECTIVE, OWNED, seed_warehouse
from .measurement_adapter import MeasurementOnlyAdapter, DECISION as MEASURE_DECISION
from .live_smoke import NativeHandoff
from integration.footnote import ENGINE, PACK, Footnote, prepare, validate_context, sha
from orchestration import Governor
from orchestration.demo import task

STUDY = 'luna-medium-L4-v2'
PROJECT = 'flow-recovery-proof'
SKILL = 'sqlite-sql-performance'
RESPONSES, MINUTES = 20, 12

# Parent-only positive control, written independently of any worker output.
KNOWN_VALID = {
    'db.py': SOURCES['db.py'].replace('''        # continue from the row after the last one of this page
        last = rows[-1][0] + 1''', '''        last = rows[-1][0]'''),
    'projection.py': SOURCES['projection.py'].replace('''    last = applied_through(conn)
    for event_id, _order, _line, sku, delta in iter_events(conn, after=last):
        conn.execute("INSERT INTO stock(sku, qty) VALUES (?, ?) "
                     "ON CONFLICT(sku) DO UPDATE SET qty = qty + excluded.qty", (sku, delta))
        last = event_id
        if event_id % 50 == 0:
            conn.commit()  # keep write transactions short on long catch-ups
    conn.execute("UPDATE meta SET value = ? WHERE key = 'applied_through'", (last,))
    conn.commit()
    return last''', '''    last = applied_through(conn)
    with conn:
        for event_id, _order, _line, sku, delta in iter_events(conn, after=last):
            conn.execute("INSERT INTO stock(sku, qty) VALUES (?, ?) "
                         "ON CONFLICT(sku) DO UPDATE SET qty = qty + excluded.qty", (sku, delta))
            last = event_id
        conn.execute("UPDATE meta SET value = ? WHERE key = 'applied_through'", (last,))
    return last'''),
    'service.py': '''import json


def receive(conn, sku, qty):
    conn.execute("INSERT INTO events(order_id, line_no, sku, delta) VALUES (NULL, NULL, ?, ?)", (sku, qty))
    conn.commit()


def check_lines(lines):
    if not isinstance(lines, (list, tuple)) or not lines:
        raise ValueError("lines must be a non-empty list")
    out = []
    for line in lines:
        if not isinstance(line, (list, tuple)) or len(line) != 2:
            raise ValueError("each line is [sku, qty]")
        sku, qty = line
        if type(sku) is not str or not sku or type(qty) is not int or qty <= 0:
            raise ValueError("line needs a sku and a positive integer qty")
        out.append([sku, qty])
    return out


def write_missing(conn, order_id, lines):
    present = {n for (n,) in conn.execute("SELECT line_no FROM events WHERE order_id = ?", (order_id,))}
    for line_no, (sku, qty) in enumerate(lines):
        if line_no not in present:
            conn.execute("INSERT INTO events(order_id, line_no, sku, delta) VALUES (?, ?, ?, ?)",
                         (order_id, line_no, sku, -qty))
    conn.execute("UPDATE orders SET status = 'done' WHERE order_id = ?", (order_id,))


def ship(conn, order_id, lines):
    """Record an outgoing order; lines is a list of [sku, qty]."""
    if type(order_id) is not str or not order_id:
        raise ValueError("order_id must be a non-empty string")
    lines = check_lines(lines)
    with conn:
        row = conn.execute("SELECT lines FROM orders WHERE order_id = ?", (order_id,)).fetchone()
        if row is None:
            conn.execute("INSERT INTO orders VALUES (?, ?, 'pending')", (order_id, json.dumps(lines)))
        elif json.loads(row[0]) != lines:
            raise ValueError(f"order {order_id} already exists with different lines")
        write_missing(conn, order_id, lines)
''',
    'recovery.py': '''import json
from projection import catch_up
from service import check_lines, write_missing


def recover(conn):
    """Finish orders interrupted by a crash, then refresh the stock projection."""
    done = []
    with conn:
        for order_id, lines in conn.execute(
                "SELECT order_id, lines FROM orders WHERE status = 'pending' ORDER BY order_id").fetchall():
            write_missing(conn, order_id, check_lines(json.loads(lines)))
            done.append(order_id)
    catch_up(conn)
    return done
''',
    'cli.py': SOURCES['cli.py'].replace('''    path, command, *rest = argv
    conn = connect(path)
    if command''', '''    path, command, *rest = argv
    try:
        out = run(connect(path), command, rest)
    except ValueError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2
    print(json.dumps(out))
    return 0


def run(conn, command, rest):
    if command''').replace('''        raise SystemExit("unknown command " + command)
    print(json.dumps(out))''', '''        raise ValueError("unknown command " + command)
    return out''').replace('    main(sys.argv[1:])', '    sys.exit(main(sys.argv[1:]))'),
}


def run(destination, study=STUDY, model='gpt-6-luna', effort='medium', controls=True, folders=None):
    root=Path(destination).resolve()
    root.mkdir(parents=True,exist_ok=False)
    project=Path(__file__).resolve().parents[1]
    pack=PACK
    order=['baseline','skills','footnote_only','footnote','flow']
    random.Random(6120).shuffle(order)
    record={'study':study,'level':'L4','model':model,'options':{'reasoningEffort':effort},
            'order':order,'maximum_runs':5,'attempts_per_arm':1,'confirmation':False,
            'objective':OBJECTIVE,'requested_model_response_limit':RESPONSES,'requested_minutes':MINUTES,
            'hard_token_limit':None,'checker_sha256':sha(project/'evaluation/level4_fixture.py'),
            'limitations':['One synthetic diagnosis-and-recovery task, one run per arm',
                'Serial randomized order; cache and run variance prevent causal ranking',
                'Common parent preparation excluded from child usage; not production setup cost',
                'Measurement-only adapter; no predictions or dollar billing claims',
                'Shared checkout with prompt isolation, not a security sandbox',
                'Limits raised from L2 (12 responses, 8 minutes) for every arm: diagnosis plus one-shot data recovery',
                'warehouse.db is mutable worker output; an unsafe recovery attempt is irreversible within the run'], 'arms':{}}
    seed=root/'seed-warehouse.db'
    seed_warehouse(seed)
    record['seed_warehouse_sha256']=sha(seed)
    for arm in order:
        began=time.perf_counter_ns()
        folder=Path(folders[arm]) if folders else root/arm
        work=folder/'workspace'
        work.mkdir(parents=True)
        for name,body in {**SOURCES,'TASK.txt':OBJECTIVE,'CONTRACT.txt':CONTRACT,'DECISION.txt':DECISION,
                          'pyproject.toml':'[project]\nname="'+PROJECT+'"\nversion="0.0.1"\n'}.items():
            (work/name).write_text(body,encoding='utf-8',newline='\n')
        shutil.copyfile(seed,work/'warehouse.db')
        item={'workspace':str(work),'initial_hashes':{p.name:sha(p) for p in work.iterdir()},
              'selected_skill':str(pack/SKILL/'SKILL.md') if arm in ('skills','footnote','flow') else None}
        # Every arm gets the same isolated setup; only assigned context delivery differs.
        (folder/'bin').mkdir()
        shutil.copy2(ENGINE,folder/'bin/footnote.exe')
        (folder/'docs').mkdir()
        (folder/'docs/contract.txt').write_text(CONTRACT+'\n'+DECISION,encoding='utf-8')
        client=Footnote(folder/'bin/footnote.exe',folder)
        # Seed memory holds the recovery decision only; it never names the planted cause.
        client.call('remember',db=client.state/'memory.db',payload=[{'text':DECISION,'kind':'fix','scope':PROJECT,'observed_at':'2026-11-18','code':['recovery.py','recover']}])
        client.call('index','--root',str(folder/'docs'),'--allow',str(folder/'docs'),db=folder/'library.db')
        client.call('code-index',str(work))
        if arm in ('footnote_only','footnote','flow'):
            context=prepare(client,project=PROJECT,workspace=work,symbol='recover',query='recovery',documents=[folder/'docs/contract.txt'])
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
    # Freeze a separate positive control before any model runs: fixed code, then recover the copy.
    good=root/'known-valid'
    shutil.copytree(root/'baseline/workspace',good)
    for name,body in KNOWN_VALID.items():
        (good/name).write_text(body,encoding='utf-8',newline='\n')
    recovered=subprocess.run(['rtk','proxy',sys.executable,'cli.py','warehouse.db','recover'],cwd=good,capture_output=True,text=True,timeout=60)
    assert recovered.returncode==0,recovered.stderr
    record['controls']={}
    for label,work,expected in [('broken',root/'baseline/workspace',1),('known_valid',good,0)]:
        proc=subprocess.run(['rtk','proxy',sys.executable,'-B','-m','evaluation.level4_fixture',str(work)],cwd=project,capture_output=True,text=True,timeout=120)
        (root/('control-'+label+'.txt')).write_text(proc.stdout+proc.stderr,encoding='utf-8')
        assert proc.returncode==expected,(label,proc.stderr)
        record['controls'][label]={'exit_code':proc.returncode,'expected':expected}
    existing=subprocess.run(['rtk','proxy',sys.executable,'-m','unittest','test_existing'],cwd=good,capture_output=True,text=True,timeout=60)
    assert existing.returncode==0,existing.stderr
    record['controls']['known_valid_existing_tests']={'exit_code':existing.returncode,'expected':0}
    # The broken checker run must not have touched the arm's production file.
    assert sha(root/'baseline/workspace/warehouse.db')==record['arms']['baseline']['initial_hashes']['warehouse.db']
    (root/'study.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
    print(json.dumps({'prepared':order,'controls':record['controls'],'launched':False}))
    return record


if __name__=='__main__': run(sys.argv[1])
