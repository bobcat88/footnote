#!/usr/bin/env node
// workflow-check: static review of GitHub Actions workflows for the mistakes that most often turn into incidents.
// usage: node workflow-check.mjs [dir-or-files...]   (default .github/workflows)   [--json]
// Exit 1 on any FAIL. A clean run is a filter, not proof the workflows are secure.
// shortcut: line-based YAML reading, enough for these rules; a workflow written in flow style (`{ }`) can slip past.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const SHA = /^[0-9a-f]{40}$/;
// Values an outside contributor controls: titles, bodies, branch names, commit messages, labels, author details.
const UNTRUSTED = /github\.head_ref|github\.event\.(?:issue|pull_request|comment|review|review_comment|discussion|discussion_comment|head_commit|commits|pages|workflow_run)\b[\w.\[\]*]*\.(?:title|body|head_ref|ref|label|labels|message|name|email|default_branch|page_name|head_branch)\b/;

export function checkWorkflow(text, file = 'workflow.yml') {
  const f = [], add = (id, severity, line, msg) => f.push({ id, severity, file, line, msg });
  const lines = text.split(/\r?\n/);
  let block = null; // { indent } while inside a multi-line run: block

  if (!/^permissions\s*:/m.test(text)) add('W2-permissions', 'WARN', 1, 'no top-level permissions: the token gets the repository default, which may be write-all');
  lines.forEach((l, i) => { if (/^\s*permissions\s*:\s*write-all\b/.test(l)) add('W2-permissions', 'FAIL', i + 1, 'permissions: write-all'); });

  const prTarget = /^\s*(?:on\s*:.*\b|-\s*|)(pull_request_target|workflow_run)\b/m.exec(text)?.[1];
  if (prTarget) {
    const n = text.slice(0, text.indexOf(prTarget)).split('\n').length;
    const headCheckout = /ref\s*:\s*\$\{\{\s*github\.event\.(?:pull_request\.head\.(?:sha|ref)|workflow_run\.head_(?:sha|branch))/.test(text) || /github\.head_ref/.test(text);
    add('W4-privileged-trigger', headCheckout ? 'FAIL' : 'WARN', n, headCheckout
      ? `${prTarget} checks out the contributor's code with a privileged token and secrets`
      : `${prTarget} runs with secrets and a write token; never build or run the contributor's code in it`);
  }

  lines.forEach((l, i) => {
    const n = i + 1, indent = l.match(/^\s*/)[0].length;
    if (block && l.trim() && indent <= block.indent) block = null;
    const run = l.match(/^(\s*)(?:-\s+)?run\s*:\s*(.*)$/);
    const script = block ? l : run ? run[2] : null;
    if (run && /^[|>][-+]?\s*$/.test(run[2])) block = { indent: run[1].length };
    if (script) for (const m of script.matchAll(/\$\{\{([^}]*)\}\}/g)) {
      if (UNTRUSTED.test(m[1])) add('W3-injection', 'FAIL', n, `untrusted ${m[1].trim()} expanded inside run:; pass it through env: and quote "$VAR"`);
      else if (/github\.event\.|inputs\./.test(m[1])) add('W3-injection', 'WARN', n, `${m[1].trim()} expanded inside run:; prefer env:`);
    }
    const uses = l.match(/^\s*(?:-\s+)?uses\s*:\s*["']?([^\s"'#]+)/);
    if (uses) {
      const ref = uses[1];
      if (ref.startsWith('./')) return;
      if (ref.startsWith('docker://')) { if (!ref.includes('@sha256:')) add('W1-pin', 'FAIL', n, `${ref} is not pinned to an image digest`); return; }
      const [name, version = ''] = ref.split('@');
      if (!SHA.test(version)) add('W1-pin', /^(actions|github)\//.test(name) ? 'WARN' : 'FAIL', n, `${ref} is not pinned to a full commit SHA (add the tag as a comment)`);
    }
    if (/^\s*continue-on-error\s*:\s*true\b/.test(l)) add('W5-continue', 'WARN', n, 'continue-on-error: true can hide a failing required check');
  });
  return f;
}

const files = (p) => statSync(p).isDirectory()
  ? readdirSync(p).filter((x) => /\.ya?ml$/.test(x)).map((x) => join(p, x))
  : [p];

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2), json = args.includes('--json');
  const targets = args.filter((a) => a !== '--json');
  const all = (targets.length ? targets : ['.github/workflows']).flatMap(files).flatMap((p) => checkWorkflow(readFileSync(p, 'utf8'), p));
  if (json) console.log(JSON.stringify(all, null, 2));
  else for (const s of ['FAIL', 'WARN']) for (const x of all.filter((x) => x.severity === s)) console.log(`${s} ${x.id.padEnd(22)} ${x.file}:${x.line}  ${x.msg}`);
  const fails = all.filter((x) => x.severity === 'FAIL').length;
  if (!json) console.log(`${all.length} findings, ${fails} FAIL. Not checked: secrets exposure in logs, self-hosted runner hygiene, branch protection.`);
  process.exit(fails ? 1 : 0);
}
