#!/usr/bin/env node
// skill-check: lint skill folders and tie each one's research file (_sources.md) to the skill's content hash.
// usage: node skill-check.mjs [skills-dir | skill-dir...] [--stamp] [--deny terms.txt] [--json]
//   --stamp  after re-reviewing sources, write today's date and the current hash into each _sources.md
//   --deny   file of private terms (one per line) that must not appear in any skill file
// Exit 1 on any FAIL. Files starting with "_" are for maintainers: excluded from the hash and from agent sync.
import { createHash } from 'node:crypto';
import { existsSync, readdirSync, readFileSync, statSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, relative, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export const MAX_LINES = 60;        // house limit: a skill focuses the agent, it isn't a manual
const SPEC_MAX_LINES = 500;         // agentskills.io recommendation for SKILL.md
const SPEC_MAX_DESC = 1024;

const walk = (dir) => readdirSync(dir).flatMap((n) => {
  const p = join(dir, n);
  if (n === 'node_modules' || n === '__pycache__' || n.startsWith('.')) return []; // build caches, gitignored
  return statSync(p).isDirectory() ? walk(p) : [p];
});
const agentFiles = (dir) => walk(dir).filter((p) => !relative(dir, p).split(/[\\/]/).some((s) => s.startsWith('_'))).sort();

export function skillHash(dir) {
  const h = createHash('sha256');
  for (const p of agentFiles(dir)) {
    h.update(relative(dir, p).replaceAll('\\', '/') + '\0');
    h.update(readFileSync(p, 'utf8').replace(/\r\n/g, '\n') + '\0');
  }
  return h.digest('hex').slice(0, 16);
}

const frontMatter = (text) => {
  const m = text.match(/^---\r?\n([\s\S]*?)\r?\n---/);
  if (!m) return null;
  return Object.fromEntries(m[1].split(/\r?\n/).map((l) => l.match(/^(\w[\w-]*):\s*(.*)$/)).filter(Boolean).map((x) => [x[1], x[2].trim()]));
};

export function checkSkill(dir, { deny = [] } = {}) {
  const f = [], name = basename(dir), add = (id, severity, msg) => f.push({ id, severity, skill: name, msg });
  const skillMd = join(dir, 'SKILL.md');
  if (!existsSync(skillMd)) { add('S0-missing', 'FAIL', 'no SKILL.md'); return f; }
  const text = readFileSync(skillMd, 'utf8'), fm = frontMatter(text);
  if (!fm) add('S1-frontmatter', 'FAIL', 'no front matter');
  else {
    if (fm.name !== name) add('S1-name', 'FAIL', `name "${fm.name}" differs from folder "${name}"`);
    if (!/^[a-z0-9]+(-[a-z0-9]+)*$/.test(name) || name.length > 64) add('S1-name', 'FAIL', 'name must be lowercase-hyphenated, at most 64 characters');
    if (!fm.description) add('S1-description', 'FAIL', 'no description');
    else if (fm.description.length > SPEC_MAX_DESC) add('S1-description', 'FAIL', `description is ${fm.description.length} characters (max ${SPEC_MAX_DESC})`);
    if (!fm.license) add('S1-license', 'WARN', 'no license');
  }
  const lines = text.split(/\r?\n/).length;
  if (lines > SPEC_MAX_LINES) add('S2-length', 'FAIL', `SKILL.md is ${lines} lines (spec max ${SPEC_MAX_LINES})`);
  else if (lines > MAX_LINES) add('S2-length', 'WARN', `SKILL.md is ${lines} lines; keep it under ${MAX_LINES} and move detail to references/`);
  if (/^##\s+Sources\b/m.test(text)) add('S5-sources-inline', 'WARN', 'Sources section in SKILL.md; move it to _sources.md');
  for (const p of agentFiles(dir).filter((x) => x.endsWith('.md'))) {
    const body = readFileSync(p, 'utf8');
    for (const m of body.matchAll(/\]\(([^)#\s]+)(?:#[^)]*)?\)/g)) {
      const target = m[1];
      if (/^[a-z]+:/i.test(target)) continue;
      if (!existsSync(resolve(dirname(p), target))) add('S3-link', 'FAIL', `${relative(dir, p)} links to missing ${target}`);
    }
  }
  for (const p of walk(dir)) {
    const body = readFileSync(p, 'utf8').toLowerCase();
    for (const t of deny) if (body.includes(t)) add('S6-private', 'FAIL', `${relative(dir, p)} contains a denied term`);
  }
  const sources = join(dir, '_sources.md');
  if (!existsSync(sources)) add('S4-sources', 'WARN', 'no _sources.md (research and citations)');
  else {
    const sfm = frontMatter(readFileSync(sources, 'utf8')) ?? {};
    const now = skillHash(dir);
    if (!sfm.skill_hash) add('S4-sources', 'WARN', '_sources.md has no skill_hash; run --stamp after reviewing');
    else if (sfm.skill_hash !== now) add('S4-stale', 'WARN', `skill changed since sources were reviewed (${sfm.reviewed ?? 'no date'}); re-check claims, then --stamp`);
  }
  return f;
}

export function stamp(dir, today = new Date().toISOString().slice(0, 10)) {
  const p = join(dir, '_sources.md');
  if (!existsSync(p)) return false;
  const text = readFileSync(p, 'utf8'), hash = skillHash(dir), fm = frontMatter(text);
  const head = `---\nskill_hash: ${hash}\nreviewed: ${today}\n---\n`;
  writeFileSync(p, fm ? text.replace(/^---\r?\n[\s\S]*?\r?\n---\r?\n?/, head) : head + text);
  return true;
}

const skillDirs = (p) => existsSync(join(p, 'SKILL.md')) ? [p] : readdirSync(p).map((n) => join(p, n)).filter((d) => statSync(d).isDirectory() && !basename(d).startsWith('_'));

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2), take = (n) => { const i = args.indexOf(n); return i < 0 ? undefined : args.splice(i, 2)[1]; };
  const denyFile = take('--deny'), doStamp = args.includes('--stamp'), json = args.includes('--json');
  const deny = denyFile ? readFileSync(denyFile, 'utf8').split(/\r?\n/).map((s) => s.trim().toLowerCase()).filter(Boolean) : [];
  const dirs = (args.filter((a) => !a.startsWith('--')).length ? args.filter((a) => !a.startsWith('--')) : ['skills']).flatMap(skillDirs);
  if (doStamp) for (const d of dirs) if (stamp(d)) console.log(`stamped ${basename(d)} ${skillHash(d)}`);
  const all = dirs.flatMap((d) => checkSkill(d, { deny }));
  if (json) console.log(JSON.stringify(all, null, 2));
  else for (const s of ['FAIL', 'WARN']) for (const x of all.filter((x) => x.severity === s)) console.log(`${s} ${x.id.padEnd(18)} ${x.skill.padEnd(26)} ${x.msg}`);
  const fails = all.filter((x) => x.severity === 'FAIL').length;
  if (!json) console.log(`${dirs.length} skills, ${all.length} findings, ${fails} FAIL`);
  process.exit(fails ? 1 : 0);
}
