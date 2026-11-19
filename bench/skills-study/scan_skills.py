"""Inventory every SKILL.md under the given repo dirs. Reads files only; never executes them."""
import json, os, re, sys

SCRIPT_EXT = {'.py', '.sh', '.js', '.mjs', '.ts', '.ps1', '.rb', '.go', '.rs'}
SKIP_DIRS = {'.git', 'node_modules', '.venv', 'dist', 'build'}

def licence(root):
    for n in os.listdir(root):
        if n.upper().startswith(('LICENSE', 'LICENCE', 'COPYING')):
            head = open(os.path.join(root, n), encoding='utf-8', errors='replace').read(400)
            for k in ('MIT', 'Apache', 'BSD', 'GPL', 'MPL', 'ISC', 'Unlicense', 'Creative Commons', 'CC BY'):
                if k.lower() in head.lower():
                    return k
            return 'other:' + head.strip().splitlines()[0][:40]
    return 'none'

def frontmatter(text):
    m = re.match(r'^---\s*\n(.*?)\n---\s*\n', text, re.S)
    if not m:
        return {}, text
    fm = {}
    for line in m.group(1).splitlines():
        if ':' in line and not line.startswith(' '):
            k, v = line.split(':', 1)
            fm[k.strip()] = v.strip().strip('"\'')
    return fm, text[m.end():]

def scan(root):
    repo = os.path.basename(root.rstrip('/\\'))
    lic = licence(root)
    out = []
    for d, dirs, files in os.walk(root):
        dirs[:] = [x for x in dirs if x not in SKIP_DIRS]
        if 'SKILL.md' not in files:
            continue
        p = os.path.join(d, 'SKILL.md')
        text = open(p, encoding='utf-8', errors='replace').read()
        fm, body = frontmatter(text)
        scripts, refs, other_bytes = [], [], 0
        for sd, sdirs, sfiles in os.walk(d):
            sdirs[:] = [x for x in sdirs if x not in SKIP_DIRS]
            for f in sfiles:
                fp = os.path.join(sd, f)
                if fp == p:
                    continue
                ext = os.path.splitext(f)[1].lower()
                if ext in SCRIPT_EXT:
                    scripts.append(os.path.relpath(fp, d))
                elif ext in ('.md', '.txt', '.csv', '.json', '.yaml', '.yml'):
                    refs.append(os.path.relpath(fp, d))
                    other_bytes += os.path.getsize(fp)
        low = body.lower()
        out.append({
            'repo': repo, 'licence': lic,
            'path': os.path.relpath(p, root).replace('\\', '/'),
            'name': fm.get('name') or os.path.basename(d),
            'desc_chars': len(fm.get('description', '')),
            'bytes': len(text.encode('utf-8')),
            'tokens': round(len(text.encode('utf-8')) / 4),
            'lines': text.count('\n') + 1,
            'scripts': len(scripts), 'refs': len(refs), 'ref_tokens': round(other_bytes / 4),
            'has_examples': bool(re.search(r'^#+ .*example', body, re.I | re.M)),
            'has_done_check': bool(re.search(r'(definition of done|done when|checklist|verify|verification)', low)),
            'has_antipatterns': bool(re.search(r"(anti-?pattern|don't|do not|never|avoid)", low)),
            'description': fm.get('description', '')[:300],
        })
    return out

if __name__ == '__main__':
    rows = [r for root in sys.argv[2:] for r in scan(root)]
    json.dump(rows, open(sys.argv[1], 'w', encoding='utf-8'), indent=1)
    print(len(rows), 'skills')
    assert all(r['tokens'] > 0 for r in rows)
