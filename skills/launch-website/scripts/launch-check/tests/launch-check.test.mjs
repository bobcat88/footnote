import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { gzipSync } from 'node:zlib';
import { launchCheck, pageFindings } from '../launch-check.mjs';

const GOOD = (t) => `<!doctype html><html lang="en"><head><title>${t}</title><meta name="viewport" content="width=device-width">
<meta name="description" content="d"><link rel="canonical" href="/"><meta property="og:title" content="t"><meta property="og:image" content="/i.png">
</head><body><h1>${t}</h1><img src="/i.png" alt=""><a href="/about">About</a></body></html>`;
const BAD = `<html><body><img src="x.png"><a href="/gone">x</a><a href="/about">again</a></body></html>`;
const SAFE = { 'content-security-policy': "default-src 'self'; frame-ancestors 'none'", 'x-content-type-options': 'nosniff', 'referrer-policy': 'strict-origin-when-cross-origin' };

function serve(routes, headers = {}) {
  return new Promise((ok) => {
    const s = createServer((req, res) => {
      const body = routes[req.url];
      if (body === undefined) { res.writeHead(404, { 'content-type': 'text/html' }); return res.end('nope'); }
      const type = req.url.endsWith('.txt') ? 'text/plain' : req.url.endsWith('.xml') ? 'application/xml' : 'text/html';
      res.writeHead(200, { 'content-type': type, 'content-encoding': 'gzip', ...headers });
      res.end(gzipSync(body));
    }).listen(0, '127.0.0.1', () => ok(s));
  });
}
const ids = (r) => r.findings.filter((f) => f.severity !== 'INFO').map((f) => f.id).sort();

test('clean site has no findings', async () => {
  const s = await serve({ '/': GOOD('Home'), '/about': GOOD('About'), '/robots.txt': 'User-agent: *', '/sitemap.xml': '<urlset/>' }, SAFE);
  const r = await launchCheck(`http://127.0.0.1:${s.address().port}/`);
  s.close();
  assert.equal(r.pages, 2);
  assert.deepEqual(ids(r), []);
});

test('broken site reports each problem', async () => {
  const s = await serve({ '/': BAD, '/about': BAD });
  const r = await launchCheck(`http://127.0.0.1:${s.address().port}/`);
  s.close();
  for (const id of ['L2-csp', 'L2-framing', 'L2-nosniff', 'L2-referrer', 'L3-title', 'L3-lang', 'L3-viewport', 'L3-description', 'L3-h1', 'L4-broken', 'L5-robots', 'L5-sitemap', 'L9-alt'])
    assert.ok(ids(r).includes(id), `expected ${id} in ${ids(r)}`);
  assert.ok(r.findings.some((f) => f.id === 'L4-broken' && f.url.endsWith('/gone')));
});

test('soft 404 is a FAIL', async () => {
  const s = createServer((_, res) => { res.writeHead(200, { 'content-type': 'text/html' }); res.end(GOOD('Same')); });
  await new Promise((ok) => s.listen(0, '127.0.0.1', ok));
  const r = await launchCheck(`http://127.0.0.1:${s.address().port}/`, { max: 2 });
  s.close();
  assert.ok(r.findings.some((f) => f.id === 'L7-soft404' && f.severity === 'FAIL'));
});

test('mixed content counts resources, not links, and only on https pages', () => {
  const html = `<html lang="en"><img src="http://x/a.png" alt=""><a href="http://x/page">ok</a><link rel="stylesheet" href="http://x/s.css">`;
  assert.equal(pageFindings('https://site/', html, new Headers()).f.find((f) => f.id === 'L8-mixed').msg, '2 resource(s) loaded over http://');
  assert.equal(pageFindings('http://site/', html, new Headers()).f.find((f) => f.id === 'L8-mixed'), undefined);
});

test('noindex is flagged', () => {
  assert.ok(pageFindings('https://s/', `<html lang="en"><meta name="robots" content="noindex">`, new Headers()).f.some((f) => f.id === 'L5-noindex'));
});
