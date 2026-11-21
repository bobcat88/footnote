#!/usr/bin/env node
// ref-capture: turn live sites into a design reference pack (screenshots + tokens + a motion fingerprint).
//   node ref-capture.mjs <url> [<url> ...] --out <dir> [--shots 6] [--wait 6000]
// Per site it writes <out>/<slug>/{01-hero.jpg, 02.jpg, ...}, tokens.json, and one <out>/INDEX.md for the run.
// Viewport is always 1920x1080. It only reads: screenshots, computed styles, script text scanned with regexes.
// Motion tier: "css" (transitions and keyframes only), "scripted" (GSAP, Lenis, rAF loops, WAAPI), "webgl" (a real canvas or shaders).
// Signatures are evidence, not proof: minified bundles can hide names, cross-origin CSS cannot be read (counted, not hidden).
// Only dependency: playwright-core, resolved by slop-scan.mjs.
import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync, readdirSync, readFileSync, existsSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { loadPlaywright } from '../slop-scan/slop-scan.mjs';

// Real Chrome renders WebGL on the GPU. Playwright's default headless shell falls back to software (SwiftShader):
// measured on lusion.co, 2 frames/s against 120 on the GPU, which freezes entrance animations half-way. Chrome first.
export async function launchGpu(pw) {
  try { return { browser: await pw.chromium.launch({ channel: 'chrome' }), engine: 'chrome' }; }
  catch { return { browser: await pw.chromium.launch(), engine: 'chromium-shell (software WebGL: motion captures are unreliable)' }; }
}

const VIEWPORT = { width: 1920, height: 1080 };
const wait = (ms) => new Promise((r) => setTimeout(r, ms));
const SIGS = {
  gsap: /gsap\.(?:to|from|fromTo|timeline|set|registerPlugin)\b|GreenSock/,
  ScrollTrigger: /ScrollTrigger/,
  Lenis: /\bLenis\b/,
  locomotive: /LocomotiveScroll|locomotive-scroll/,
  three: /WebGLRenderer|\bTHREE\./,
  shaders: /gl_FragColor|gl_Position|precision\s+(?:highp|mediump)\s+float/,
  postprocessing: /EffectComposer|UnrealBloomPass/,
  barba: /barba\.init|@barba\/core/,
  swup: /new Swup|swup\.js/,
  viewTransitions: /startViewTransition/,
  framerMotion: /framer-motion|data-framer-/,
  lottie: /lottie-web|bodymovin|lottie\.loadAnimation/,
  rive: /@rive-app|rive-canvas/,
  spline: /@splinetool|spline-viewer/,
  pixi: /\bPIXI\./,
  splitText: /SplitText|SplitType|Splitting\(/,
  flip: /Flip\.(?:getState|from)\b/,
  physics: /RAPIER|Matter\.Engine|cannon-es/,
  webflow: /Webflow\.push|window\.Webflow/,
  swiper: /new Swiper|swiper-bundle/,
};
const scanText = (text, where, into) => { for (const [k, re] of Object.entries(SIGS)) if (re.test(text)) (into[k] ||= new Set()).add(where); };

// Runs before any page script: records canvas context types, WAAPI use, View Transition calls and rAF count.
function probeInit() {
  const p = (window.__probe = { ctx: [], waapi: 0, vt: 0, raf: 0 });
  const gc = HTMLCanvasElement.prototype.getContext;
  HTMLCanvasElement.prototype.getContext = function (t, ...a) { p.ctx.push(String(t)); return gc.call(this, t, ...a); };
  const an = Element.prototype.animate;
  Element.prototype.animate = function (...a) { p.waapi++; return an.apply(this, a); };
  if (document.startViewTransition) { const v = document.startViewTransition.bind(document); document.startViewTransition = (...a) => { p.vt++; return v(...a); }; }
  const raf = window.requestAnimationFrame;
  window.requestAnimationFrame = function (cb) { p.raf++; return raf.call(window, cb); };
}

// In-page: design tokens from computed styles. Self-contained (Playwright serialises it).
function readTokens() {
  const vw = innerWidth, vh = innerHeight;
  const hex = (c) => {
    const m = /rgba?\((\d+),\s*(\d+),\s*(\d+)(?:,\s*([\d.]+))?/.exec(c || '');
    if (!m || (m[4] !== undefined && parseFloat(m[4]) < 0.5)) return null;
    return '#' + [m[1], m[2], m[3]].map((v) => (+v).toString(16).padStart(2, '0')).join('');
  };
  const top = (map, n) => [...map.entries()].sort((a, b) => b[1] - a[1]).slice(0, n);
  const add = (m, k, w = 1) => { if (k) m.set(k, (m.get(k) || 0) + w); };
  const els = [...document.querySelectorAll('body *')].filter((e) => { const r = e.getBoundingClientRect(); return r.width > 0 && r.height > 0 && getComputedStyle(e).visibility !== 'hidden'; }).slice(0, 4000);
  const bg = new Map(), fg = new Map(), rad = new Map(), shadow = new Map(), tr = new Map();
  const uses = { backdropFilter: 0, blendMode: 0, clipPath: 0, mask: 0, willChange: 0, video: 0, svgAnimate: 0, sticky: 0 };
  for (const e of els) {
    const cs = getComputedStyle(e), r = e.getBoundingClientRect();
    add(bg, hex(cs.backgroundColor), Math.min(r.width, vw) * Math.min(r.height, vh * 2));
    const own = [...e.childNodes].filter((n) => n.nodeType === 3).reduce((s, n) => s + n.textContent.trim().length, 0);
    if (own) add(fg, hex(cs.color), own * parseFloat(cs.fontSize));
    const br = parseFloat(cs.borderTopLeftRadius);
    if (br >= 2 && r.width >= 24) add(rad, cs.borderTopLeftRadius.includes('%') ? cs.borderTopLeftRadius : Math.round(br) + 'px');
    if (cs.boxShadow !== 'none') add(shadow, cs.boxShadow.slice(0, 90));
    const durs = cs.transitionDuration.split(',').map((x) => parseFloat(x) || 0);
    if (durs.some((d) => d > 0)) {
      const props = cs.transitionProperty.split(',').map((x) => x.trim());
      const fns = cs.transitionTimingFunction.match(/cubic-bezier\([^)]*\)|steps\([^)]*\)|[a-z-]+/g) || [];
      durs.forEach((d, i) => { if (d > 0) add(tr, `${props[i % props.length]} ${d}s ${fns[i % fns.length] || ''}`.trim()); });
    }
    if (cs.backdropFilter !== 'none' && cs.backdropFilter) uses.backdropFilter++;
    if (cs.mixBlendMode !== 'normal') uses.blendMode++;
    if (cs.clipPath !== 'none') uses.clipPath++;
    if (cs.maskImage && cs.maskImage !== 'none') uses.mask++;
    if (cs.willChange !== 'auto') uses.willChange++;
    if (cs.position === 'sticky') uses.sticky++;
    if (e.tagName === 'VIDEO') uses.video++;
    if (e.tagName === 'ANIMATE' || e.tagName === 'ANIMATETRANSFORM') uses.svgAnimate++;
  }
  const first = (sel) => { const e = [...document.querySelectorAll(sel)].find((x) => x.getBoundingClientRect().width > 0 && getComputedStyle(x).visibility !== 'hidden'); if (!e) return null; const c = getComputedStyle(e); return { family: c.fontFamily.slice(0, 80), size: c.fontSize, weight: c.fontWeight, lineHeight: c.lineHeight, letterSpacing: c.letterSpacing, transform: c.textTransform }; };
  const type = {}; for (const s of ['h1', 'h2', 'h3', 'p', 'button', 'nav a', 'small']) type[s] = first(s);
  const kf = new Set(); let viewTransitionCss = false, scrollTimeline = false, reducedMotionRule = false, unreadableSheets = 0;
  const walk = (rs) => { for (const r of rs) {
    if (r.type === 7) kf.add(r.name);
    if (r.type === 4 && /prefers-reduced-motion/.test(r.conditionText || r.media?.mediaText || '')) reducedMotionRule = true;
    if (r.type === 1 && r.selectorText && /::view-transition/.test(r.selectorText)) viewTransitionCss = true;
    if (r.type === 1 && /animation-timeline|view-timeline|scroll-timeline/.test(r.cssText)) scrollTimeline = true;
    if (/^@view-transition/.test(r.cssText || '')) viewTransitionCss = true;
    if (r.cssRules) walk(r.cssRules);
  } };
  for (const ss of document.styleSheets) { try { walk(ss.cssRules); } catch { unreadableSheets++; } }
  const bodyBg = hex(getComputedStyle(document.body).backgroundColor) || hex(getComputedStyle(document.documentElement).backgroundColor);
  const lum = (h) => { if (!h) return null; const v = [1, 3, 5].map((i) => parseInt(h.slice(i, i + 2), 16) / 255); return 0.2126 * v[0] + 0.7152 * v[1] + 0.0722 * v[2]; };
  const canvases = [...document.querySelectorAll('canvas')].filter((c) => c.clientWidth > 0).map((c) => ({ w: c.clientWidth, h: c.clientHeight }));
  return {
    title: document.title, description: document.querySelector('meta[name=description]')?.content || '', lang: document.documentElement.lang,
    scheme: lum(bodyBg) === null ? 'unknown' : lum(bodyBg) < 0.4 ? 'dark' : 'light', bodyBg,
    backgrounds: top(bg, 8), textColors: top(fg, 6), radii: top(rad, 5), shadows: top(shadow, 3),
    fonts: [...new Set([...document.fonts].filter((f) => f.status === 'loaded').map((f) => f.family.replace(/"/g, '')))], type,
    transitions: top(tr, 8), keyframes: [...kf].slice(0, 20), viewTransitionCss, scrollTimeline, reducedMotionRule, unreadableSheets,
    canvases, uses,
    globals: ['gsap', 'ScrollTrigger', 'Lenis', 'THREE', 'lottie', 'Webflow', 'barba', 'Swiper', 'PIXI', 'jQuery', '__NEXT_DATA__', '__NUXT__', 'Alpine', 'htmx'].filter((k) => k in window),
    inlineScripts: [...document.scripts].filter((s) => !s.src).map((s) => s.textContent.slice(0, 2e6)),
    scrollHeight: document.documentElement.scrollHeight,
  };
}

const slugOf = (u) => { const x = new URL(u); return (x.hostname.replace(/^www\./, '') + (x.pathname === '/' ? '' : x.pathname)).replace(/[^a-z0-9]+/gi, '-').replace(/^-|-$/g, '').toLowerCase(); };

export async function capture(browser, url, outDir, { shots = 6, waitMs = 6000, engine = '', hide = '' } = {}) {
  const slug = slugOf(url), dir = join(outDir, slug);
  mkdirSync(dir, { recursive: true });
  const ctx = await browser.newContext({ viewport: VIEWPORT });
  await ctx.addInitScript(probeInit);
  const page = await ctx.newPage();
  const sigs = {}, scripts = []; let scanned = 0, scannedBytes = 0;
  page.on('response', async (r) => {
    try {
      if (r.request().resourceType() !== 'script' || !r.ok()) return;
      const name = new URL(r.url()).pathname.split('/').pop() || r.url();
      scripts.push(new URL(r.url()).host + '/' + name);
      if (scanned >= 60 || scannedBytes > 40e6 || Number(r.headers()['content-length'] || 0) > 4e6) return;
      scanned++;
      const t = await r.text(); scannedBytes += t.length; scanText(t, name, sigs);
    } catch { /* body unavailable */ }
  });
  const result = { url, slug, shots: [], error: null, engine };
  try {
    await page.goto(url, { waitUntil: 'domcontentloaded', timeout: 45000 }).catch((e) => { result.error = 'goto: ' + String(e.message).split('\n')[0]; });
    await page.waitForTimeout(waitMs);
    // --hide: remove a preloader that never clears (ventriloc.ca's .site-loader). Recorded in the result, never silent.
    result.hidden = hide || null;
    if (hide) await page.addStyleTag({ content: `${hide}{display:none!important}` }).catch(() => {});
    // consent gates can lock scrolling (enpowertrading.co.za reports a 1080px page until "Confirm" is clicked)
    const ok = page.getByRole('button', { name: /^(accept( all)?( cookies)?|agree|allow( all)?|got it|ok(ay)?|i agree|confirm|continue|yes)$/i }).first();
    result.consentClicked = false;
    if (await ok.count().catch(() => 0)) { result.consentClicked = await ok.click({ timeout: 2000 }).then(() => true).catch(() => false); await page.waitForTimeout(1200); }
    const shot = async (name) => { const buf = await page.screenshot({ type: 'jpeg', quality: 72 }); writeFileSync(join(dir, name), buf); result.shots.push(name); return createHash('sha1').update(buf).digest('hex'); };
    // idle motion: does the first screen change by itself over 0.9 s? (two frames, both kept)
    const h1 = await shot('01-hero.jpg');
    await page.waitForTimeout(900);
    const h2 = await shot('01b-hero-plus-0.9s.jpg');
    result.idleMotion = h1 !== h2;
    const t = await page.evaluate(readTokens);
    for (const s of t.inlineScripts) scanText(s, 'inline', sigs);
    delete t.inlineScripts;
    // rAF density while idle
    await page.evaluate(() => { window.__probe.raf = 0; });
    await page.waitForTimeout(2000);
    const probe = await page.evaluate(() => window.__probe);
    const rafPerSec = Math.round(probe.raf / 2);
    // the renderer string (read after the probe, because creating a canvas here would be recorded as a page context)
    result.renderer = await page.evaluate(() => { const c = document.createElement('canvas'); const g = c.getContext('webgl2') || c.getContext('webgl'); if (!g) return 'no webgl'; const e = g.getExtension('WEBGL_debug_renderer_info'); return e ? g.getParameter(e.UNMASKED_RENDERER_WEBGL) : 'hidden'; });
    // cursor reaction: move the pointer across the hero, keep the frame
    await page.mouse.move(480, 300); await page.waitForTimeout(150);
    await page.mouse.move(1440, 760, { steps: 12 }); await page.waitForTimeout(700);
    await shot('01c-hero-cursor.jpg');
    // scroll through with real wheel events (works for native scroll, Lenis and virtual scroll)
    await page.mouse.move(VIEWPORT.width / 2, VIEWPORT.height / 2);
    // A page that reports one screen of height may be waiting for its first wheel input (enpowertrading.co.za grows from 1080 to 29080px after it).
    if (t.scrollHeight <= VIEWPORT.height + 8) {
      for (let k = 0; k < 3; k++) { for (let i = 0; i < 10; i++) { await page.mouse.wheel(0, 100); await wait(16); } await page.waitForTimeout(1500); }
      t.scrollHeight = await page.evaluate(() => document.documentElement.scrollHeight);
      if (t.scrollHeight > VIEWPORT.height + 8) { await page.evaluate(() => scrollTo(0, 0)); await page.waitForTimeout(1500); }
    }
    const total = Math.max(VIEWPORT.height * 0.9, Math.min(t.scrollHeight - VIEWPORT.height, 60000) / Math.max(1, shots - 1));
    const scrollY = [0];
    for (let i = 2; i <= shots; i++) {
      for (let d = 0; d < total; d += 300) { await page.mouse.wheel(0, Math.min(300, total - d)); await wait(16); }
      await page.waitForTimeout(1200);
      scrollY.push(await page.evaluate(() => Math.round(scrollY)));
      await shot(String(i).padStart(2, '0') + '.jpg');
    }
    const webglCtx = probe.ctx.some((c) => /webgl|webgpu/i.test(c)), bigCanvas = t.canvases.some((c) => c.w >= 300 && c.h >= 300);
    const libs = Object.fromEntries(Object.entries(sigs).map(([k, v]) => [k, [...v].slice(0, 3)]));
    const has = (k) => k in libs || t.globals.includes(k);
    // webgl needs a real context or Three.js; a shader-looking string alone (framework runtimes carry them) is not enough
    const webgl = (webglCtx && bigCanvas) || (webglCtx && has('three')) || (has('three') && has('shaders')) || has('THREE');
    const scripted = has('gsap') || has('ScrollTrigger') || has('Lenis') || has('locomotive') || has('framerMotion') || has('rive') || has('lottie') || has('spline') || has('pixi') || rafPerSec > 20 || probe.waapi > 20;
    // a View Transition counts only when the page calls it or styles it; the string in a bundle is just capability
    const pageTransitions = [has('barba') && 'barba', has('swup') && 'swup', (t.viewTransitionCss || probe.vt > 0) && 'view-transitions-api'].filter(Boolean);
    Object.assign(result, {
      tier: webgl ? 'webgl' : scripted ? 'scripted' : 'css', pageTransitions, viewTransitionCalls: probe.vt, viewTransitionsInBundle: 'viewTransitions' in libs, libs, rafPerSec, waapiCalls: probe.waapi, canvasContexts: [...new Set(probe.ctx)],
      scrollY, virtualScroll: scrollY.every((y) => y === 0), scriptCount: scripts.length, scriptsScanned: scanned, tokens: t,
    });
  } catch (e) { result.error = String(e.message).split('\n')[0]; } finally { await ctx.close(); }
  writeFileSync(join(dir, 'tokens.json'), JSON.stringify(result, null, 2) + '\n');
  return result;
}

const SKILLS = { css: 'transitions-dev', scripted: 'gsap-animation, transitions-dev, react-bits', webgl: 'gsap-animation for the DOM layer; **no skill for WebGL / Three.js / shaders (gap)**' };
export function indexNote(results, date) {
  const L = ['---', 'type: reference', 'status: active', 'tags:', '  - domain/design', '  - domain/dev/ui', '  - type/reference', `date_maj: ${date}`, '---', '# Design references: captured sites',
    'Up: [[UI-Design-Ruleset]] · Related: [[WebUI Preferred Style Guide - Dataiku Dobre Ponder]] · [[ui-slop-patterns]] · [[qloop-ui-protocol]]', '',
    `Captured ${date} at 1920×1080 with \`ref-capture\` (\`AI-Agents/skills/ui-ux-qloop/scripts/ref-capture/\`). Tokens are read from computed styles; library names are signatures found in scripts, evidence not proof. Cross-origin CSS cannot be read (count shown).`, '',
    '| Tier | Meaning | Skills in the stack |', '|---|---|---|', ...Object.entries(SKILLS).map(([k, v]) => `| ${k} | ${{ css: 'CSS transitions and keyframes only', scripted: 'GSAP, Lenis, rAF loops or WAAPI drive the motion', webgl: 'a real WebGL canvas or shaders' }[k]} | ${v} |`), '',
    '| Site | Tier | Page transitions | Idle motion | Libraries |', '|---|---|---|---|---|',
    ...results.map((r) => `| [${r.slug}](#${r.slug}) | ${r.tier || 'error'} | ${(r.pageTransitions || []).join(', ') || '-'} | ${r.idleMotion ? 'yes' : 'no'} | ${Object.keys(r.libs || {}).join(', ') || '-'} |`), ''];
  for (const r of results) {
    L.push(`## ${r.slug}`, `<${r.url}>`, '');
    if (r.error && !r.tier) { L.push(`Capture failed: ${r.error}`, ''); continue; }
    const t = r.tokens;
    L.push(`**What I like:** _(yours, one or two lines)_`, '',
      `- **Scheme:** ${t.scheme} · bg ${t.bodyBg || '?'} · text ${t.textColors.slice(0, 3).map((c) => c[0]).join(', ')}`,
      `- **Colours by area:** ${t.backgrounds.slice(0, 6).map((c) => c[0]).join(', ')}`,
      `- **Fonts:** ${t.fonts.join(', ') || 'none reported'} · h1 ${t.type.h1?.size || '?'}/${t.type.h1?.weight || '?'} · body ${t.type.p?.size || '?'}`,
      `- **Radii:** ${t.radii.map((c) => c[0]).join(', ') || '-'} · **Shadows:** ${t.shadows.length}`,
      `- **Transitions (property, duration, easing):** ${t.transitions.slice(0, 5).map((c) => c[0]).join(' · ') || '-'}`,
      `- **Keyframes:** ${t.keyframes.slice(0, 8).join(', ') || '-'} · scroll-driven CSS: ${t.scrollTimeline ? 'yes' : 'no'} · reduced-motion rule: ${t.reducedMotionRule ? 'yes' : 'no'} · unreadable stylesheets: ${t.unreadableSheets}`,
      `- **Motion:** ${r.tier} · rAF ${r.rafPerSec}/s idle · WAAPI calls ${r.waapiCalls} · canvases ${t.canvases.length} (${r.canvasContexts.join(', ') || 'no context'}) · ${r.virtualScroll ? 'scroll did not move the page (virtual scroll or locked)' : 'native scroll'}`,
      `- **Libraries seen:** ${Object.entries(r.libs).map(([k, v]) => `${k} (${v.join(', ')})`).join('; ') || 'none found'}`,
      `- **Rendered with:** ${r.engine} · ${r.renderer}${r.hidden ? ` · preloader forced off: \`${r.hidden}\`` : ''}`,
      '- Frames: \`01-hero\` first screen, \`01b\` 0.9 s later (idle motion), \`01c\` after moving the cursor across the hero (reaction); the rest are scroll positions. One frame at a time: easing and feel need your eyes.', '',
      ...r.shots.map((s) => `![${r.slug} ${s}](${r.slug}/${s})`), '');
  }
  return L.join('\n') + '\n';
}

async function main() {
  const a = process.argv.slice(2);
  const flag = (k, d) => { const i = a.indexOf('--' + k); return i >= 0 && a[i + 1] ? a[i + 1] : d; };
  const urls = a.filter((x, i) => /^https?:\/\//i.test(x) && !['--out', '--shots', '--wait', '--hide'].includes(a[i - 1]));
  const out = flag('out');
  if (!urls.length || !out) { console.error('usage: node ref-capture.mjs <url> [<url> ...] --out <dir> [--shots 6] [--wait 6000] [--hide <css selector>]'); process.exit(2); }
  let browser;
  try {
    const g = await launchGpu(await loadPlaywright());
    browser = g.browser;
    const results = [];
    for (const u of urls) {
      const r = await Promise.race([capture(browser, u, resolve(out), { shots: Number(flag('shots', 6)), waitMs: Number(flag('wait', 6000)), engine: g.engine, hide: flag('hide', '') }), wait(180000).then(() => ({ url: u, slug: slugOf(u), shots: [], error: 'timeout after 180s' }))]);
      results.push(r);
      console.error(`${r.slug}: ${r.tier || 'ERROR ' + r.error}${r.pageTransitions?.length ? ' · transitions: ' + r.pageTransitions.join(',') : ''}`);
    }
    // rebuild the index from every site folder in --out, so separate runs add to the pack instead of replacing it
    const all = readdirSync(resolve(out), { withFileTypes: true }).filter((d) => d.isDirectory() && existsSync(join(resolve(out), d.name, 'tokens.json')))
      .map((d) => JSON.parse(readFileSync(join(resolve(out), d.name, 'tokens.json'), 'utf8'))).sort((a, b) => a.slug.localeCompare(b.slug));
    writeFileSync(join(resolve(out), 'INDEX.md'), indexNote(all, new Date().toISOString().slice(0, 10)));
  } catch (e) { console.error('ref-capture error: ' + e.message); process.exitCode = 2; } finally { if (browser) await browser.close(); }
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) main();
