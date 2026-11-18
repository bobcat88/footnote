"""Collect one completed matrix cell and rebuild the aggregate. Never launches, retries or repairs.

From the project root:
  rtk proxy python -m evaluation.collect_matrix CELL_ID [CODEX_SESSION_ID] [--t3 PATH]
  rtk proxy python -m evaluation.collect_matrix --aggregate
  rtk proxy python -m evaluation.collect_matrix --self-test
CELL_ID comes from build/matrix-v1/launch-manifest.json. T3 run metadata defaults to <cell_dir>/t3-run.json
({threadId, link, recentRuns:[...]} as returned by t3_thread_read). Without a session ID the collector
looks for the unique Codex session whose prompt names the cell workspace.
"""
import ast
import json
import os
import re
import shutil
import subprocess
import tempfile
import sys
from datetime import datetime
from pathlib import Path

from .native_receipts import read_session
from .measurement_adapter import MeasurementOnlyAdapter
from integration.footnote import sha

PROJECT = Path(__file__).resolve().parents[1]
ROOT = PROJECT / 'build/matrix-v1'
CODEX = [Path.home() / '.codex/sessions', Path.home() / '.codex/archived_sessions']
KEYS = ('input_tokens', 'cached_input_tokens', 'output_tokens')
# Recorded exploratory observations; kept with their own limitations, never re-collected.
EXISTING = {'L1': ['evidence/luna-medium-development-v1/result.json', 'evidence/luna-footnote-only-v1/result.json'],
            'L2': ['results/luna-medium-dependency-v1/result.json']}


def instant(value):
    return datetime.fromisoformat(value.replace('Z', '+00:00'))


def prompts(path):
    text = []
    for line in Path(path).open(encoding='utf-8'):
        r = json.loads(line)
        if r['type'] == 'response_item' and r['payload'].get('role') == 'user':
            text += [c.get('text', '') for c in r['payload'].get('content', [])]
    return ' '.join(text)


def find_session(session_id, workspace):
    if session_id:
        matches = [p for d in CODEX for p in d.rglob('*' + session_id + '.jsonl')]
    else:
        # shortcut: scans every session file, fine for hundreds of files; pass the ID once there are thousands.
        matches = [p for d in CODEX for p in d.rglob('*.jsonl') if workspace in prompts(p)]
    if len(matches) != 1:
        raise SystemExit(f'expected one Codex session, found {len(matches)}; pass CODEX_SESSION_ID')
    return matches[0]


# String literals in a tool call (JSON arguments or code-mode JS), shell words, apply_patch headers.
LITERAL = re.compile(r'"((?:[^"\\]|\\.)*)"|\'((?:[^\'\\]|\\.)*)\'|`([^`]*)`')
WORD = re.compile(r"""'([^']*)'|"([^"]*)"|(\S+)""")
PATCH = re.compile(r'^\*\*\* (?:(?:Add|Update|Delete) File|Move to): (.+?)\s*$', re.M)
WORKDIR = re.compile(r'["\']?(?:workdir|working_directory|cwd)["\']?\s*:\s*"((?:[^"\\]|\\.)*)"')
ABSOLUTE = re.compile(r'^(?:[A-Za-z]:[\\/]|\\\\)')


def unescape(text):
    try:
        return json.loads('"' + text + '"')
    except ValueError:
        return text


def norm(path):
    return os.path.normcase(os.path.normpath(str(path)))


def inside(path, root):
    try:
        return os.path.commonpath([path, root]) == root
    except ValueError:  # different drives
        return False


def accessed_paths(session):
    """Every path a Codex session's tool calls named, resolved against the call's working directory.
    shortcut: one workdir per tool call; a code-mode call chaining commands with different workdirs is approximated."""
    cwd, out = None, []
    for line in Path(session).open(encoding='utf-8'):
        r = json.loads(line)
        p = r.get('payload') or {}
        if r['type'] == 'session_meta':
            cwd = p.get('cwd')
        if r['type'] != 'response_item' or p.get('type') not in ('function_call', 'custom_tool_call', 'local_shell_call'):
            continue
        raw = p.get('arguments') or p.get('input') or p.get('action') or ''
        raw = raw if isinstance(raw, str) else json.dumps(raw)
        m = WORKDIR.search(raw)
        workdir = unescape(m.group(1)) if m else cwd
        literals = [unescape(a) if a else b or c for a, b, c in LITERAL.findall(WORKDIR.sub('', raw))]
        if any(t.strip() == 'rtk --version' for t in literals) and not any('\\' in t or '/' in t for t in literals):
            continue  # the mandated first command touches no file
        paths, relative = [], False
        for text in literals:
            if text.lstrip().startswith('*** Begin Patch'):
                found = PATCH.findall(text)
            else:
                found = [a or b or c for a, b, c in WORD.findall(text)]
                found = [t for t in found if '://' not in t and (t in ('.', '..') or '/' in t or '\\' in t)]
                if ABSOLUTE.match(text) and '\n' not in text:
                    found.append(text)  # one argv element of a JSON command list may hold spaces
            for token in found:
                if ABSOLUTE.match(token):
                    paths.append(token)
                elif workdir:
                    paths.append(os.path.join(workdir, token))
                    relative = True
        if workdir and (relative or not paths):
            paths.append(workdir)  # the command ran in, and may have searched, its working directory
        out += paths
    return out


def outside_accesses(session, cell, allowed):
    """Accesses outside the cell, excluding the Python install, rtk and the assigned skill. Contaminated when one
    reaches the project outside the cell (other cells, controls, known-valid code, evaluation, integration,
    evidence) or an ancestor folder from which all of those are reachable."""
    cell, project = norm(cell), norm(PROJECT)
    allowed = [norm(a) for a in allowed if a]
    outside = sorted({norm(p) for p in accessed_paths(session)})
    outside = [p for p in outside if not inside(p, cell) and not any(inside(p, a) for a in allowed)]
    return outside, any(inside(p, project) or inside(project, p) for p in outside)


def runtime_paths():
    rtk = shutil.which('rtk')
    return [sys.base_prefix, sys.prefix, os.path.dirname(sys.executable), os.path.dirname(rtk) if rtk else None]


def self_test():
    """Fake sessions: clean, harmless-outside and contaminated accesses."""
    cell = PROJECT / 'build/matrix-v1/cells/abc123'
    work = str(cell / 'workspace')
    vault = str(PROJECT.parents[2])
    allowed = runtime_paths()
    with tempfile.TemporaryDirectory() as tmp:
        def scan(*calls):
            path = Path(tmp) / f's{len(list(Path(tmp).iterdir()))}.jsonl'
            lines = [{'type': 'session_meta', 'payload': {'cwd': vault}}] + [{'type': 'response_item', 'payload': c} for c in calls]
            path.write_text('\n'.join(json.dumps(x) for x in lines), encoding='utf-8')
            return outside_accesses(path, cell, allowed)

        def js(cmd, workdir=None):
            body = 'cmd:' + json.dumps(cmd) + (',workdir:' + json.dumps(workdir) if workdir else '')
            return {'type': 'custom_tool_call', 'input': 'const r = await tools.exec_command({' + body + ',shell:"powershell"}); text(r.output);'}

        clean = scan(js('rtk --version', vault), js(f"Get-Content -Raw -LiteralPath '{work}\\TASK.txt'", vault),
                     js('rtk proxy python -m unittest test_existing', work),
                     {'type': 'custom_tool_call', 'input': 'const patch = ' + json.dumps(f'*** Begin Patch\n*** Update File: {work}\\db.py\n@@\n-a/b\n+c\n*** End Patch') + ';'},
                     {'type': 'function_call', 'arguments': json.dumps({'command': ['rtk', 'proxy', 'python', '-c', 'print(1)'], 'workdir': work})})
        assert clean == ([], False), clean
        got = scan(js("Copy-Item warehouse.db 'C:\\Temp\\copy.db'", work))
        assert got[0] == [norm('C:/Temp/copy.db')] and not got[1], got
        for bad in (js('rg iter_events', vault), js('rg iter_events .', str(cell.parent)),
                    js('Get-Content ..\\..\\..\\..\\..\\evaluation\\level4_fixture.py', work),
                    js(f"Get-ChildItem -Recurse '{PROJECT / 'build/matrix-v1/cells/other'}'", work),
                    {'type': 'function_call', 'arguments': json.dumps({'command': ['rg', 'x', str(PROJECT / 'evaluation')], 'workdir': work})}):
            got = scan(bad)
            assert got[1], (bad, got)
    print('collect_matrix self-test ok')


def foreign_imports(work, owned):
    """Third-party imports in owned Python files (every task is standard-library only)."""
    local = {p.stem for p in work.glob('*.py')}
    bad = set()
    for name in owned:
        if name.endswith('.py') and (work / name).is_file():
            try:
                tree = ast.parse((work / name).read_text(encoding='utf-8'))
            except SyntaxError:
                continue  # the independent checker reports broken code
            for node in ast.walk(tree):
                mods = [a.name for a in node.names] if isinstance(node, ast.Import) else [node.module] if isinstance(node, ast.ImportFrom) and node.module and not node.level else []
                bad |= {m.split('.')[0] for m in mods} - set(sys.stdlib_module_names) - local
    return sorted(bad)


def collect(cell_id, session_id=None, t3_path=None):
    manifest = json.loads((ROOT / 'launch-manifest.json').read_text(encoding='utf-8'))
    entry = next((c for c in manifest['cells'] if c['cell_id'] == cell_id), None)
    if entry is None:
        raise SystemExit('unknown or existing cell: ' + cell_id)
    cell, work, level = Path(entry['cell_dir']), Path(entry['workspace']), manifest['levels'][entry['level']]
    if (cell / 'result.json').exists():
        raise SystemExit('already collected; results are not replaced: ' + str(cell / 'result.json'))
    study = json.loads(Path(entry['study_file']).read_text(encoding='utf-8'))
    module = level['checker'][level['checker'].index('-m') + 1]
    assert sha(PROJECT / (module.replace('.', '/') + '.py')) == level['checker_sha256'], 'independent checker changed since preparation'
    meta = json.loads(Path(t3_path or entry['t3_run_file']).read_text(encoding='utf-8'))
    run = meta['recentRuns'][0]
    session = find_session(session_id, entry['workspace'].replace('\\', '/'))
    # T3 children lack native parent fields: bind by session ID, unique workspace in the prompt and host timestamps.
    assert entry['workspace'].replace('\\', '/') in prompts(session), 'session prompt does not name this cell workspace'
    try:
        receipt, receipt_error = read_session(session, agent_path=None, parent_id=None), None
    except ValueError as exc:
        receipt, receipt_error = None, str(exc)
    executed = run.get('executedModel') or run.get('model')
    effort = receipt['reasoning_effort'] if receipt else None
    configuration_match = executed == entry['model'] and receipt is not None and receipt['model'] == entry['model'] and effort == entry['options']['reasoningEffort']
    timestamps_match = receipt is not None and run.get('startedAt') and run.get('completedAt') and \
        abs((instant(receipt['started_at']) - instant(run['startedAt'])).total_seconds()) < 2 and \
        abs((instant(receipt['completed_at']) - instant(run['completedAt'])).total_seconds()) < 2
    command = [part.replace('{work}', str(work)) for part in level['checker']]
    checked = subprocess.run(['rtk', 'proxy', sys.executable, *command], cwd=PROJECT, text=True, capture_output=True, timeout=180)
    (cell / 'independent-check.txt').write_text(checked.stdout + checked.stderr, encoding='utf-8')
    existing_code = None
    if level['existing_tests']:
        existing = subprocess.run(['rtk', 'proxy', sys.executable, '-B', '-m', 'unittest', '-v', level['existing_tests']], cwd=work, text=True, capture_output=True, timeout=120)
        (cell / 'existing-check.txt').write_text(existing.stdout + existing.stderr, encoding='utf-8')
        existing_code = existing.returncode
    owned = level['owned']
    immutable = {name: h for name, h in level['initial_hashes'].items() if name not in owned}
    unchanged = all((work / name).is_file() and sha(work / name) == h for name, h in immutable.items())
    missing = [name for name in owned if not (work / name).is_file()]
    foreign = foreign_imports(work, owned)
    skill = entry['selected_skill']
    outside, contaminated = outside_accesses(session, cell, runtime_paths() + [os.path.dirname(skill) if skill else None])
    usage, calls = None, None
    if receipt:
        activation = study['arms'][entry['arm']]['measurement_activation']
        adapter = MeasurementOnlyAdapter(activation['evaluation_run_id'])
        adapter.activations = {a['attempt_id']: a for a in activation['activations']}
        attempt = next(iter(adapter.activations))
        for call in receipt['calls']:
            call['agent_id'] = entry['arm']
            adapter.record_call(attempt_id=attempt, agent_id=entry['arm'], call_id=call['call_id'],
                                usage={'source': 'runtime', 'accounting': 'per-call', **{k: call[k] for k in KEYS}})
        usage = {k: sum(c[k] for c in receipt['calls']) for k in KEYS}
        # read_session already rejects calls that do not sum to the cumulative snapshot.
        if receipt['cumulative_usage']:
            assert all(usage[k] == receipt['cumulative_usage'][k] for k in KEYS), 'per-call usage does not reconcile'
        usage['uncached_input_tokens'] = usage['input_tokens'] - usage['cached_input_tokens']
        usage['total_tokens'] = usage['input_tokens'] + usage['output_tokens']
        calls = len(receipt['calls'])
        receipt['identity_binding'] = {'method': 'session-id+cell-workspace-in-prompt+host-timestamps', 't3_thread_id': meta.get('threadId')}
        (cell / 'runtime-receipt.json').write_text(json.dumps(receipt, indent=2), encoding='utf-8')
        (cell / 'measurement.json').write_text(json.dumps(adapter.snapshot(), indent=2), encoding='utf-8')
    completed = run.get('status') == 'completed'
    accepted = all([completed, configuration_match, not contaminated, checked.returncode == 0, existing_code in (None, 0), unchanged, not missing, not foreign])
    result = {
        'cell_id': cell_id, 'configuration': entry['configuration'], 'level': entry['level'], 'arm': entry['arm'],
        'stack': entry['stack'], 'model': entry['model'], 'options': entry['options'], 'sequence': entry['sequence'],
        'accepted': accepted, 't3_status': run.get('status'), 'executed_model': executed, 'reasoning_effort': effort,
        'configuration_match': configuration_match, 'timestamps_match': bool(timestamps_match), 'receipt_error': receipt_error,
        'independent_exit_code': checked.returncode, 'existing_exit_code': existing_code, 'immutable_inputs_unchanged': unchanged,
        'missing_owned_outputs': missing, 'non_stdlib_imports': foreign,
        'contaminated': contaminated, 'outside_accesses': outside, 'delivery_attempts': 1, 'delivery_reruns': 0,
        'internal_repairs': None, 'model_calls': calls, 'usage': usage,
        'requested_limit_exceeded': calls > entry['requested_model_response_limit'] if calls is not None else None,
        'worker_seconds': (instant(run['completedAt']) - instant(run['requestedAt'])).total_seconds() if run.get('completedAt') else None,
        'native_seconds': receipt['elapsed_seconds'] if receipt else None,
        'host_start_delay_ms': (instant(run['startedAt']) - instant(run['requestedAt'])).total_seconds() * 1000 if run.get('startedAt') else None,
        'runtime_session_id': receipt['session_id'] if receipt else session.stem[-36:], 'thread_link': meta.get('link'),
        'artifacts': {name: sha(work / name) for name in owned if (work / name).is_file()},
        'limitations': ['One observation per cell; not an independent task sample', 'TTFT and internal repair rounds unknown',
                        'Cached tokens are a subset of input', 'No billing or subscription conversion']}
    (cell / 'result.json').write_text(json.dumps(result, indent=2), encoding='utf-8')
    aggregate()
    print(json.dumps({k: result[k] for k in ('cell_id', 'accepted', 'configuration_match', 'contaminated', 'independent_exit_code', 'existing_exit_code', 'model_calls', 'usage')}, indent=2))


def aggregate():
    cells = [json.loads(p.read_text(encoding='utf-8')) for p in sorted(ROOT.glob('cells/*/result.json'))]
    cells.sort(key=lambda c: c['sequence'])
    existing = []
    for level, paths in EXISTING.items():
        for rel in paths:
            for arm, item in json.loads((PROJECT / rel).read_text(encoding='utf-8'))['arms'].items():
                existing.append({'cell_id': f'luna-medium-{level}-{"footnote_and_skills" if arm == "footnote" else arm}', 'source': rel,
                                 'accepted': item['accepted'], 'model_calls': item['model_calls'], 'usage': item['usage'],
                                 'worker_seconds': item['worker_seconds'], 'exploratory_existing': True})
    valid = [c for c in cells if c['configuration_match']]
    total = sum(c['usage']['total_tokens'] for c in valid if c['usage'])
    accepted = sum(c['accepted'] for c in valid)
    failed = [c for c in cells if not c['accepted']]
    # Every attempt's usage counts: mismatched, contaminated and failed cells stay in the totals.
    summary = {'collected': len(cells), 'configuration_mismatch': len(cells) - len(valid), 'accepted': accepted,
               'contaminated_cells': [c['cell_id'] for c in cells if c.get('contaminated')],
               'total_tokens_valid_cells': total,
               'total_tokens_all_cells': sum(c['usage']['total_tokens'] for c in cells if c['usage']),
               'failed_attempt_tokens': sum(c['usage']['total_tokens'] for c in failed if c['usage']),
               'tokens_per_accepted_outcome': total / accepted if accepted else None,
               'configuration_mismatch_cells': [c['cell_id'] for c in cells if not c['configuration_match'] and not c['receipt_error']],
               'receipt_error_cells': [c['cell_id'] for c in cells if c['receipt_error']],
               'unknown_usage_cells': [c['cell_id'] for c in cells if not c['usage']]}
    (ROOT / 'results.json').write_text(json.dumps({'study': 'flow-complexity-ladder-v1', 'summary': summary, 'cells': cells,
                                                   'existing_cells': existing}, indent=2), encoding='utf-8')
    return summary


if __name__ == '__main__':
    args = sys.argv[1:]
    if args == ['--self-test']:
        self_test()
    elif args == ['--aggregate']:
        print(json.dumps(aggregate(), indent=2))
    elif args and not args[0].startswith('-'):
        t3 = args[args.index('--t3') + 1] if '--t3' in args else None
        rest = [a for i, a in enumerate(args[1:], 1) if a != '--t3' and args[i - 1] != '--t3']
        collect(args[0], rest[0] if rest else None, t3)
    else:
        raise SystemExit(__doc__)
