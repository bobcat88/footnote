"""Structural comparison of same-job skills across repos. Reads files only."""
import json, os, re, sys

ROOT = sys.argv[1]
CLUSTERS = {
    'tdd': ['cs_obra_superpowers/skills/test-driven-development', 'cs_addyosmani_agent-skills/skills/test-driven-development', 'cs_mattpocock_skills/*/tdd', 'cs_affaan-m_ECC/skills/tdd-workflow'],
    'debugging': ['cs_obra_superpowers/skills/systematic-debugging', 'cs_addyosmani_agent-skills/skills/debugging-and-error-recovery', 'cs_mattpocock_skills/*/diagnosing-bugs'],
    'planning': ['cs_obra_superpowers/skills/writing-plans', 'cs_addyosmani_agent-skills/skills/planning-and-task-breakdown', 'cs_mattpocock_skills/*/to-spec', 'cs_affaan-m_ECC/skills/blueprint'],
    'brainstorm-interview': ['cs_obra_superpowers/skills/brainstorming', 'cs_addyosmani_agent-skills/skills/interview-me', 'cs_mattpocock_skills/*/grilling'],
    'code-review': ['cs_obra_superpowers/skills/requesting-code-review', 'cs_addyosmani_agent-skills/skills/code-review-and-quality', 'cs_mattpocock_skills/*/code-review', 'cs_DietrichGebert_ponytail/skills/ponytail-review'],
    'verify-before-done': ['cs_obra_superpowers/skills/verification-before-completion', 'cs_affaan-m_ECC/skills/verification-loop'],
    'frontend-design': ['cs_anthropics_skills/skills/frontend-design', 'cs_pbakaus_impeccable/skill*/impeccable', 'cs_Leonxlnx_taste-skill/*', 'cs_emilkowalski_skills/*/emil-design-eng', 'cs_nextlevelbuilder_ui-ux-pro-max-skill/*/ui-ux-pro-max'],
    'animation': ['cs_emilkowalski_skills/*/animate', 'cs_affaan-m_ECC/skills/motion-foundations'],
    'writing-human': ['cs_blader_humanizer', 'cs_ayghri_i-have-adhd/*', 'cs_mattpocock_skills/*/writing-for-agents'],
    'skill-authoring': ['cs_anthropics_skills/skills/skill-creator', 'cs_obra_superpowers/skills/writing-skills'],
    'git-finish': ['cs_obra_superpowers/skills/finishing-a-development-branch', 'cs_addyosmani_agent-skills/skills/git-workflow-and-versioning', 'cs_mattpocock_skills/*/pr', 'cs_affaan-m_ECC/skills/git-workflow'],
    'handoff-context': ['cs_mattpocock_skills/*/handoff', 'cs_affaan-m_ECC/skills/strategic-compact', 'cs_addyosmani_agent-skills/skills/context-engineering'],
    'security': ['cs_addyosmani_agent-skills/skills/security-and-hardening', 'cs_affaan-m_ECC/skills/security-review'],
    'seo': ['marketingskills/skills/seo-audit', 'cs_affaan-m_ECC/skills/seo'],
}

def find(pattern):
    import glob
    hits = [h for h in glob.glob(os.path.join(ROOT, pattern, 'SKILL.md'))] + glob.glob(os.path.join(ROOT, pattern, '**', 'SKILL.md'), recursive=True)
    hits = [h for h in hits if not re.search(r'[\\/](docs|\.[a-z]+|pi|tests)[\\/]', h[len(ROOT):])]
    return sorted(set(hits), key=len)[0] if hits else None

def metrics(p):
    t = open(p, encoding='utf-8', errors='replace').read()
    m = re.match(r'^---\s*\n(.*?)\n---', t, re.S)
    desc = re.search(r'^description:\s*(.*)$', m.group(1), re.M).group(1) if m and 'description:' in m.group(1) else ''
    d = os.path.dirname(p)
    side = [f for _, _, fs in os.walk(d) for f in fs if f != 'SKILL.md']
    return {
        'tok': len(t.encode()) // 4, 'desc': len(desc), 'h2': len(re.findall(r'^## ', t, re.M)),
        'code': t.count('```') // 2, 'caps': len(re.findall(r'\b(MUST|NEVER|ALWAYS|DO NOT|IMPORTANT|CRITICAL)\b', t)),
        'checkbox': len(re.findall(r'^\s*- \[ \]', t, re.M)), 'table': len(re.findall(r'^\|.*\|\s*$', t, re.M)),
        'dot': t.count('digraph'), 'when': bool(re.search(r'when (to|not to) use|use (this|it) when', t, re.I)),
        'redflags': bool(re.search(r'rationaliz|red flag|excuse', t, re.I)),
        'side_files': len(side), 'scripts': sum(os.path.splitext(f)[1] in ('.py', '.sh', '.js', '.mjs', '.ts') for f in side),
    }

for c, pats in CLUSTERS.items():
    print('##', c)
    for pat in pats:
        p = find(pat)
        if not p:
            print('   MISSING', pat); continue
        m = metrics(p)
        print(f"   {os.path.relpath(os.path.dirname(p), ROOT).replace(os.sep, '/')[:62]:62} " + ' '.join(f'{k}={v}' for k, v in m.items()))
