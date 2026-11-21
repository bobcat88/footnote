import { test } from 'node:test';
import assert from 'node:assert/strict';
import { addedLines, leftovers } from '../leftover-check.mjs';

const diff = `diff --git a/src/a.js b/src/a.js
--- a/src/a.js
+++ b/src/a.js
@@ -10,0 +11,3 @@ fn
+const x = 1;
+console.log('[DEBUG-a4f2]', x);
+++counter;
@@ -40 +43 @@
-it('works', () => {});
+it.only('works', () => {});
diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -1,0 +2 @@
+Never commit \`it.only(\`.
diff --git a/lib.rs b/lib.rs
--- a/lib.rs
+++ b/lib.rs
@@ -1,0 +2,2 @@
+    todo!()
+    dbg!(v); // leftover-ok
`;

test('added lines keep their new numbers and survive a "++" line', () => {
  assert.deepEqual(addedLines(diff).slice(0, 4).map((a) => `${a.file}:${a.line}`), ['src/a.js:11', 'src/a.js:12', 'src/a.js:13', 'src/a.js:43']);
});

test('debug tags, focused tests and stubs are caught; docs and leftover-ok are skipped', () => {
  assert.deepEqual(leftovers(diff).map((f) => `${f.id}@${f.line}`), ['D1-debug-tag@12', 'D5-print@12', 'D3-focused-test@43', 'D6-stub@2']);
});
