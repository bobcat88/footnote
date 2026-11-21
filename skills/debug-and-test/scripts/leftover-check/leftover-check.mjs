#!/usr/bin/env node
// leftover-check: flags debug leftovers and disabled tests in the lines a change adds.
// usage: node leftover-check.mjs [base] [--json]   (no base: uncommitted changes against HEAD)
// Exit 1 on any FAIL. A line containing `leftover-ok` is skipped.
// shortcut: untracked files are not in `git diff`; `git add -N <file>` first to include them.
import { execFileSync } from 'node:child_process';
import { pathToFileURL } from 'node:url';

const RULES = [
  ['D1-debug-tag', 'FAIL', /\[DEBUG-[\w-]+\]/, 'debug tag left in'],
  ['D2-breakpoint', 'FAIL', /\bdebugger\s*;|\bbreakpoint\(\)|pdb\.set_trace\(|binding\.pry\b/, 'breakpoint left in'],
  ['D3-focused-test', 'FAIL', /\b(?:it|test|describe|context)\.only\(|\bf(?:it|describe)\(/, 'focused test: the rest of the suite stops running'],
  ['D4-skipped-test', 'WARN', /\b(?:it|test|describe)\.skip\(|\bx(?:it|describe)\(|#\[ignore\]|@pytest\.mark\.skip|@unittest\.skip|\bt\.Skip\(/, 'test disabled: link an issue or remove it'],
  ['D5-print', 'WARN', /\bconsole\.(?:log|debug)\(|\bdbg!\(|^\s*print\(/, 'debug print'],
  ['D6-stub', 'WARN', /\btodo!\(|\bunimplemented!\(|raise NotImplementedError/, 'stub reachable at run time'],
];
const SKIP_FILE = /\.(?:md|txt|rst)$/i;

// Added lines of a unified diff, with their new line numbers.
export function addedLines(diff) {
  const out = [];
  let file = null, line = 0, prev = '';
  for (const l of diff.split('\n')) {
    if (l.startsWith('+++ ') && prev.startsWith('--- ')) file = l === '+++ /dev/null' ? null : l.slice(6);
    else if (l.startsWith('@@')) line = +(/\+(\d+)/.exec(l)?.[1] ?? 0);
    else if (file && l.startsWith('+')) out.push({ file, line: line++, text: l.slice(1) });
    else if (file && l.startsWith(' ')) line++;
    prev = l;
  }
  return out;
}

export function leftovers(diff) {
  const found = [];
  for (const a of addedLines(diff)) {
    if (SKIP_FILE.test(a.file) || a.text.includes('leftover-ok')) continue;
    for (const [id, severity, re, msg] of RULES) if (re.test(a.text)) found.push({ id, severity, file: a.file, line: a.line, msg, text: a.text.trim() });
  }
  return found;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2), json = args.includes('--json');
  const base = args.find((a) => !a.startsWith('--'));
  const git = (...a) => execFileSync('git', a, { encoding: 'utf8', maxBuffer: 1 << 28 });
  const from = base ? git('merge-base', base, 'HEAD').trim() : 'HEAD';
  const all = leftovers(git('diff', '-U0', '--no-color', '--no-ext-diff', from));
  if (json) console.log(JSON.stringify(all, null, 2));
  else {
    for (const s of ['FAIL', 'WARN']) for (const x of all.filter((x) => x.severity === s)) console.log(`${s} ${x.id.padEnd(16)} ${x.file}:${x.line}  ${x.msg}  | ${x.text.slice(0, 80)}`);
    console.log(`${all.length} findings since ${base ?? 'HEAD'}, ${all.filter((x) => x.severity === 'FAIL').length} FAIL.`);
  }
  process.exit(all.some((x) => x.severity === 'FAIL') ? 1 : 0);
}
