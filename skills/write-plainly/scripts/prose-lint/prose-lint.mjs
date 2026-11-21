#!/usr/bin/env node
// prose-lint: flags the word-level tells of machine-written prose in Markdown and text files.
// usage: node prose-lint.mjs <files or dirs...> [--json]
// Skips front matter, fenced code, inline code and URLs. A line containing `prose-ok` is skipped. Exit 1 on FAIL.
// shortcut: one sentence per line is assumed for P9; hard-wrapped paragraphs are measured line by line.
import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';

const words = (list) => new RegExp(`\\b(?:${list.join('|')})\\b`, 'i');
const RULES = [
  ['P1-chatbot', 'FAIL', /\bI hope this helps\b|\bgreat question\b|you're absolutely right|\bcertainly!|\bof course!|\bas an AI\b|\blet me know if you\b|\bhappy to help\b/i, 'chatbot phrase'],
  ['P2-ai-word', 'WARN', words(['delve[sd]?', 'pivotal', 'tapestry', 'testament', 'underscores?', 'showcas(?:e|es|ing)', 'fostering', 'garner', 'interplay', 'intricate', 'vibrant', 'seamless(?:ly)?', 'leverag(?:e|es|ing)', 'utiliz(?:e|es|ing)', 'facilitat(?:e|es|ing)', 'elevate', 'empower(?:s|ing)?', 'unlock(?:s|ing)?', 'game-?changer', 'cutting-edge', 'groundbreaking', 'realm', 'embark', 'holistic', 'synergy', 'paradigm', 'boasts']), 'AI vocabulary: use the plain word'],
  ['P3-filler', 'WARN', /\bin order to\b|\bdue to the fact that\b|\bit is (?:important|worth) (?:to note|noting) that\b|\bit's worth noting\b|\bserves as\b|\bstands as\b|\bat the end of the day\b|\bin today's\b/i, 'filler: cut it or say "to" / "because" / "is"'],
  ['P4-framing', 'WARN', /\bnot (?:just|only|merely) [^.;]{1,60}?,? but\b|\bplays? an? (?:crucial|key|vital|pivotal|important) role\b/i, 'framing device: state the point'],
  ['P5-hedge', 'WARN', /\b(?:could|might|may) (?:potentially|possibly)\b|\bpotentially could\b/i, 'stacked hedge: one is enough'],
  ['P6-dash', 'WARN', /—| – /, 'em dash: use a period or comma'],
  // A label that repeats in its own line ("**Speed:** speed improved") is the tell; a label heading a terse list is not.
  ['P7-label', 'WARN', /^\s*(?:[-*+]|\d+\.)\s+\*\*([A-Za-z]{4,})[^*]{0,36}(?::\*\*|\*\*:).*\b\1/i, 'bold label restated in its line: write it as a sentence'],
  ['P8-quotes', 'WARN', /[“”‘’]/, 'curly quotes: use straight quotes'],
];
const MAX_WORDS = 35;

export function lint(text, file = '') {
  const out = [];
  const lines = text.replace(/\r\n/g, '\n').split('\n');
  let fence = false, front = lines[0] === '---';
  lines.forEach((raw, i) => {
    if (front) { if (i > 0 && raw === '---') front = false; return; }
    if (/^\s*(```|~~~)/.test(raw)) { fence = !fence; return; }
    if (fence || raw.includes('prose-ok')) return;
    // Quoted text is a mention ("avoid \"delve\""), not a use.
    const l = raw.replace(/`[^`]*`/g, '``').replace(/"[^"]*"/g, '""').replace(/\]\([^)]*\)|https?:\/\/\S+/g, ']');
    for (const [id, severity, re, msg] of RULES) if (re.test(l)) out.push({ id, severity, file, line: i + 1, msg, text: raw.trim() });
    if (/^\s*\|/.test(l)) return; // table rows are not sentences
    for (const s of l.split(/(?<=[.!?])\s+/)) {
      const n = s.split(/\s+/).filter((w) => /\w/.test(w)).length;
      if (n > MAX_WORDS) { out.push({ id: 'P9-long', severity: 'WARN', file, line: i + 1, msg: `${n}-word sentence: split it`, text: s.trim() }); break; }
    }
  });
  return out;
}

const files = (p) => statSync(p).isDirectory()
  ? readdirSync(p, { recursive: true }).map((f) => join(p, f)).filter((f) => /\.(?:md|mdx|txt)$/i.test(f) && !/node_modules|[\\/]\./.test(f))
  : [p];

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2), json = args.includes('--json');
  const targets = args.filter((a) => a !== '--json');
  if (!targets.length) { console.error('usage: prose-lint.mjs <files or dirs...> [--json]'); process.exit(2); }
  const all = targets.flatMap(files).flatMap((p) => lint(readFileSync(p, 'utf8'), p));
  if (json) console.log(JSON.stringify(all, null, 2));
  else {
    for (const s of ['FAIL', 'WARN']) for (const x of all.filter((x) => x.severity === s)) console.log(`${s} ${x.id.padEnd(12)} ${x.file}:${x.line}  ${x.msg}  | ${x.text.slice(0, 70)}`);
    console.log(`${all.length} findings, ${all.filter((x) => x.severity === 'FAIL').length} FAIL. Words only: empty or vague sentences still need a read-through.`);
  }
  process.exit(all.some((x) => x.severity === 'FAIL') ? 1 : 0);
}
