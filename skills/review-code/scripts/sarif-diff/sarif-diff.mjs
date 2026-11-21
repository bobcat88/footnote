#!/usr/bin/env node
// sarif-diff: keeps static-analysis findings (SARIF 2.1) that sit on lines the branch added, so review starts with new problems.
// usage: node sarif-diff.mjs <file.sarif...> [--base origin/main] [--all] [--json]
// Exit 1 when an error-level finding remains. Run from the repository root.
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { isAbsolute, relative } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const LEVEL = { error: 'FAIL', warning: 'WARN', note: 'INFO', none: 'INFO' };

// file -> Set of line numbers added by a unified diff.
export function addedLines(diff) {
  const map = new Map();
  let file = null, line = 0, prev = '';
  for (const l of diff.split('\n')) {
    if (l.startsWith('+++ ') && prev.startsWith('--- ')) file = l === '+++ /dev/null' ? null : l.slice(6);
    else if (l.startsWith('@@')) line = +(/\+(\d+)/.exec(l)?.[1] ?? 0);
    else if (file && l.startsWith('+')) { if (!map.has(file)) map.set(file, new Set()); map.get(file).add(line++); }
    else if (file && l.startsWith(' ')) line++;
    prev = l;
  }
  return map;
}

const norm = (uri, root) => {
  let p = uri.startsWith('file:') ? fileURLToPath(uri) : decodeURIComponent(uri);
  if (root && isAbsolute(p)) p = relative(root, p);
  return p.replaceAll('\\', '/').replace(/^\.\//, '');
};

export function sarifFindings(sarif, root = '') {
  const out = [];
  for (const run of sarif.runs ?? []) {
    const rules = run.tool?.driver?.rules ?? [];
    for (const r of run.results ?? []) {
      const rule = r.ruleId ?? rules[r.ruleIndex]?.id ?? '?';
      const level = r.level ?? rules.find((x) => x.id === rule)?.defaultConfiguration?.level ?? 'warning';
      const loc = r.locations?.[0]?.physicalLocation;
      out.push({ tool: run.tool?.driver?.name ?? '?', rule, severity: LEVEL[level] ?? 'WARN', file: norm(loc?.artifactLocation?.uri ?? '', root), line: loc?.region?.startLine ?? 0, msg: (r.message?.text ?? '').split('\n')[0] });
    }
  }
  return out;
}

export const onDiff = (findings, added) => findings.filter((f) => added.get(f.file)?.has(f.line));

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const a = process.argv.slice(2), flag = (k) => { const i = a.indexOf(k); return i >= 0 && a.splice(i, 1); };
  const json = flag('--json'), all = flag('--all');
  const bi = a.indexOf('--base'), base = bi >= 0 ? a.splice(bi, 2)[1] : 'origin/main';
  if (!a.length) { console.error('usage: sarif-diff.mjs <file.sarif...> [--base origin/main] [--all] [--json]'); process.exit(2); }
  const git = (...x) => execFileSync('git', x, { encoding: 'utf8', maxBuffer: 1 << 28 }).trim();
  const root = git('rev-parse', '--show-toplevel');
  const found = a.flatMap((p) => sarifFindings(JSON.parse(readFileSync(p, 'utf8').replace(/^﻿/, '')), root));
  const kept = all ? found : onDiff(found, addedLines(git('diff', '-U0', '--no-color', '--no-ext-diff', git('merge-base', base, 'HEAD'))));
  if (json) console.log(JSON.stringify(kept, null, 2));
  else {
    for (const s of ['FAIL', 'WARN', 'INFO']) for (const x of kept.filter((x) => x.severity === s)) console.log(`${s} ${x.file}:${x.line}  ${x.tool}/${x.rule}  ${x.msg}`);
    console.log(`${kept.length} findings${all ? '' : ` on lines added since ${base}; ${found.length - kept.length} elsewhere (pre-existing or on unchanged lines)`}.`);
  }
  process.exit(kept.some((x) => x.severity === 'FAIL') ? 1 : 0);
}
