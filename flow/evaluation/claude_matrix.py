"""Complexity ladder on Claude (Haiku, Sonnet, Opus 5.5 at medium) while the Codex quota is out (#225, #222, #224).

From the flow/ folder:
  python -m evaluation.claude_matrix prepare      # cells + launch manifest; launches nothing
  python -m evaluation.claude_matrix run          # every unrun cell in sequence until done or the token cap
  python -m evaluation.claude_matrix collect ID   # collect one finished cell (run does this itself)
  python -m evaluation.claude_matrix aggregate    # rebuild results.json from collected cells
  python -m evaluation.claude_matrix --self-test

Workers are `claude -p` processes started here, not T3 children, so each setup's profile is exact (#222):
- clean (baseline, skills, footnote_only, footnote): no user settings, hooks, plugins, MCP servers, skills or CLAUDE.md;
  what a company's Claude Code does with nothing added. The prompt does not mention rtk.
- harness (flow): the same, plus RTK's hook and instructions, the Ponytail plugin, and Footnote's hook and MCP server
  on an empty per-cell memory (never Johan's own memory, which knows this benchmark).
Each cell's stream-json init record proves which setup ran; a cell whose setup does not match is not accepted.
"""
import json
import os
import shutil
import subprocess
import sys
import tempfile
import time
from pathlib import Path

from integration.footnote import sha
from .prepare_matrix import LEVELS, ARMS, STACK, opaque, posix, workspace_hashes
from .collect_matrix import foreign_imports, inside, norm, runtime_paths, WORD, ABSOLUTE

PROJECT = Path(__file__).resolve().parents[1]
ROOT = PROJECT / 'build/ladder-v2'  # neutral: the path is in every prompt
CONFIGS = [('haiku-medium', 'claude-haiku-5-5'), ('sonnet-medium', 'claude-sonnet-5-5'), ('opus-medium', 'claude-opus-5-5')]
EFFORT = 'medium'
REPS = {'baseline': 3, 'skills': 3, 'flow': 3, 'footnote_only': 1, 'footnote': 1}  # Johan, 2026-11-19
TOKEN_CAP = 125_000_000  # Johan, 2026-11-19: aggregate over every attempt; no cell starts past it
HARD_MINUTES = 30  # kills a hung worker; the requested per-level limits stay soft, as in the Codex matrix
FOOTNOTE = Path(os.environ.get('LOCALAPPDATA', '')) / 'Programs/footnote/footnote.exe'
RTK_RULES = Path.home() / '.claude/RTK.md'
BUILTIN = '@builtin'


def claude_task(level, arm, arm_dir, skill):
    """The Codex matrix prompt (prepare_matrix.worker_task) minus rtk outside the harness arm (#222)."""
    spec, work = LEVELS[level], posix(arm_dir / 'workspace')
    reads = 'TASK.txt, CONTRACT.txt and DECISION.txt' if arm in ('baseline', 'skills') else 'TASK.txt, CONTRACT.txt, DECISION.txt and footnote-context.json'
    parts = [f'Complete exactly one evaluation delivery in {work}.', f'Read {reads}.']
    if arm == 'flow':
        parts.append(f'Follow the frozen Flow brief at {posix(arm_dir / "handoff.json")}.')
    parts.append(f'Apply the curated skill {posix(skill)}.' if skill else
                 'No curated skill is assigned.' if arm == 'footnote_only' else 'No curated skill or Footnote context is assigned.')
    parts.append("Own only files listed in TASK.txt. You are not alone in the checkout; preserve others' changes.")
    if arm == 'flow':
        parts.append('First shell command: rtk --version. Prefix supported external commands with rtk. Use rtk proxy python for Python.')
    parts += ['Measurement-only adapter already activated; no predictor.',
              'No GSD, other curated skills, MCP, network, installs, commits or delegation.',
              'Run every command from the workspace (set it as the working directory). Never read, list or search outside it, except the paths named here; no searches from a parent folder.',
              f'Aim at most {spec["responses"]} model responses and {spec["minutes"]} minutes. Run local tests and deliver once, no parent repair.']
    if spec['checks_word']:
        parts.append('Include the word checks in delivery.txt.')
    parts.append('Short final answer.')
    return ' '.join(parts)


def prepare():
    ROOT.mkdir(parents=True, exist_ok=False)
    started, levels_out, reference, entries = time.time(), {}, {}, []
    for level, spec in LEVELS.items():
        record = spec['pilot'].run(ROOT / 'controls' / level, study='matrix-claude-v1-controls-' + level, controls=True, **spec['extra'])
        reference[level] = workspace_hashes(ROOT / 'controls' / level / 'baseline/workspace')
        levels_out[level] = {'order': record['order'], 'requested_model_response_limit': spec['responses'],
                             'requested_minutes': spec['minutes'], 'owned': spec['owned'], 'checker': spec['checker'],
                             'checker_sha256': sha(PROJECT / spec['checker_file']), 'existing_tests': spec['existing_tests'],
                             'controls': record['controls'], 'initial_hashes': reference[level]}
    for ci, (cid, model) in enumerate(CONFIGS):
        for level, spec in LEVELS.items():
            for rep in range(1, max(REPS.values()) + 1):
                arms = spec['extra'].get('arms', ARMS)
                name = lambda arm: f'{cid}-{level}-{STACK.get(arm, arm)}-r{rep}'
                folders = {arm: ROOT / 'cells' / opaque(name(arm)) for arm in arms}
                record = spec['pilot'].run(ROOT / cid / level / f'r{rep}', study=f'matrix-claude-v1-{cid}-{level}-r{rep}', model=model,
                                           effort=EFFORT, controls=False, folders=folders, **spec['extra'])
                for oi, arm in enumerate(record['order']):
                    if REPS[arm] < rep:
                        continue  # prepared by the pilot with its siblings, never launched
                    arm_dir = folders[arm]
                    assert workspace_hashes(arm_dir / 'workspace') == reference[level], (cid, level, arm, rep)
                    skill = record['arms'][arm]['selected_skill']
                    # Baseline and harness first, so the token cap cuts the least important cells (Johan, 2026-11-19).
                    phase = 0 if rep == 1 and arm in ('baseline', 'flow') else 1 if rep == 1 else 2
                    entries.append({
                        'cell_id': name(arm), 'configuration': cid, 'level': level, 'arm': arm, 'stack': STACK.get(arm, arm),
                        'repetition': rep, 'provider': 'claude', 'model': model, 'effort': EFFORT,
                        'profile': 'harness' if arm == 'flow' else 'clean',
                        'task': claude_task(level, arm, arm_dir, skill), 'workspace': str(arm_dir / 'workspace'),
                        'cell_dir': str(arm_dir), 'study_file': str(ROOT / cid / level / f'r{rep}' / 'study.json'),
                        'selected_skill': skill, 'skill_sha256': sha(skill) if skill else None,
                        'requested_model_response_limit': spec['responses'], 'requested_minutes': spec['minutes'],
                        '_order': (phase, rep, ci, list(LEVELS).index(level), oi)})
    entries.sort(key=lambda e: e.pop('_order'))
    for n, e in enumerate(entries, 1):
        e['sequence'] = n
        prompt = e['task'].replace(posix(e['selected_skill']) if e['selected_skill'] else '\0', '').lower()
        assert not any(w in prompt for w in ('hidden', 'other arm', 'haiku', 'sonnet', 'opus', 'claude')), e['cell_id']
        assert ('rtk' in prompt) == (e['profile'] == 'harness'), e['cell_id']
        assert posix(Path(e['cell_dir']).relative_to(ROOT)) == 'cells/' + opaque(e['cell_id'])
    assert len({e['cell_dir'] for e in entries}) == len(entries) == len(CONFIGS) * len(LEVELS) * sum(REPS.values())
    manifest = {'study': 'flow-complexity-ladder-claude-v1', 'created_at': time.strftime('%Y-%m-%dT%H:%M:%S%z'),
                'prepare_seconds': round(time.time() - started, 1), 'configurations': CONFIGS, 'effort': EFFORT,
                'repetitions': REPS, 'token_cap': TOKEN_CAP, 'levels': levels_out,
                'run_order': 'phase (repetition 1 of baseline and flow, then the rest of repetition 1, then repetitions 2-3), configuration, level, seeded arm order',
                'known_limitations': ['L1/L2 TASK.txt keep their recorded frozen text, which mentions "other arms" and "hidden checks"',
                                      'Flow handoff.json carries engine t3:codex:<model>:<effort>, written by the shared pilots; the worker is Claude Code',
                                      'Harness Footnote memory is empty per cell: it measures the harness overhead and tools, not a filled memory'],
                'cells': entries}
    (ROOT / 'launch-manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    print(json.dumps({'prepared': len(entries), 'first': entries[0]['cell_id'], 'seconds': manifest['prepare_seconds']}))


def manifest():
    return json.loads((ROOT / 'launch-manifest.json').read_text(encoding='utf-8'))


def profile_args(entry, cell):
    """Command line and environment for a cell's setup (#222)."""
    args = ['claude', '-p', entry['task'], '--model', entry['model'], '--effort', entry['effort'],
            '--setting-sources', 'project,local', '--strict-mcp-config', '--disable-slash-commands',
            '--permission-mode', 'bypassPermissions', '--output-format', 'stream-json', '--verbose']
    env = {k: v for k, v in os.environ.items() if not k.startswith(('FOOTNOTE_', 'CLAUDECODE', 'CLAUDE_CODE_'))}
    env['CLAUDE_CODE_DISABLE_CLAUDE_MDS'] = '1'  # Johan's global CLAUDE.md (RTK, design rules) is not a company default
    if entry['profile'] == 'harness':
        state = cell / 'footnote-state'
        state.mkdir()
        hook = [{'hooks': [{'type': 'command', 'command': f'"{posix(FOOTNOTE)}" hook'}]}]
        settings = {'hooks': {'PreToolUse': [{'matcher': 'Bash', 'hooks': [{'type': 'command', 'command': 'rtk hook claude'}]}],
                              'SessionStart': hook, 'UserPromptSubmit': hook, 'Stop': hook},
                    'enabledPlugins': {'ponytail@ponytail': True}}
        mcp = {'mcpServers': {'footnote-memory': {'command': posix(FOOTNOTE), 'args': ['--db', posix(state / 'memory.db'), 'mcp']}}}
        (cell / 'harness-settings.json').write_text(json.dumps(settings, indent=2), encoding='utf-8')
        (cell / 'harness-mcp.json').write_text(json.dumps(mcp, indent=2), encoding='utf-8')
        args += ['--settings', str(cell / 'harness-settings.json'), '--mcp-config', str(cell / 'harness-mcp.json'),
                 '--append-system-prompt-file', str(RTK_RULES)]
        env['FOOTNOTE_STATE_DIR'] = str(state)
    return args, env


def launch(entry):
    cell, work = Path(entry['cell_dir']), Path(entry['workspace'])
    args, env = profile_args(entry, cell)
    record = {'cell_id': entry['cell_id'], 'profile': entry['profile'], 'argv': args[:2] + ['<task>'] + args[3:],
              'env_set': sorted(k for k in env if k.startswith(('CLAUDE_CODE_', 'FOOTNOTE_'))),
              'requested_at': time.strftime('%Y-%m-%dT%H:%M:%S%z'), 'timed_out': False}
    t0 = time.time()
    with open(cell / 'stream.jsonl', 'w', encoding='utf-8') as out, open(cell / 'stderr.txt', 'w', encoding='utf-8') as err:
        try:
            p = subprocess.run(args, cwd=work, env=env, stdin=subprocess.DEVNULL, stdout=out, stderr=err, timeout=HARD_MINUTES * 60)
            record['exit_code'] = p.returncode
        except subprocess.TimeoutExpired:
            record.update(exit_code=None, timed_out=True)
    record['wall_seconds'] = round(time.time() - t0, 1)
    (cell / 'run.json').write_text(json.dumps(record, indent=2), encoding='utf-8')


def stream(cell):
    init, result, hooks = None, None, []
    for line in (cell / 'stream.jsonl').open(encoding='utf-8'):
        try:
            e = json.loads(line)
        except ValueError:
            continue
        if e.get('type') == 'system' and e.get('subtype') == 'init':
            init = e
        elif e.get('type') == 'system' and e.get('subtype') == 'hook_started':
            hooks.append(e.get('hook_event') or e.get('hook_name'))
        elif e.get('type') == 'result':
            result = e
    return init, result, hooks


def setup_check(profile, init, hooks):
    """What the init record proves was loaded, and whether it is the setup the cell asked for (#222)."""
    if init is None:
        return {'ok': False, 'reason': 'no init record'}
    seen = {'mcp_servers': sorted(f"{m['name']}:{m['status']}" for m in init.get('mcp_servers', [])),
            'plugins': sorted(p['source'] for p in init.get('plugins', []) if not p['source'].endswith(BUILTIN)),
            'skills': init.get('skills', []), 'slash_commands': init.get('slash_commands', []), 'hooks': sorted(set(hooks))}
    if profile == 'clean':
        ok = not any(seen.values())
    else:
        ok = seen['mcp_servers'] == ['footnote-memory:connected'] and seen['plugins'] == ['ponytail@ponytail'] \
            and 'SessionStart' in seen['hooks'] and not seen['skills']
    return {'ok': ok, **seen}


def transcripts(session_id):
    """The worker's Claude Code session file, and any subagent files it started."""
    base = Path.home() / '.claude/projects'
    main = list(base.glob(f'*/{session_id}.jsonl'))
    if len(main) != 1:
        return []
    return main + sorted(main[0].parent.glob(f'{session_id}/subagents/*.jsonl'))


def tool_paths(transcript, cwd):
    """Every path the worker's tool calls named. shortcut: a `cd` inside one Bash command is not followed; the
    relative paths after it resolve against the workspace, and the cd target itself is still scanned."""
    out = []
    for line in Path(transcript).open(encoding='utf-8'):
        r = json.loads(line)
        content = (r.get('message') or {}).get('content')
        for block in content if isinstance(content, list) else []:
            if block.get('type') != 'tool_use':
                continue
            inp = block.get('input') or {}
            direct = [inp[k] for k in ('file_path', 'path', 'notebook_path') if isinstance(inp.get(k), str)]
            words = [a or b or c for a, b, c in WORD.findall(inp.get('command', ''))] if isinstance(inp.get('command'), str) else []
            words = [w for w in words if '://' not in w and (w in ('.', '..') or '/' in w or '\\' in w)]
            for token in direct + words:
                out.append(token if ABSOLUTE.match(token) else os.path.join(cwd, token))
            if words or block.get('name') in ('Glob', 'Grep') and not direct:
                out.append(cwd)
    return out


def scan(files, cell, cwd, allowed):
    """Accesses outside the cell; contaminated when one reaches the benchmark project or an ancestor of it."""
    cell, project = norm(cell), norm(PROJECT)
    allowed = [norm(a) for a in allowed if a]
    seen = sorted({norm(p) for f in files for p in tool_paths(f, cwd)})
    outside = [p for p in seen if not inside(p, cell) and not any(inside(p, a) for a in allowed)]
    return outside, any(inside(p, project) or inside(project, p) for p in outside)


def price(files):
    """List API price of every request, through Footnote's session pricing (#224)."""
    keys = ('input', 'cache_read', 'cache_write_5m', 'cache_write_1h', 'output', 'requests', 'unpriced_requests')
    total, models, prices = {k: 0 for k in keys}, {}, None
    total['api_price_usd'] = 0.0
    for f in files:
        r = json.loads(subprocess.run([str(FOOTNOTE), 'usage', '--file', str(f)], capture_output=True, text=True, check=True).stdout)
        prices = r['prices']
        for k in total:
            total[k] += r['totals'][k]
        for m in r['by_model']:
            models[m['model']] = models.get(m['model'], 0) + m['requests']
    total['total_tokens'] = sum(total[k] for k in ('input', 'cache_read', 'cache_write_5m', 'cache_write_1h', 'output'))
    total['api_price_usd'] = round(total['api_price_usd'], 6)
    return total, models, prices


def collect(cell_id):
    m = manifest()
    entry = next(c for c in m['cells'] if c['cell_id'] == cell_id)
    cell, work, level = Path(entry['cell_dir']), Path(entry['workspace']), m['levels'][entry['level']]
    if (cell / 'result.json').exists():
        raise SystemExit('already collected; results are not replaced: ' + str(cell / 'result.json'))
    run = json.loads((cell / 'run.json').read_text(encoding='utf-8'))
    module = level['checker'][level['checker'].index('-m') + 1]
    assert sha(PROJECT / (module.replace('.', '/') + '.py')) == level['checker_sha256'], 'independent checker changed since preparation'
    init, result, hooks = stream(cell)
    setup = setup_check(entry['profile'], init, hooks)
    files = transcripts(init['session_id']) if init else []
    usage, models, prices = price(files) if files else (None, {}, None)
    # Background helper calls may use another model; the worker's own calls must be the requested one.
    configuration_match = bool(init) and init.get('model') == entry['model'] and models.get(entry['model'], 0) > 0
    command = [part.replace('{work}', str(work)) for part in level['checker']]
    checked = subprocess.run([sys.executable, *command], cwd=PROJECT, text=True, capture_output=True, timeout=180)
    (cell / 'independent-check.txt').write_text(checked.stdout + checked.stderr, encoding='utf-8')
    existing_code = None
    if level['existing_tests']:
        existing = subprocess.run([sys.executable, '-B', '-m', 'unittest', '-v', level['existing_tests']], cwd=work, text=True, capture_output=True, timeout=120)
        (cell / 'existing-check.txt').write_text(existing.stdout + existing.stderr, encoding='utf-8')
        existing_code = existing.returncode
    owned = level['owned']
    unchanged = all((work / n).is_file() and sha(work / n) == h for n, h in level['initial_hashes'].items() if n not in owned)
    missing = [n for n in owned if not (work / n).is_file()]
    foreign = foreign_imports(work, owned)
    skill = entry['selected_skill']
    allowed = runtime_paths() + [os.path.dirname(skill) if skill else None]
    outside, contaminated = scan(files, cell, str(work), allowed) if files else ([], False)
    completed = bool(result) and result.get('subtype') == 'success' and not result.get('is_error') and not run['timed_out']         and not run.get('interrupted')
    accepted = all([completed, configuration_match, setup['ok'], not contaminated, checked.returncode == 0,
                    existing_code in (None, 0), unchanged, not missing, not foreign])
    out = {'cell_id': cell_id, 'configuration': entry['configuration'], 'level': entry['level'], 'arm': entry['arm'],
           'stack': entry['stack'], 'repetition': entry['repetition'], 'model': entry['model'], 'effort': entry['effort'],
           'profile': entry['profile'], 'sequence': entry['sequence'], 'accepted': accepted, 'completed': completed,
           'timed_out': run['timed_out'], 'interrupted': bool(run.get('interrupted')), 'configuration_match': configuration_match, 'setup': setup,
           'independent_exit_code': checked.returncode, 'existing_exit_code': existing_code, 'immutable_inputs_unchanged': unchanged,
           'missing_owned_outputs': missing, 'non_stdlib_imports': foreign, 'contaminated': contaminated, 'outside_accesses': outside,
           'usage': usage, 'requests_by_model': models, 'prices_read': prices,
           'model_calls': usage['requests'] if usage else None,
           'requested_limit_exceeded': usage['requests'] > entry['requested_model_response_limit'] if usage else None,
           'wall_seconds': run['wall_seconds'], 'native_seconds': result['duration_ms'] / 1000 if result else None,
           'claude_code_cost_usd': result.get('total_cost_usd') if result else None,
           'session_id': init['session_id'] if init else None, 'transcripts': [str(f) for f in files],
           'artifacts': {n: sha(work / n) for n in owned if (work / n).is_file()},
           'limitations': ['Prices are list API prices (Footnote table, read on prices_read): an upper bound for negotiated rates',
                           'Wall seconds include Claude Code start-up and hooks']}
    (cell / 'result.json').write_text(json.dumps(out, indent=2), encoding='utf-8')
    return out


def pct(value, base):
    return round((value - base) / base * 100, 1) if base else None


def aggregate():
    cells = sorted((json.loads(p.read_text(encoding='utf-8')) for p in ROOT.glob('cells/*/result.json')), key=lambda c: c['sequence'])
    groups = {}
    for c in cells:
        groups.setdefault((c['configuration'], c['level'], c['arm']), []).append(c)
    rows = []
    for (cid, level, arm), runs in sorted(groups.items()):
        ok = [r for r in runs if r['accepted']]
        tokens = sum(r['usage']['total_tokens'] for r in runs if r['usage'])
        usd = sum(r['usage']['api_price_usd'] for r in runs if r['usage'])
        seconds = sum(r['wall_seconds'] or 0 for r in runs)  # an interrupted run's time is unknown
        # Every attempt's time and money counts; only accepted runs deliver (#224).
        rows.append({'configuration': cid, 'level': level, 'arm': arm, 'runs': len(runs), 'accepted': len(ok),
                     'seconds_per_accepted': round(seconds / len(ok)) if ok else None,
                     'usd_per_accepted': round(usd / len(ok), 4) if ok else None,
                     'tokens_per_accepted': round(tokens / len(ok)) if ok else None,
                     'seconds_each': [round(r['wall_seconds']) if r['wall_seconds'] is not None else None for r in runs],
                     'usd_each': [r['usage']['api_price_usd'] if r['usage'] else None for r in runs]})
    base = {(r['configuration'], r['level']): r for r in rows if r['arm'] == 'baseline'}
    for r in rows:
        b = base.get((r['configuration'], r['level']))
        if b and r['arm'] != 'baseline':
            for k in ('seconds_per_accepted', 'usd_per_accepted', 'tokens_per_accepted'):
                r[k.replace('_per_accepted', '_vs_baseline_pct')] = pct(r[k], b[k]) if r[k] is not None and b[k] else None
    total = sum(c['usage']['total_tokens'] for c in cells if c['usage'])
    summary = {'collected': len(cells), 'accepted': sum(c['accepted'] for c in cells), 'total_tokens_all_cells': total,
               'token_cap': TOKEN_CAP, 'api_price_usd_all_cells': round(sum(c['usage']['api_price_usd'] for c in cells if c['usage']), 2),
               'setup_mismatch_cells': [c['cell_id'] for c in cells if not c['setup']['ok']],
               'contaminated_cells': [c['cell_id'] for c in cells if c['contaminated']],
               'configuration_mismatch_cells': [c['cell_id'] for c in cells if not c['configuration_match']],
               'timed_out_cells': [c['cell_id'] for c in cells if c['timed_out']]}
    (ROOT / 'results.json').write_text(json.dumps({'study': 'flow-complexity-ladder-claude-v1', 'summary': summary,
                                                   'rows': rows, 'cells': cells}, indent=2), encoding='utf-8')
    return summary


def usage_limited(cell):
    """The worker was refused at its first call because the plan's usage limit was reached."""
    result = stream(cell)[1] or {}
    return result.get('api_error') == 'usage_limit_reached' and result.get('num_turns', 0) <= 1


def run_all():
    for entry in manifest()['cells']:
        cell = Path(entry['cell_dir'])
        if (cell / 'stream.jsonl').exists():  # launched before; never relaunched in a used workspace
            if not (cell / 'run.json').exists():
                (cell / 'run.json').write_text(json.dumps({'cell_id': entry['cell_id'], 'interrupted': True, 'timed_out': False,
                                                           'exit_code': None, 'wall_seconds': None}), encoding='utf-8')
            if not (cell / 'result.json').exists():
                collect(entry['cell_id'])
            continue
        spent = aggregate()['total_tokens_all_cells']
        if spent >= TOKEN_CAP:
            print(json.dumps({'stopped': 'token cap', 'spent': spent, 'next': entry['cell_id']}), flush=True)
            break
        while True:
            launch(entry)
            if not usage_limited(cell):
                break
            # The subscription's usage window ran out before the worker did anything: not a result, so wait and rerun.
            for f in ('stream.jsonl', 'stderr.txt', 'run.json'):
                (cell / f).unlink()
            print(json.dumps({'waiting': 'usage limit', 'cell_id': entry['cell_id'], 'retry_in_s': 900}), flush=True)
            time.sleep(900)
        r = collect(entry['cell_id'])
        print(json.dumps({k: r[k] for k in ('sequence', 'cell_id', 'accepted', 'wall_seconds')} |
                         {'usd': r['usage']['api_price_usd'] if r['usage'] else None, 'setup_ok': r['setup']['ok']}), flush=True)
    print(json.dumps(aggregate(), indent=2))


def self_test():
    clean = {'mcp_servers': [], 'plugins': [{'source': 'cc-plugin-privacy@builtin'}], 'skills': [], 'slash_commands': []}
    assert setup_check('clean', clean, [])['ok']
    assert not setup_check('clean', clean, ['SessionStart'])['ok']
    assert not setup_check('clean', {**clean, 'mcp_servers': [{'name': 'x', 'status': 'connected'}]}, [])['ok']
    harness = {**clean, 'mcp_servers': [{'name': 'footnote-memory', 'status': 'connected'}],
               'plugins': clean['plugins'] + [{'source': 'ponytail@ponytail'}]}
    assert setup_check('harness', harness, ['SessionStart'])['ok']
    assert not setup_check('harness', {**harness, 'mcp_servers': []}, ['SessionStart'])['ok']
    cell = ROOT / 'cells/abc123'
    work = str(cell / 'workspace')
    with tempfile.TemporaryDirectory() as tmp:
        (Path(tmp) / 'stream.jsonl').write_text(json.dumps({'type': 'result', 'api_error': 'usage_limit_reached', 'num_turns': 1}), encoding='utf-8')
        assert usage_limited(Path(tmp))
        (Path(tmp) / 'stream.jsonl').write_text(json.dumps({'type': 'result', 'num_turns': 9}), encoding='utf-8')
        assert not usage_limited(Path(tmp))
    with tempfile.TemporaryDirectory() as tmp:
        def scan_calls(*calls):
            f = Path(tmp) / f's{len(list(Path(tmp).iterdir()))}.jsonl'
            f.write_text('\n'.join(json.dumps({'message': {'content': [{'type': 'tool_use', 'name': n, 'input': i}]}}) for n, i in calls), encoding='utf-8')
            return scan([f], cell, work, runtime_paths())
        assert scan_calls(('Read', {'file_path': work + '/TASK.txt'}), ('Bash', {'command': 'python -m unittest test_existing'}),
                          ('Edit', {'file_path': work + '\\db.py'}), ('Grep', {'pattern': 'x'})) == ([], False)
        got = scan_calls(('Bash', {'command': "cp warehouse.db 'C:/Temp/copy.db'"}))
        assert got[0] == [norm('C:/Temp/copy.db')] and not got[1], got
        for bad in [('Grep', {'pattern': 'x', 'path': str(PROJECT / 'evaluation')}), ('Bash', {'command': 'cat ../../../../evaluation/level4_fixture.py'}),
                    ('Glob', {'pattern': '**/*.py', 'path': str(PROJECT.parent)}), ('Read', {'file_path': str(cell.parent / 'other/workspace/db.py')})]:
            assert scan_calls(bad)[1], bad
    print('claude_matrix self-test ok')


if __name__ == '__main__':
    a = sys.argv[1:]
    if a == ['prepare']:
        prepare()
    elif a == ['run']:
        run_all()
    elif a[:1] == ['collect'] and len(a) == 2:
        print(json.dumps(collect(a[1]), indent=2))
        aggregate()
    elif a == ['aggregate']:
        print(json.dumps(aggregate(), indent=2))
    elif a == ['--self-test']:
        self_test()
    else:
        raise SystemExit(__doc__)
