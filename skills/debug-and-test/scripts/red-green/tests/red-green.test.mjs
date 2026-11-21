import { test } from 'node:test';
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { redGreen, verdict } from '../red-green.mjs';

test('verdict: only red-then-green proves the test', () => {
  assert.equal(verdict(1, 0).ok, true);
  assert.equal(verdict(0, 0).ok, false);
  assert.equal(verdict(1, 1).ok, false);
});

test('reverts the fix for the red run, restores it, and proves the test', () => {
  const dir = mkdtempSync(join(tmpdir(), 'rg-test-'));
  const git = (...a) => execFileSync('git', ['-c', 'user.name=t', '-c', 'user.email=t@t', '-c', 'core.autocrlf=false', ...a], { cwd: dir, stdio: 'ignore' });
  git('init', '-q');
  writeFileSync(join(dir, 'a.js'), 'module.exports = 1;\n');
  git('add', '.'); git('commit', '-qm', 'bug');
  writeFileSync(join(dir, 'a.js'), 'module.exports = 2;\n');
  const check = `node -e "process.exit(require('./a.js') === 2 ? 0 : 1)"`;
  const r = redGreen(check, ['a.js'], 'HEAD', dir);
  assert.equal(r.ok, true, r.msg);
  assert.equal(readFileSync(join(dir, 'a.js'), 'utf8'), 'module.exports = 2;\n');
  assert.equal(redGreen('node -e ""', ['a.js'], 'HEAD', dir).ok, false);
});
