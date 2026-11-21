import { test } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { checkSkill, skillHash, stamp } from '../skill-check.mjs';

const make = (name, skill, extra = {}) => {
  const root = mkdtempSync(join(tmpdir(), 'skill-')), dir = join(root, name);
  mkdirSync(dir);
  writeFileSync(join(dir, 'SKILL.md'), skill);
  for (const [p, body] of Object.entries(extra)) { mkdirSync(join(dir, p, '..'), { recursive: true }); writeFileSync(join(dir, p), body); }
  return { dir, done: () => rmSync(root, { recursive: true }) };
};
const GOOD = `---\nname: demo-skill\ndescription: Use when testing.\nlicense: MIT\n---\n\n# Demo\n- a / b / c\n- see [ref](references/x.md)\n`;
const ids = (f) => f.map((x) => `${x.id}:${x.severity}`).sort();

test('a stamped, linked, short skill is clean', () => {
  const s = make('demo-skill', GOOD, { 'references/x.md': 'x', '_sources.md': '# sources\n' });
  stamp(s.dir, '2026-11-21');
  assert.deepEqual(checkSkill(s.dir), []);
  assert.match(readFileSync(join(s.dir, '_sources.md'), 'utf8'), /^---\nskill_hash: [0-9a-f]{16}\nreviewed: 2026-11-21\n---\n# sources/);
  s.done();
});

test('editing the skill makes sources stale; editing _sources.md does not', () => {
  const s = make('demo-skill', GOOD, { 'references/x.md': 'x', '_sources.md': 'y' });
  stamp(s.dir);
  const h = skillHash(s.dir);
  writeFileSync(join(s.dir, '_sources.md'), readFileSync(join(s.dir, '_sources.md'), 'utf8') + '\nmore research');
  assert.equal(skillHash(s.dir), h);
  writeFileSync(join(s.dir, 'references/x.md'), 'changed');
  assert.deepEqual(ids(checkSkill(s.dir)), ['S4-stale:WARN']);
  s.done();
});

test('CRLF and LF hash the same', () => {
  const a = make('demo-skill', GOOD), b = make('demo-skill', GOOD.replace(/\n/g, '\r\n'));
  assert.equal(skillHash(a.dir), skillHash(b.dir));
  a.done(); b.done();
});

test('bad front matter, broken link, inline sources, length and denied terms are caught', () => {
  const long = `---\nname: other\ndescription: d\n---\n# X\n[gone](references/nope.md)\n## Sources\n` + 'line\n'.repeat(70);
  const s = make('demo-skill', long, { '_sources.md': 'Secret Client notes' });
  assert.deepEqual(ids(checkSkill(s.dir, { deny: ['secret client'] })),
    ['S1-license:WARN', 'S1-name:FAIL', 'S2-length:WARN', 'S3-link:FAIL', 'S4-sources:WARN', 'S5-sources-inline:WARN', 'S6-private:FAIL']);
  s.done();
});
