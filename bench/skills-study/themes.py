"""Dedupe the skill inventory and bucket it by theme with keyword rules."""
import collections, json, re, sys

THEMES = [  # first match wins; order matters
    ('science', r'\b(genom|protein|molecul|chem|bio|astro|clinical|scanpy|rdkit|pubmed|single-cell|lab|scientific)'),
    ('office-docs', r'\b(docx|pptx|xlsx|pdf|spreadsheet|slide|presentation|word document)'),
    ('marketing-seo', r'\b(seo|marketing|copywrit|cro|ads?|launch|pricing|email sequence|social|brand|growth|referral|onboarding)'),
    ('design-ui', r'\b(design|ui|ux|frontend|css|tailwind|animation|motion|typography|color|layout|landing|component|accessib|diagram|visual)'),
    ('writing', r'\b(writ|prose|humaniz|docs?|documentation|readme|comms|tone|blog|article)'),
    ('testing-debug', r'\b(test|tdd|debug|bug|e2e|playwright|qa|verif)'),
    ('review-quality', r'\b(review|refactor|lint|simplif|clean|audit|quality)'),
    ('planning-process', r'\b(plan|spec|brainstorm|prd|task|roadmap|requirement|interview|grill|estimate|scope)'),
    ('git-ci-devops', r'\b(git|commit|pr\b|pull request|ci|deploy|docker|kubernetes|release|infra|terraform)'),
    ('security', r'\b(secur|auth|vulnerab|secret|threat|owasp)'),
    ('agent-meta', r'\b(skill|agent|prompt|context|memory|mcp|subagent|token|harness|hook)'),
    ('backend-data', r'\b(api|database|sql|postgres|backend|server|data|python|typescript|rust|go\b|java|django|rails|node)'),
]

def canonical_rank(path):
    p = path.lower()
    if re.match(r'docs/[a-z]{2}(-[a-z]{2})?/', p): return 3  # translations
    if p.startswith('.') or p.startswith('pi/') or p.startswith('tests/'): return 2  # harness mirrors
    return 0 if p.startswith('skills/') else 1

def theme(r):
    text = (r['name'] + ' ' + r['description']).lower()
    for t, rx in THEMES:
        if re.search(rx, text): return t
    return 'other'

rows = json.load(open(sys.argv[1], encoding='utf-8'))
best = {}
for r in rows:
    k = (r['repo'], r['name'])
    if k not in best or canonical_rank(r['path']) < canonical_rank(best[k]['path']):
        best[k] = r
uniq = [dict(r, theme=theme(r)) for r in best.values()]
json.dump(uniq, open(sys.argv[2], 'w', encoding='utf-8'), indent=1)
print(len(rows), '->', len(uniq), 'unique skills')
by = collections.defaultdict(list)
for r in uniq: by[r['theme']].append(r)
for t, v in sorted(by.items(), key=lambda kv: -len(kv[1])):
    repos = collections.Counter(r['repo'].replace('cs_', '') for r in v)
    tok = sorted(r['tokens'] for r in v)
    print(f"{t:18} n={len(v):4} median_tok={tok[len(tok)//2]:5} scripts={sum(r['scripts']>0 for r in v):3} | {', '.join(f'{k}:{n}' for k, n in repos.most_common(6))}")
assert theme({'name': 'tdd', 'description': ''}) == 'testing-debug'
