"""Import explicitly identified completed runs; no launch, retry or report submission."""
import json
import subprocess
import sys
from datetime import datetime
from pathlib import Path

from .dependency_fixture import OWNED
from .native_receipts import read_session
from .measurement_adapter import MeasurementOnlyAdapter
from integration.footnote import sha


def instant(value):
    return datetime.fromisoformat(value.replace('Z','+00:00'))


def run(destination):
    root=Path(destination).resolve()
    project=Path(__file__).resolve().parents[1]
    study=json.loads((root/'study.json').read_text())
    t3=json.loads((root/'t3-runs.json').read_text())
    identities=json.loads((root/'session-identities.json').read_text())
    assert sha(project/'evaluation/dependency_fixture.py')==study['checker_sha256']
    result={k:study[k] for k in ('study','model','order','limitations','confirmation')}
    result['arms']={}
    for arm in study['order']:
        meta=t3[arm]
        run=meta['recentRuns'][0]
        assert run['status']=='completed' and run['executedModel']=='gpt-6-luna'
        ident=identities[arm]
        matches=list((Path.home()/'.codex/sessions').rglob('*'+ident+'.jsonl'))
        assert len(matches)==1
        session=matches[0]
        records=[json.loads(line) for line in session.open(encoding='utf-8')]
        users=[r['payload'] for r in records if r['type']=='response_item' and r['payload'].get('role')=='user']
        prompts=' '.join(c.get('text','') for u in users for c in u.get('content',[]))
        assert 'luna-medium-dependency-v1/'+arm+'/workspace' in prompts
        receipt=read_session(session,agent_path=None,parent_id=None)
        assert receipt['model']=='gpt-6-luna' and receipt['reasoning_effort']=='medium'
        assert abs((instant(receipt['started_at'])-instant(run['startedAt'])).total_seconds())<2
        assert abs((instant(receipt['completed_at'])-instant(run['completedAt'])).total_seconds())<2
        work=Path(study['arms'][arm]['workspace'])
        immutable=['TASK.txt','CONTRACT.txt','DECISION.txt','test_existing.py','pyproject.toml']
        unchanged=all(sha(work/name)==study['arms'][arm]['initial_hashes'][name] for name in immutable)
        checked=subprocess.run(['rtk','proxy',sys.executable,'-m','evaluation.dependency_fixture',str(work)],cwd=project,text=True,capture_output=True,timeout=30)
        (root/arm/'independent-check.txt').write_text(checked.stdout+checked.stderr,encoding='utf-8')
        existing=subprocess.run(['rtk','proxy',sys.executable,'-m','unittest','-v','test_existing'],cwd=work,text=True,capture_output=True,timeout=30)
        (root/arm/'existing-check.txt').write_text(existing.stdout+existing.stderr,encoding='utf-8')
        accepted=checked.returncode==0 and existing.returncode==0 and unchanged and all((work/name).is_file() for name in OWNED)
        activation=study['arms'][arm]['measurement_activation']
        adapter=MeasurementOnlyAdapter(activation['evaluation_run_id'])
        adapter.activations={a['attempt_id']:a for a in activation['activations']}
        attempt=next(iter(adapter.activations))
        for call in receipt['calls']:
            call['agent_id']=arm
            adapter.record_call(attempt_id=attempt,agent_id=arm,call_id=call['call_id'],usage={'source':'runtime','accounting':'per-call',**{k:call[k] for k in ('input_tokens','cached_input_tokens','output_tokens')}})
        usage={k:sum(c[k] for c in receipt['calls']) for k in ('input_tokens','cached_input_tokens','output_tokens')}
        usage['uncached_input_tokens']=usage['input_tokens']-usage['cached_input_tokens']
        usage['total_tokens']=usage['input_tokens']+usage['output_tokens']
        receipt['identity_binding']={'method':'explicit-session-id+workspace+host-timestamps','session_id':ident,'t3_thread_id':meta['threadId']}
        (root/arm/'runtime-receipt.json').write_text(json.dumps(receipt,indent=2),encoding='utf-8')
        (root/arm/'measurement.json').write_text(json.dumps(adapter.snapshot(),indent=2),encoding='utf-8')
        result['arms'][arm]={'accepted':accepted,'independent_exit_code':checked.returncode,'existing_exit_code':existing.returncode,'immutable_inputs_unchanged':unchanged,'delivery_attempts':1,'delivery_reruns':0,'internal_repairs':None,'model_calls':len(receipt['calls']),'usage':usage,'worker_seconds':(instant(run['completedAt'])-instant(run['requestedAt'])).total_seconds(),'native_seconds':receipt['elapsed_seconds'],'host_start_delay_ms':(instant(run['startedAt'])-instant(run['requestedAt'])).total_seconds()*1000,'runtime_usage_reconciled':True,'artifacts':{name:sha(work/name) for name in OWNED if (work/name).is_file()},'thread_link':meta['link'],'requested_limit_exceeded':len(receipt['calls'])>12}
    result['aggregate_usage']={k:sum(a['usage'][k] for a in result['arms'].values()) for k in usage}
    result['limitations']+=['Worker time excludes parent preparation and external acceptance scheduling','TTFT and internal repair rounds unknown','Cached tokens are a subset of input, not an additional token category','No production Usage attribution; no billing or subscription conversion']
    (root/'result.json').write_text(json.dumps(result,indent=2),encoding='utf-8')
    print(json.dumps(result,indent=2))


if __name__=='__main__': run(sys.argv[1])
