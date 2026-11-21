#!/usr/bin/env node
// launch-check: crawl a site from its start URL and check what a launch needs: statuses, redirects, security
// headers, page metadata, broken internal links, mixed content, missing alt, robots/sitemap, real 404s, compression.
// usage: node launch-check.mjs <url> [--max 50] [--external] [--out report.json]
// Exit 1 on any FAIL. Zero dependencies (Node 18+ fetch).
// shortcut: regex HTML parsing, fine for a launch filter; switch to a parser if a site defeats it.
import { writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const UA = 'launch-check/1.0';
const get = (url, opts = {}) => fetch(url, { redirect: 'manual', headers: { 'user-agent': UA }, signal: AbortSignal.timeout(15000), ...opts });
const attr = (tag, name) => tag.match(new RegExp(`\\s${name}\\s*=\\s*("([^"]*)"|'([^']*)'|([^\\s>]+))`, 'i'))?.slice(2).find((v) => v !== undefined);
const tags = (html, name) => html.match(new RegExp(`<${name}\\b[^>]*>`, 'gi')) ?? [];
const meta = (html, key) => tags(html, 'meta').find((t) => [attr(t, 'name'), attr(t, 'property')].some((v) => v?.toLowerCase() === key));

export function pageFindings(url, html, headers) {
  const f = [], add = (id, severity, msg) => f.push({ id, severity, url, msg });
  const title = html.match(/<title[^>]*>([\s\S]*?)<\/title>/i)?.[1].trim();
  if (!title) add('L3-title', 'FAIL', 'no <title>');
  if (!attr(tags(html, 'html')[0] ?? '', 'lang')) add('L3-lang', 'FAIL', '<html> has no lang (WCAG 3.1.1)');
  if (!meta(html, 'viewport')) add('L3-viewport', 'FAIL', 'no meta viewport');
  if (!meta(html, 'description')) add('L3-description', 'WARN', 'no meta description');
  if (!tags(html, 'link').some((t) => attr(t, 'rel')?.toLowerCase() === 'canonical')) add('L3-canonical', 'WARN', 'no canonical link');
  for (const k of ['og:title', 'og:image']) if (!meta(html, k)) add('L3-og', 'WARN', `no ${k} (sharing preview)`);
  const h1 = tags(html, 'h1').length;
  if (h1 !== 1) add('L3-h1', 'WARN', `${h1} <h1> elements`);
  const noindex = /noindex/i.test(attr(meta(html, 'robots') ?? '', 'content') ?? '') || /noindex/i.test(headers.get('x-robots-tag') ?? '');
  if (noindex) add('L5-noindex', 'WARN', 'page asks not to be indexed; right for staging, wrong for launch');
  const noAlt = tags(html, 'img').filter((t) => attr(t, 'alt') === undefined).length;
  if (noAlt) add('L9-alt', 'FAIL', `${noAlt} <img> without alt (WCAG 1.1.1; use alt="" for decoration)`);
  if (url.startsWith('https:')) {
    const res = tags(html, '(?:img|script|iframe|source|video|audio)').filter((t) => attr(t, 'src')?.startsWith('http://'));
    const css = tags(html, 'link').filter((t) => attr(t, 'href')?.startsWith('http://') && /stylesheet|preload|icon|manifest/i.test(attr(t, 'rel') ?? ''));
    const mixed = res.length + css.length;
    if (mixed) add('L8-mixed', 'FAIL', `${mixed} resource(s) loaded over http://`);
  }
  return { f, title };
}

export function headerFindings(url, headers) {
  const f = [], add = (id, msg) => f.push({ id, severity: 'WARN', url, msg });
  const csp = headers.get('content-security-policy') ?? '';
  if (url.startsWith('https:') && !headers.get('strict-transport-security')) add('L2-hsts', 'no Strict-Transport-Security');
  if (!csp) add('L2-csp', 'no Content-Security-Policy (start in Report-Only if unsure)');
  if (!/frame-ancestors/i.test(csp) && !headers.get('x-frame-options')) add('L2-framing', 'no frame-ancestors or X-Frame-Options (clickjacking)');
  if (headers.get('x-content-type-options')?.toLowerCase() !== 'nosniff') add('L2-nosniff', 'no X-Content-Type-Options: nosniff');
  if (!headers.get('referrer-policy')) add('L2-referrer', 'no Referrer-Policy');
  if (!headers.get('content-encoding')) add('L6-compression', 'HTML served uncompressed (no gzip/br)');
  return f;
}

const links = (html, base) => tags(html, 'a').map((t) => attr(t, 'href'))
  .filter((h) => h && !/^(mailto:|tel:|javascript:|#)/i.test(h))
  .map((h) => { try { const u = new URL(h, base); u.hash = ''; return u.href; } catch { return null; } }).filter(Boolean);

export async function launchCheck(start, { max = 50, external = false } = {}) {
  const origin = new URL(start).origin, findings = [], seen = new Set([start]), queue = [start], ext = new Set(), titles = new Map();
  const add = (id, severity, url, msg) => findings.push({ id, severity, url, msg });
  let pages = 0, headersDone = false;
  while (queue.length && pages < max) {
    const url = queue.shift();
    let res, hops = 0, at = url;
    try {
      res = await get(at);
      while (res.status >= 300 && res.status < 400 && res.headers.get('location') && hops < 5) {
        at = new URL(res.headers.get('location'), at).href;
        res = await get(at);
        hops++;
      }
    } catch (e) { add('L4-broken', 'FAIL', url, `unreachable: ${e.cause?.code ?? e.message}`); continue; }
    if (hops > 1) add('L1-chain', 'WARN', url, `${hops} redirects before content`);
    if (res.status >= 400) { add('L4-broken', 'FAIL', url, `HTTP ${res.status}`); continue; }
    if (new URL(at).origin !== origin || !/html/i.test(res.headers.get('content-type') ?? '')) continue;
    pages++;
    const html = await res.text();
    if (!headersDone) { findings.push(...headerFindings(at, res.headers)); headersDone = true; }
    const { f, title } = pageFindings(at, html, res.headers);
    findings.push(...f);
    if (title) titles.set(title, [...(titles.get(title) ?? []), at]);
    for (const l of links(html, at)) {
      if (new URL(l).origin === origin) { if (!seen.has(l)) { seen.add(l); queue.push(l); } }
      else if (/^https?:/.test(l)) ext.add(l);
    }
  }
  for (const [t, urls] of titles) if (urls.length > 1) add('L3-dup-title', 'WARN', urls[0], `title "${t}" shared by ${urls.length} pages`);
  if (queue.length) add('L0-max', 'INFO', start, `stopped at --max ${max}; ${queue.length} pages not checked`);

  if (start.startsWith('https:')) {
    try {
      const r = await get(start.replace(/^https:/, 'http:'));
      const loc = r.headers.get('location') ?? '';
      if (r.status < 300 || r.status >= 400) add('L1-https', 'FAIL', start, `http:// answers ${r.status} instead of redirecting to https`);
      else if (!loc.startsWith('https:')) add('L1-https', 'FAIL', start, `http:// redirects to ${loc || 'nowhere'}, not https`);
      else if (![301, 308].includes(r.status)) add('L1-https', 'WARN', start, `http→https uses ${r.status}; use 301 or 308`);
    } catch { add('L1-https', 'WARN', start, 'http:// not reachable (fine only if HSTS-preloaded)'); }
  }
  const probe = async (path) => { try { return (await get(new URL(path, origin))).status; } catch { return 0; } };
  if (await probe('/robots.txt') !== 200) add('L5-robots', 'WARN', origin, 'no /robots.txt');
  if (await probe('/sitemap.xml') !== 200) add('L5-sitemap', 'WARN', origin, 'no /sitemap.xml');
  const missing = await probe(`/launch-check-${Date.now()}`);
  if (missing !== 404 && missing !== 410) add('L7-soft404', 'FAIL', origin, `a missing page answers ${missing}, not 404`);

  if (external) {
    const list = [...ext];
    for (let i = 0; i < list.length; i += 8) await Promise.all(list.slice(i, i + 8).map(async (u) => {
      try {
        let r = await get(u, { method: 'HEAD', redirect: 'follow' });
        if (r.status === 405 || r.status === 403) r = await get(u, { redirect: 'follow' });
        if (r.status >= 400 && r.status !== 429) add('L4-external', 'WARN', u, `external link HTTP ${r.status}`);
      } catch (e) { add('L4-external', 'WARN', u, `external link unreachable: ${e.cause?.code ?? e.message}`); }
    }));
  }
  return { start, pages, externalLinks: ext.size, findings,
    notRun: ['legal notices, consent and privacy (manual, per jurisdiction)', 'Core Web Vitals (lab: Lighthouse; field: CrUX after launch)', 'full accessibility (design-check + manual)', 'certificate renewal ownership, backups, monitoring'] };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const args = process.argv.slice(2), flag = (n) => { const i = args.indexOf(n); return i < 0 ? undefined : args.splice(i, 2)[1]; };
  const max = Number(flag('--max') ?? 50), out = flag('--out'), external = args.includes('--external');
  const url = args.find((a) => !a.startsWith('--'));
  if (!url) { console.error('usage: node launch-check.mjs <url> [--max 50] [--external] [--out report.json]'); process.exit(2); }
  const r = await launchCheck(url, { max, external });
  if (out) writeFileSync(out, JSON.stringify(r, null, 2));
  for (const s of ['FAIL', 'WARN', 'INFO']) for (const x of r.findings.filter((x) => x.severity === s)) console.log(`${s.padEnd(4)} ${x.id.padEnd(16)} ${x.url}  ${x.msg}`);
  const fails = r.findings.filter((x) => x.severity === 'FAIL').length;
  console.log(`\n${r.pages} pages, ${r.findings.length} findings, ${fails} FAIL. Not run: ${r.notRun.join('; ')}.`);
  process.exit(fails ? 1 : 0);
}
