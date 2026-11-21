import { test } from 'node:test';
import assert from 'node:assert/strict';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { pathToFileURL } from 'node:url';
import { addedLines, onDiff, sarifFindings } from '../sarif-diff.mjs';

const root = join(tmpdir(), 'repo');
const sarif = {
  runs: [{
    tool: { driver: { name: 'semgrep', rules: [{ id: 'sql-injection', defaultConfiguration: { level: 'error' } }, { id: 'weak-hash' }] } },
    results: [
      { ruleId: 'sql-injection', message: { text: 'user input reaches a query\nmore' }, locations: [{ physicalLocation: { artifactLocation: { uri: 'src/db.py' }, region: { startLine: 12 } } }] },
      { ruleIndex: 1, level: 'note', message: { text: 'md5' }, locations: [{ physicalLocation: { artifactLocation: { uri: pathToFileURL(join(root, 'src', 'old.py')).href }, region: { startLine: 3 } } }] },
      { ruleId: 'sql-injection', locations: [{ physicalLocation: { artifactLocation: { uri: './src/db.py' }, region: { startLine: 40 } } }] },
    ],
  }],
};

test('rule level, rule index and absolute file URIs are resolved', () => {
  assert.deepEqual(sarifFindings(sarif, root).map((f) => `${f.severity} ${f.rule} ${f.file}:${f.line}`),
    ['FAIL sql-injection src/db.py:12', 'INFO weak-hash src/old.py:3', 'FAIL sql-injection src/db.py:40']);
});

test('only findings on added lines are kept', () => {
  const diff = `--- a/src/db.py\n+++ b/src/db.py\n@@ -10,0 +11,2 @@\n+a\n+b\n`;
  assert.deepEqual(onDiff(sarifFindings(sarif, root), addedLines(diff)).map((f) => f.line), [12]);
});
