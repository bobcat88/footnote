// One-shot visual check (design-studio directions and builds): first screen (PNG) and whole page (JPEG) at 1920x1080 on real Chrome.
// usage: node <skill>/scripts/studio/shot.mjs <page.html> <out-prefix>
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { loadPlaywright } from '../../../review-ui-quality/scripts/slop-scan/slop-scan.mjs';
import { launchGpu } from '../../../review-ui-quality/scripts/ref-capture/ref-capture.mjs';
const [page, out] = process.argv.slice(2);
if (!page || !out) { console.error('usage: node shot.mjs <page.html> <out-prefix>'); process.exit(2); }
const { browser } = await launchGpu(await loadPlaywright());
const p = await (await browser.newContext({ viewport: { width: 1920, height: 1080 } })).newPage();
const errors = [];
p.on('pageerror', (e) => errors.push(e.message)); p.on('console', (m) => m.type() === 'error' && errors.push(m.text()));
await p.goto(pathToFileURL(resolve(page)).href, { waitUntil: 'load' });
await p.waitForTimeout(2500);
await p.screenshot({ path: out + '-top.png' });
await p.screenshot({ path: out + '-full.jpg', fullPage: true, type: 'jpeg', quality: 60 });
const overflow = await p.evaluate(() => document.documentElement.scrollWidth > document.documentElement.clientWidth);
console.log(JSON.stringify({ saved: [out + '-top.png', out + '-full.jpg'], horizontalOverflow: overflow, consoleErrors: errors.slice(0, 5) }));
await browser.close();
