import { test } from 'node:test';
import assert from 'node:assert/strict';
import { allowed, groupFor, parseRobots } from '../robots-check.mjs';

const robots = parseRobots(`# comment
User-agent: *
Disallow: /private/
Allow: /private/public-page
Disallow: /*.pdf$
Crawl-delay: 5

User-agent: GoodBot
User-agent: OtherBot
Disallow:

Sitemap: https://example.com/sitemap.xml
`);

const can = (ua, path) => allowed(groupFor(robots, ua).rules, path).allowed;

test('longest match wins; Allow wins a tie; $ anchors; * matches anything', () => {
  assert.equal(can('*', '/private/x'), false);
  assert.equal(can('*', '/private/public-page'), true);
  assert.equal(can('*', '/docs/a.pdf'), false);
  assert.equal(can('*', '/docs/a.pdf?x=1'), true);
  assert.equal(can('*', '/'), true);
  assert.equal(allowed([{ allow: false, path: '/a' }, { allow: true, path: '/a' }], '/a').allowed, true);
});

test('a named group replaces *; consecutive user-agent lines share a group', () => {
  assert.equal(can('GoodBot/2.1', '/private/x'), true);
  assert.equal(can('otherbot', '/private/x'), true);
  assert.equal(groupFor(robots, '*').delay, 5);
  assert.deepEqual(robots.sitemaps, ['https://example.com/sitemap.xml']);
});
