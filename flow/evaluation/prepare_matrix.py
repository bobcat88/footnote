"""Prepare every unrun complexity-ladder cell and its launch manifest. Never launches agents.

Run once from the project root: rtk proxy python -m evaluation.prepare_matrix
"""
import hashlib
import json
import sys
import time
from pathlib import Path

from integration.footnote import sha
from . import luna_pilot, dependency_pilot, level3_pilot, level4_pilot
from .dependency_fixture import OWNED as L2_OWNED
from .level3_fixture import OWNED as L3_OWNED
from .level4_fixture import OWNED as L4_OWNED

PROJECT = Path(__file__).resolve().parents[1]
LADDER = PROJECT / 'evaluation/complexity-ladder-v1.json'
ROOT = PROJECT / 'build/matrix-v1'
ARMS = ['baseline', 'skills', 'footnote_only', 'footnote', 'flow']
STACK = {'footnote': 'footnote_and_skills'}  # pilot arm name -> ladder stack name
EXISTING = {'luna-medium': ('L1', 'L2')}
# Recorded pilots whose fixture, knowledge, skill card and Footnote context L1/L2 cells must reproduce.
RECORDED = {'L1': PROJECT / 'build/luna-medium-development-v1', 'L2': PROJECT / 'build/luna-medium-dependency-v1'}
LEVELS = {
    'L1': {'pilot': luna_pilot, 'extra': {'arms': ARMS}, 'responses': 8, 'minutes': 5,
           'owned': ['intervals.py', 'test_intervals.py', 'delivery.txt'],
           'checker': ['-m', 'integration.check_intervals', '{work}', '--followup'],
           'checker_file': 'integration/check_intervals.py', 'existing_tests': None, 'checks_word': False},
    'L2': {'pilot': dependency_pilot, 'extra': {}, 'responses': 12, 'minutes': 8, 'owned': L2_OWNED,
           'checker': ['-m', 'evaluation.dependency_fixture', '{work}'],
           'checker_file': 'evaluation/dependency_fixture.py', 'existing_tests': 'test_existing', 'checks_word': True},
    'L3': {'pilot': level3_pilot, 'extra': {}, 'responses': 16, 'minutes': 10, 'owned': L3_OWNED,
           'checker': ['-B', '-m', 'evaluation.level3_fixture', '{work}'],
           'checker_file': 'evaluation/level3_fixture.py', 'existing_tests': 'test_existing', 'checks_word': True},
    'L4': {'pilot': level4_pilot, 'extra': {}, 'responses': 20, 'minutes': 12, 'owned': L4_OWNED,
           'checker': ['-B', '-m', 'evaluation.level4_fixture', '{work}'],
           'checker_file': 'evaluation/level4_fixture.py', 'existing_tests': 'test_existing', 'checks_word': True},
}


def opaque(cell_id):
    """Neutral cell folder name: the workspace path reveals neither configuration, level nor setup."""
    return hashlib.sha256(cell_id.encode()).hexdigest()[:10]


def posix(path):
    return str(path).replace('\\', '/')


def workspace_hashes(work):
    # Footnote context is a treatment, not initial project state; it is compared separately.
    return {p.name: sha(p) for p in sorted(Path(work).iterdir()) if p.is_file() and p.name != 'footnote-context.json'}


def normalized_context(arm_dir):
    data = json.loads((Path(arm_dir) / 'workspace/footnote-context.json').read_text(encoding='utf-8'))
    data.pop('context_bytes')  # size of the serialised context; varies only with the workspace path length
    return json.dumps(data, sort_keys=True).replace(json.dumps(str(Path(arm_dir)))[1:-1], '<ARM>')


def worker_task(level, arm, arm_dir, skill):
    """Template from the recorded L2 Codex prompts, normalised; no model, other-arm or hidden-check wording."""
    spec, work = LEVELS[level], posix(arm_dir / 'workspace')
    reads = 'TASK.txt, CONTRACT.txt and DECISION.txt' if arm in ('baseline', 'skills') else 'TASK.txt, CONTRACT.txt, DECISION.txt and footnote-context.json'
    parts = [f'Complete exactly one evaluation delivery in {work}.', f'Read {reads}.']
    if arm == 'flow':
        parts.append(f'Follow the frozen Flow brief at {posix(arm_dir / "handoff.json")}.')
    parts.append(f'Apply the curated skill {posix(skill)}.' if skill else
                 'No curated skill is assigned.' if arm == 'footnote_only' else 'No curated skill or Footnote context is assigned.')
    parts += ["Own only files listed in TASK.txt. You are not alone in the checkout; preserve others' changes.",
              'First shell command: rtk --version. Prefix supported external commands with rtk. Use rtk proxy python for Python.',
              'Measurement-only adapter already activated; no predictor.',
              'No GSD, other curated skills, MCP, network, installs, commits or delegation.',
              'Run every command from the workspace (set it as the working directory). Never read, list or search outside it, except the paths named here; no searches from a parent folder.',
              f'Aim at most {spec["responses"]} model responses and {spec["minutes"]} minutes. Run local tests and deliver once, no parent repair.']
    if spec['checks_word']:
        parts.append('Include the word checks in delivery.txt.')
    parts.append('Short final answer.')
    return ' '.join(parts)


def run():
    ladder = json.loads(LADDER.read_text(encoding='utf-8'))
    ROOT.mkdir(parents=True, exist_ok=False)
    started = time.time()
    controls, orders, reference, entries, levels_out = {}, {}, {}, [], {}
    # Controls once per level, in a throwaway prepared root so no cell workspace is touched by a checker.
    for level, spec in LEVELS.items():
        record = spec['pilot'].run(ROOT / 'controls' / level, study='matrix-v1-controls-' + level, controls=True, **spec['extra'])
        controls[level] = record['controls']
        orders[level] = record['order']
        reference[level] = workspace_hashes(ROOT / 'controls' / level / 'baseline/workspace')
        levels_out[level] = {'order': record['order'], 'requested_model_response_limit': spec['responses'],
                             'requested_minutes': spec['minutes'], 'owned': spec['owned'], 'checker': spec['checker'],
                             'checker_sha256': sha(PROJECT / spec['checker_file']), 'existing_tests': spec['existing_tests'],
                             'controls': record['controls'], 'initial_hashes': reference[level]}
    for config in ladder['configurations']:
        cid = config['id']
        for level, spec in LEVELS.items():
            if level in EXISTING.get(cid, ()):
                continue
            root = ROOT / cid / level  # study.json only; the prompt never points here
            arms = spec['extra'].get('arms', ARMS)
            folders = {arm: ROOT / 'cells' / opaque(f'{cid}-{level}-{STACK.get(arm, arm)}') for arm in arms}
            record = spec['pilot'].run(root, study=f'matrix-v1-{cid}-{level}', model=config['model'],
                                       effort=config['reasoningEffort'], controls=False, folders=folders, **spec['extra'])
            assert record['order'] == orders[level], (cid, level)
            for arm in record['order']:
                arm_dir = folders[arm]
                hashes = workspace_hashes(arm_dir / 'workspace')
                # Same initial state for every arm of every configuration at this level.
                assert hashes == reference[level], (cid, level, arm)
                skill = record['arms'][arm]['selected_skill']
                entries.append({
                    'cell_id': f'{cid}-{level}-{STACK.get(arm, arm)}', 'configuration': cid, 'level': level,
                    'arm': arm, 'stack': STACK.get(arm, arm), 'provider': 'codex', 'model': config['model'],
                    'options': {'reasoningEffort': config['reasoningEffort']}, 'role': 'implementation',
                    'task': worker_task(level, arm, arm_dir, skill), 'workspace': str(arm_dir / 'workspace'),
                    'cell_dir': str(arm_dir), 'study_file': str(root / 'study.json'), 'selected_skill': skill, 'skill_sha256': sha(skill) if skill else None,
                    'context_sha256': sha(arm_dir / 'workspace/footnote-context.json') if (arm_dir / 'workspace/footnote-context.json').is_file() else None,
                    'requested_model_response_limit': spec['responses'], 'requested_minutes': spec['minutes'],
                    't3_run_file': str(arm_dir / 't3-run.json'),
                    'collector_command': f'rtk proxy python -m evaluation.collect_matrix {cid}-{level}-{STACK.get(arm, arm)} [CODEX_SESSION_ID]'})
    # L1/L2 must reproduce the recorded pilots exactly (fixture, knowledge, skill card, Footnote context).
    reproduction = {}
    for level, recorded in RECORDED.items():
        study = json.loads((recorded / 'study.json').read_text(encoding='utf-8'))
        ours = reference[level]
        for arm, item in study['arms'].items():
            if 'initial_hashes' in item:
                assert item['initial_hashes'] == ours, (level, arm)
            else:
                assert item['initial_source_sha256'] == ours['intervals.py'] and all(ours[k] == v for k, v in item['initial_knowledge'].items()), (level, arm)
        skills = {Path(i['selected_skill']).resolve() for i in study['arms'].values() if i['selected_skill']}
        assert len(skills) == 1
        skill_sha = sha(next(iter(skills)))
        recorded_context = normalized_context(recorded / 'footnote')
        cells = [e for e in entries if e['level'] == level]
        assert all(e['skill_sha256'] in (None, skill_sha) for e in cells)
        assert all(normalized_context(e['cell_dir']) == recorded_context for e in cells if e['context_sha256']), level
        reproduction[level] = {'recorded_pilot': str(recorded), 'initial_hashes_match': True, 'skill_sha256': skill_sha,
                               'footnote_context_match_after_path_normalisation': True}
    # Neutrality: no prompt or workspace path names a configuration, model, effort, level or setup.
    names = {c['id'] for c in ladder['configurations']} | {c['model'] for c in ladder['configurations']} | set(LEVELS) | set(ARMS) | set(STACK.values())
    for e in entries:
        rel = posix(Path(e['cell_dir']).relative_to(ROOT))
        assert rel == 'cells/' + opaque(e['cell_id']), rel
        prompt = e['task'].replace(posix(e['selected_skill']) if e['selected_skill'] else '\0', '')
        lowered = prompt.lower()
        assert not any(w in lowered for w in ('hidden', 'other arm', 'other setup', 'known-valid', 'luna', 'terra', 'sol-', 'gpt-')), e['cell_id']
        assert not any(posix(n) in posix(Path(e['cell_dir']).relative_to(ROOT)) for n in names), e['cell_id']
        if e['level'] in ('L3', 'L4'):
            for name in ('TASK.txt', 'CONTRACT.txt', 'DECISION.txt'):
                text = (Path(e['workspace']) / name).read_text(encoding='utf-8').lower()
                assert 'hidden' not in text and 'other arm' not in text, (e['cell_id'], name)
    assert len({e['cell_dir'] for e in entries}) == len(entries)
    for e, n in zip(entries, range(1, len(entries) + 1)):
        e['sequence'] = n
        e['title'] = f'matrix-v1 #{n:03d} {e["level"]} {e["arm"]}'
    existing = [c['id'] for c in ladder['cells'] if c['configuration'] in EXISTING and c['level'] in EXISTING[c['configuration']]]
    prepared = {e['cell_id'] for e in entries}
    assert len(existing) == 10 and len(prepared) == len(entries) == 110 and not prepared & set(existing)
    assert prepared | set(existing) == {c['id'] for c in ladder['cells']}
    manifest = {
        'study': 'flow-complexity-ladder-v1', 'manifest': 'matrix-v1', 'created_at': time.strftime('%Y-%m-%dT%H:%M:%S%z'),
        'launched': False, 'prepare_seconds': round(time.time() - started, 1),
        'counts': {'cells': 120, 'existing': len(existing), 'prepared': len(entries)},
        'run_order': 'configuration order, then L1..L4, then the level seeded arm order',
        'levels': levels_out, 'existing_cells': existing, 'reproduction_of_recorded_pilots': reproduction,
        'prompt_template_source': ['Codex session user prompts identified by build/luna-medium-dependency-v1/session-identities.json (archived under ~/.codex/archived_sessions)',
                                   'Normalised to one template: dropped model, other-arm and hidden-check wording; stated the skill path up front (the recorded skills run needed a path correction)',
                                   'Workspaces live in opaque folders cells/<sha256(cell_id)[:10]>; the cell_id mapping is kept only in this manifest and the study files'],
        'known_limitations': ['L1/L2 TASK.txt keep their recorded frozen text, which mentions "other arms" and "hidden checks"; L3/L4 use neutral wording',
                              'Flow handoff.json carries engine t3:codex:<model>:<effort> (Flow arm only)',
                              'Footnote context content is unchanged (REVIEW item 3): it repeats CONTRACT/DECISION that every arm already reads'],
        'delegate_task': {'role': 'implementation', 'target': {'providerInstanceId': 'codex', 'model': '<entry.model>', 'options': {'reasoningEffort': '<entry.options.reasoningEffort>'}}, 'mode': 'async', 'isolation': 'shared'},
        'cells': entries}
    (ROOT / 'launch-manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    for cell in ladder['cells']:
        if cell['id'] in prepared:
            cell['status'] = 'prepared_not_run'
    ladder['counts'] = {'cells': 120, 'existing_exploratory': len(existing), 'prepared_not_run': len(entries), 'not_run': 0}
    ladder['status'] = 'cells_prepared_aggregate_budget_pending'
    ladder['limits']['per_level_elapsed_and_response_caps'] = {lvl: {'requested_model_responses': s['responses'], 'requested_minutes': s['minutes'], 'enforced': False} for lvl, s in LEVELS.items()}
    ladder['launch_manifest'] = 'build/matrix-v1/launch-manifest.json'
    LADDER.write_text(json.dumps(ladder, indent=2) + '\n', encoding='utf-8')
    print(json.dumps({'prepared': len(entries), 'existing': len(existing), 'orders': orders, 'controls': controls,
                      'reproduction': reproduction, 'launched': False}, indent=2))


if __name__ == '__main__':
    if len(sys.argv) != 1:
        raise SystemExit('usage: python -m evaluation.prepare_matrix')
    run()
