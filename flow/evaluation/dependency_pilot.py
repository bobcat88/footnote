"""Prepare five equal-source arms. Never launches agents."""
import json
import random
import shutil
import subprocess
import sys
import time
from pathlib import Path

from .dependency_fixture import SOURCES, CONTRACT, DECISION, OBJECTIVE, OWNED
from .measurement_adapter import MeasurementOnlyAdapter, DECISION as MEASURE_DECISION
from .live_smoke import NativeHandoff
from integration.footnote import ENGINE, PACK, Footnote, prepare, validate_context, sha
from orchestration import Governor
from orchestration.demo import task


def run(destination, study='luna-medium-dependency-v1', model='gpt-6-luna', effort='medium', controls=True, folders=None):
    root=Path(destination).resolve()
    root.mkdir(parents=True,exist_ok=False)
    project=Path(__file__).resolve().parents[1]
    pack=PACK
    order=['baseline','skills','footnote_only','footnote','flow']
    random.Random(6118).shuffle(order)
    record={'study':study,'model':model,'options':{'reasoningEffort':effort},
            'order':order,'maximum_runs':5,'attempts_per_arm':1,'confirmation':False,
            'objective':OBJECTIVE,'requested_model_response_limit':12,'requested_minutes':8,
            'hard_token_limit':None,'checker_sha256':sha(project/'evaluation/dependency_fixture.py'),
            'limitations':['One synthetic multi-module task, one run per arm',
                'Serial randomized order; cache and run variance prevent causal ranking',
                'Common parent preparation excluded from child usage; not production setup cost',
                'Measurement-only adapter; no predictions or dollar billing claims',
                'Shared checkout with prompt isolation, not a security sandbox'], 'arms':{}}
    for arm in order:
        began=time.perf_counter_ns()
        folder=Path(folders[arm]) if folders else root/arm
        work=folder/'workspace'
        work.mkdir(parents=True)
        for name,body in {**SOURCES,'TASK.txt':OBJECTIVE,'CONTRACT.txt':CONTRACT,'DECISION.txt':DECISION,
                          'pyproject.toml':'[project]\nname="flow-dependency-proof"\nversion="0.0.1"\n'}.items():
            (work/name).write_text(body,encoding='utf-8',newline='\n')
        item={'workspace':str(work),'initial_hashes':{p.name:sha(p) for p in work.iterdir()},
              'selected_skill':str(pack/'python-performance/SKILL.md') if arm in ('skills','footnote','flow') else None}
        # Every arm gets the same isolated setup; only assigned context delivery differs.
        (folder/'bin').mkdir()
        shutil.copy2(ENGINE,folder/'bin/footnote.exe')
        (folder/'docs').mkdir()
        (folder/'docs/contract.txt').write_text(CONTRACT+'\n'+DECISION,encoding='utf-8')
        client=Footnote(folder/'bin/footnote.exe',folder)
        client.call('remember',db=client.state/'memory.db',payload=[{'text':DECISION,'kind':'fix','scope':'flow-dependency-proof','observed_at':'2026-11-17','code':['intervals.py','merge_intervals']}])
        client.call('index','--root',str(folder/'docs'),'--allow',str(folder/'docs'),db=folder/'library.db')
        client.call('code-index',str(work))
        if arm in ('footnote_only','footnote','flow'):
            context=prepare(client,project='flow-dependency-proof',workspace=work,symbol='merge_intervals',query='merge_intervals',documents=[folder/'docs/contract.txt'])
            validate_context(context)
            (work/'footnote-context.json').write_text(json.dumps(context,indent=2),encoding='utf-8')
            item['context_bytes']=context['context_bytes']
        item['footnote_calls']=client.calls
        adapter=MeasurementOnlyAdapter(study.removesuffix('-v1')+'-'+arm)
        if arm=='flow':
            governor=Governor(folder/'project.sqlite',work,pack)
            try:
                contract=task('implement','implementer',OWNED,[{'kind':'contains','path':'delivery.txt','value':'checks'}],skills=['python-performance'],context=['TASK.txt','CONTRACT.txt','DECISION.txt','footnote-context.json'])
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
    (good/'intervals.py').write_text('''def merge_intervals(intervals, *, join_touching=False):
    if type(join_touching) is not bool:
        raise ValueError('policy')
    try:
        rows = list(intervals)
        for row in rows:
            if not isinstance(row, (list, tuple)) or len(row) != 2 or any(type(v) is not int for v in row) or row[0] >= row[1]:
                raise ValueError('interval')
    except (TypeError, ValueError):
        raise ValueError('intervals') from None
    result=[]
    for a,b in sorted(rows):
        if result and (a < result[-1][1] or join_touching and a == result[-1][1]):
            result[-1]=(result[-1][0],max(b,result[-1][1]))
        else: result.append((a,b))
    return result
''',encoding='utf-8')
    availability=SOURCES['availability.py'].replace('    merged = merge_intervals(bookings)','    if type(start) is not int or type(end) is not int or start >= end:\n        raise ValueError("bounds")\n    merged = merge_intervals(bookings, join_touching=True)').replace('def busy_windows(bookings):\n    return merge_intervals(bookings)','def busy_windows(bookings, *, join_touching=False):\n    return merge_intervals(bookings, join_touching=join_touching)')
    (good/'availability.py').write_text(availability,encoding='utf-8')
    (good/'api.py').write_text(SOURCES['api.py'].replace('bookings = payload["bookings"]','bookings = list(payload["bookings"])').replace('busy_windows(bookings)','busy_windows(bookings, join_touching=payload.get("join_touching", False))'),encoding='utf-8')
    record['controls']={}
    for label,work,expected in [('broken',root/'baseline/workspace',1),('known_valid',good,0)]:
        proc=subprocess.run(['rtk','proxy',sys.executable,'-m','evaluation.dependency_fixture',str(work)],cwd=project,capture_output=True,text=True,timeout=30)
        (root/('control-'+label+'.txt')).write_text(proc.stdout+proc.stderr,encoding='utf-8')
        assert proc.returncode==expected,(label,proc.stderr)
        record['controls'][label]={'exit_code':proc.returncode,'expected':expected}
    (root/'study.json').write_text(json.dumps(record,indent=2),encoding='utf-8')
    print(json.dumps({'prepared':order,'controls':record['controls'],'launched':False}))
    return record


if __name__=='__main__': run(sys.argv[1])
