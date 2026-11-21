import { test } from 'node:test';
import assert from 'node:assert/strict';
import { lint } from '../prose-lint.mjs';

const ids = (t) => lint(t).map((f) => `${f.id}@${f.line}`);

test('plain text is clean', () => {
  assert.deepEqual(ids('The loader parses the file.\nA column rename fails the build.\n'), []);
});

test('tells are caught with their line numbers', () => {
  const t = [
    'Great question! Let us delve into it.',
    'We use a cache in order to save time — a lot.',
    'It is not just fast, but reliable.',
    '- **Speed:** speed went up. - **Cache:** a / b / c',
    'This could potentially break.',
    '“Quoted”',
  ].join('\n');
  assert.deepEqual(ids(t), ['P1-chatbot@1', 'P2-ai-word@1', 'P3-filler@2', 'P6-dash@2', 'P4-framing@3', 'P7-label@4', 'P5-hedge@5', 'P8-quotes@6']);
});

test('front matter, code, inline code, links, tables and prose-ok are skipped', () => {
  const t = ['---', 'description: leverage it', '---', '```', 'delve()', '```', 'Run `leverage --now`.', '[docs](https://x.y/delve) and https://x.y/leverage',
    '| a | b |', 'We delve here. <!-- prose-ok -->', 'Avoid "I hope this helps".'].join('\n');
  assert.deepEqual(ids(t), []);
});

test('sentences over 35 words are flagged once per line', () => {
  assert.deepEqual(ids(`${'word '.repeat(36)}end.`), ['P9-long@1']);
});
