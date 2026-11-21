#!/usr/bin/env node
// red-green: proves a regression test catches the bug. Puts the fix files back to <base>, runs the test
// (it must fail), restores the fix, runs it again (it must pass).
// usage: node red-green.mjs --test "<command>" [--base HEAD] <fix files...>
// Exit 0 proved, 1 not proved, 2 usage. A backup of the fix files is written first and its path printed.
import { execFileSync, spawnSync } from 'node:child_process';
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { pathToFileURL } from 'node:url';

export function verdict(red, green) {
  if (red === 0) return { ok: false, msg: 'the test passes without the fix: it does not catch the bug' };
  if (green !== 0) return { ok: false, msg: 'the test fails with the fix in place' };
  return { ok: true, msg: 'red without the fix, green with it' };
}

export function redGreen(test, files, base = 'HEAD', cwd = process.cwd()) {
  const run = () => spawnSync(test, { shell: true, stdio: 'inherit', cwd }).status ?? 1;
  const saved = files.map((f) => readFileSync(join(cwd, f)));
  const old = files.map((f) => {
    try { return execFileSync('git', ['show', `${base}:./${f.replaceAll('\\', '/')}`], { cwd, stdio: ['ignore', 'pipe', 'ignore'] }); }
    catch { return null; } // file is new since base
  });
  const backup = mkdtempSync(join(tmpdir(), 'red-green-'));
  files.forEach((f) => { mkdirSync(dirname(join(backup, f)), { recursive: true }); copyFileSync(join(cwd, f), join(backup, f)); });
  console.error(`red-green: backup of the fix files in ${backup}`);
  let red;
  const ignore = () => {}; // Ctrl+C stops the test, not the restore
  process.on('SIGINT', ignore);
  try {
    files.forEach((f, i) => (old[i] === null ? unlinkSync(join(cwd, f)) : writeFileSync(join(cwd, f), old[i])));
    red = run();
  } finally {
    files.forEach((f, i) => writeFileSync(join(cwd, f), saved[i]));
    process.off('SIGINT', ignore);
  }
  const green = run();
  return { red, green, ...verdict(red, green) };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const a = process.argv.slice(2), opt = (k) => { const i = a.indexOf(k); return i < 0 ? undefined : a.splice(i, 2)[1]; };
  const test = opt('--test'), base = opt('--base') ?? 'HEAD';
  if (!test || !a.length) { console.error('usage: red-green.mjs --test "<command>" [--base HEAD] <fix files...>'); process.exit(2); }
  const r = redGreen(test, a, base);
  console.log(`${r.ok ? 'PROVED' : 'NOT PROVED'}: ${r.msg} (red exit ${r.red}, green exit ${r.green})`);
  process.exit(r.ok ? 0 : 1);
}
