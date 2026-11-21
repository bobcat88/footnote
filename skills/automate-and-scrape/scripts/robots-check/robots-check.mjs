#!/usr/bin/env node
// robots-check: may this user agent fetch this URL under the site's robots.txt (RFC 9309)? Also prints crawl-delay and sitemaps.
// usage: node robots-check.mjs <url> [--ua MyBot] [--json]
// Exit 0 allowed, 1 disallowed, 2 usage. robots.txt states crawler preferences; it does not grant access or replace the site's terms.
import { pathToFileURL } from 'node:url';

export function parseRobots(text) {
  const groups = [], sitemaps = [];
  let cur = null, lastWasAgent = false;
  for (const raw of text.split(/\r?\n/)) {
    const m = /^\s*([A-Za-z-]+)\s*:\s*(.*?)\s*$/.exec(raw.replace(/#.*/, ''));
    if (!m) continue;
    const key = m[1].toLowerCase(), val = m[2];
    if (key === 'user-agent') {
      if (!lastWasAgent) groups.push((cur = { agents: [], rules: [], delay: undefined }));
      cur.agents.push(val.toLowerCase());
      lastWasAgent = true;
      continue;
    }
    lastWasAgent = false;
    if (key === 'sitemap') sitemaps.push(val);
    else if (cur && (key === 'allow' || key === 'disallow') && val) cur.rules.push({ allow: key === 'allow', path: val });
    else if (cur && key === 'crawl-delay') cur.delay = +val;
  }
  return { groups, sitemaps };
}

// Groups that name this agent (all of them merge), else the * groups.
export function groupFor(robots, ua) {
  const token = ua.toLowerCase().split('/')[0];
  const named = robots.groups.filter((g) => g.agents.some((a) => a !== '*' && token.includes(a)));
  const pick = named.length ? named : robots.groups.filter((g) => g.agents.includes('*'));
  return { rules: pick.flatMap((g) => g.rules), delay: pick.find((g) => g.delay !== undefined)?.delay };
}

const toRe = (p) => new RegExp('^' + p.replace(/[.+?^{}()|[\]\\]/g, '\\$&').replace(/\*/g, '.*').replace(/\\\$$|\$$/, '$'));

// Longest matching path wins; on a tie, Allow wins. No match = allowed.
export function allowed(rules, pathAndQuery) {
  let best = null;
  for (const r of rules) {
    if (!toRe(r.path).test(pathAndQuery)) continue;
    if (!best || r.path.length > best.path.length || (r.path.length === best.path.length && r.allow)) best = r;
  }
  return { allowed: best ? best.allow : true, rule: best };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const a = process.argv.slice(2), ui = a.indexOf('--ua'), ua = ui >= 0 ? a.splice(ui, 2)[1] : '*';
  const json = a.includes('--json'), target = a.find((x) => !x.startsWith('--'));
  if (!target) { console.error('usage: robots-check.mjs <url> [--ua MyBot] [--json]'); process.exit(2); }
  const url = new URL(target), robotsUrl = `${url.origin}/robots.txt`;
  let status, text = '';
  try { const res = await fetch(robotsUrl, { redirect: 'follow', signal: AbortSignal.timeout(15000) }); status = res.status; text = await res.text(); }
  catch (e) { status = `unreachable (${e.cause?.code ?? e.name})`; }
  let out;
  if (typeof status === 'number' && status >= 400 && status < 500) out = { allowed: true, why: `robots.txt ${status}: no rules, crawling allowed` };
  else if (status !== 200) out = { allowed: false, why: `robots.txt ${status}: treat the whole site as disallowed until it answers` };
  else {
    const robots = parseRobots(text), g = groupFor(robots, ua), v = allowed(g.rules, url.pathname + url.search);
    out = { allowed: v.allowed, why: v.rule ? `${v.rule.allow ? 'Allow' : 'Disallow'}: ${v.rule.path}` : 'no matching rule', crawlDelay: g.delay, sitemaps: robots.sitemaps };
  }
  if (json) console.log(JSON.stringify({ url: target, ua, robots: robotsUrl, ...out }, null, 2));
  else console.log(`${out.allowed ? 'ALLOWED' : 'DISALLOWED'} ${target} for ${ua}: ${out.why}${out.crawlDelay ? `\ncrawl-delay: ${out.crawlDelay} s` : ''}${out.sitemaps?.length ? `\nsitemaps: ${out.sitemaps.join(' ')}` : ''}`);
  process.exit(out.allowed ? 0 : 1);
}
