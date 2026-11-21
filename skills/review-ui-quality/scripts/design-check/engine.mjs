#!/usr/bin/env node
// design-check: measurable parts of general UI craft (design-language neutral), run in a real browser.
// Only checks what has a source or is a declared project decision (see review-ui-quality SKILL.md).
// usage: node engine.mjs <url|file> [--grid 4] [--min-target 44] [--cards "article,.card"] [--out report.json]
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { writeFileSync } from 'node:fs';
import { loadPlaywright, launch } from '../slop-scan/slop-scan.mjs';

const FOCUSABLE = 'a[href],button,input:not([type=hidden]),select,textarea,[tabindex]:not([tabindex="-1"])';

// Runs inside the page. Returns findings [{id, severity, source, msg}].
export function inPage({ grid, minTarget, touchInput, cards, focusable }) {
  const out = [];
  const add = (id, severity, source, msg) => out.push({ id, severity, source, msg });
  const vis = (e) => { const r = e.getBoundingClientRect(), s = getComputedStyle(e); return r.width > 0 && r.height > 0 && s.visibility !== 'hidden' && s.display !== 'none'; };
  const name = (e) => e.tagName.toLowerCase() + (e.id ? '#' + e.id : '') + (typeof e.className === 'string' && e.className.trim() ? '.' + e.className.trim().split(/\s+/)[0] : '');

  // C1 hit targets. WCAG 2.2 SC 2.5.8 AA: 24x24 CSS px, unless inline in text or spaced (a 24 px circle centred on the target touches no other target).
  // Comfort size (minTarget) is checked only for touch input; Apple HIG uses 44 pt, Material 48 dp.
  const targets = [...document.querySelectorAll(focusable + ',[role=button],[role=link]')].filter(vis).map((e) => ({ e, r: e.getBoundingClientRect() }));
  const inline = (e) => e.tagName === 'A' && e.closest('p,li,dd,td,blockquote,figcaption') && getComputedStyle(e).display === 'inline';
  const small = (t) => Math.min(t.r.width, t.r.height) < 24;
  const circle = (t) => ({ x: t.r.left + t.r.width / 2, y: t.r.top + t.r.height / 2 });
  const touches = (a, b) => { const c = circle(a);
    if (small(b)) { const d = circle(b); return Math.hypot(c.x - d.x, c.y - d.y) < 24; }
    const nx = Math.max(b.r.left, Math.min(c.x, b.r.right)), ny = Math.max(b.r.top, Math.min(c.y, b.r.bottom)); return Math.hypot(c.x - nx, c.y - ny) < 12; };
  for (const t of targets) {
    if (inline(t.e)) continue;
    const m = Math.min(t.r.width, t.r.height), dim = `${Math.round(t.r.width)}x${Math.round(t.r.height)} px`;
    if (m < 24) { if (targets.some((o) => o !== t && !o.e.contains(t.e) && !t.e.contains(o.e) && touches(t, o))) add('C1-target', 'FAIL', 'WCAG 2.2 SC 2.5.8', `${name(t.e)} is ${dim}, under 24 and too close to another target`); }
    else if (touchInput && m < minTarget) add('C1-target', 'WARN', 'touch comfort size (Apple HIG 44 pt, Material 48 dp)', `${name(t.e)} is ${dim}, under ${minTarget}`);
  }

  // A4 contrast. HIG recommends 4.5:1 for text (3:1 for large). Only where the background is a plain colour.
  const parse = (c) => { const m = c.match(/rgba?\(([^)]+)\)/); if (!m) return null; const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number); return { r: p[0], g: p[1], b: p[2], a: p[3] ?? 1 }; };
  const lum = ({ r, g, b }) => { const f = (v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }; return 0.2126 * f(r) + 0.7152 * f(g) + 0.0722 * f(b); };
  const bgOf = (e) => {
    for (let n = e; n; n = n.parentElement) {
      const s = getComputedStyle(n);
      if (s.backgroundImage !== 'none' || (s.backdropFilter && s.backdropFilter !== 'none')) return 'complex';
      const c = parse(s.backgroundColor);
      if (c && c.a === 1) return c;
      if (c && c.a > 0) return 'complex';
    }
    return { r: 255, g: 255, b: 255, a: 1 };
  };
  let unverified = 0;
  for (const e of document.querySelectorAll('body *')) {
    if (!vis(e) || ![...e.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim())) continue;
    const s = getComputedStyle(e), fg = parse(s.color), bg = bgOf(e);
    if (bg === 'complex' || !fg || fg.a < 1) { unverified++; continue; }
    const L1 = lum(fg), L2 = lum(bg), ratio = (Math.max(L1, L2) + 0.05) / (Math.min(L1, L2) + 0.05);
    const px = parseFloat(s.fontSize), large = px >= 24 || (px >= 18.66 && Number(s.fontWeight) >= 700);
    if (ratio < (large ? 3 : 4.5)) add('C4-contrast', 'FAIL', 'WCAG 1.4.3 (Apple HIG also recommends 4.5:1)', `${name(e)} text ${ratio.toFixed(2)}:1, needs ${large ? 3 : 4.5}`);
  }
  if (unverified) add('C4-contrast', 'WARN', 'script limit', `${unverified} text nodes over images, gradients or glass were not measured: check them by eye in light and dark`);

  // A5/A6 layout inside each card: one left anchor, spacing on the project grid. Source: layout rules, project decision.
  for (const card of document.querySelectorAll(cards)) {
    if (!vis(card)) continue;
    const leaves = [...card.querySelectorAll('*')].filter((e) => vis(e) && !e.children.length && e.textContent.trim());
    const rows = [];
    for (const e of leaves) {
      const rg = document.createRange(); rg.selectNodeContents(e); // text box, so centred text shows its real left edge
      const eb = e.getBoundingClientRect(), r = { top: eb.top, bottom: eb.bottom, left: rg.getBoundingClientRect().left, height: eb.height };
      const row = rows.find((x) => Math.abs(x.top - r.top) < r.height / 2);
      if (row) { row.left = Math.min(row.left, r.left); row.top = Math.min(row.top, r.top); row.bottom = Math.max(row.bottom, r.bottom); } else rows.push({ top: r.top, bottom: r.bottom, left: r.left });
    }
    rows.sort((a, b) => a.top - b.top);
    const lefts = new Set(rows.map((r) => Math.round(r.left)));
    if (rows.length >= 3 && lefts.size > 2) add('C5-anchors', 'WARN', 'layout rule: one left anchor', `${name(card)} text rows start at ${lefts.size} different x positions`);
    const gaps = rows.slice(1).map((r, i) => Math.round(r.top - rows[i].bottom)).filter((g) => g > 0);
    const off = gaps.filter((g) => g % grid !== 0);
    if (off.length) add('C6-spacing', 'WARN', `project grid ${grid}px`, `${name(card)} gaps off the ${grid}px grid: ${[...new Set(off)].join(', ')}`);
    if (new Set(gaps).size > 4) add('C6-spacing', 'WARN', 'layout rule: few spacing steps', `${name(card)} uses ${new Set(gaps).size} different gaps`);
  }
  // A7-A9 icon + label rows (menus, sidebars): icon sized to the text line, centred on it, same gap and left edge in every row of a list.
  const lists = new Map();
  for (const ic of document.querySelectorAll('svg,i,[class*=icon]')) {
    const ib = ic.getBoundingClientRect(), row = ic.parentElement;
    if (!vis(ic) || ib.width > 48 || !row || ic.closest('button,a,li,[role]') === null && row === document.body) continue;
    const tn = [...row.childNodes].filter((n) => n.nodeType === 3 && n.textContent.trim()).concat([...row.children].filter((c) => c !== ic && !c.contains(ic) && !ic.contains(c) && c.textContent.trim() && !c.querySelector('svg,img')));
    if (!tn.length) continue;
    const rg = document.createRange(); rg.selectNodeContents(tn[0].nodeType === 3 ? tn[0] : tn[0]);
    const tb = rg.getBoundingClientRect(), lh = parseFloat(getComputedStyle(row).lineHeight) || tb.height;
    if (tb.left < ib.right - 1) continue; // label must follow the icon
    const li = { name: name(row), size: ib.height, dy: Math.abs((ib.top + ib.bottom) / 2 - (tb.top + tb.bottom) / 2), gap: Math.round(tb.left - ib.right), left: Math.round(ib.left), lh };
    const key = row.parentElement; (lists.get(key) ?? lists.set(key, []).get(key)).push(li);
  }
  for (const rows of lists.values()) {
    if (rows.length < 2) continue;
    const q = rows[0].name + ' list';
    if (new Set(rows.map((r) => Math.round(r.size))).size > 1) add('C7-icon-size', 'WARN', 'layout rule: one icon size per list', `${q} icon sizes ${[...new Set(rows.map((r) => Math.round(r.size)))].join(', ')} px`);
    const bad = rows.filter((r) => Math.abs(r.size - r.lh) > 4 && Math.abs(r.size - r.lh) / r.lh > 0.25);
    if (bad.length) add('C7-icon-size', 'WARN', 'layout rule: icon about as tall as the text line', `${q} icon ${Math.round(bad[0].size)} px vs line height ${Math.round(bad[0].lh)} px`);
    if (rows.some((r) => r.dy > 1.5)) add('C8-icon-centre', 'FAIL', 'layout rule: icon centred on label', `${q} icon off the label centre by ${Math.max(...rows.map((r) => r.dy)).toFixed(1)} px`);
    if (new Set(rows.map((r) => r.gap)).size > 1) add('C9-icon-gap', 'WARN', 'layout rule: one icon-to-label gap', `${q} gaps ${[...new Set(rows.map((r) => r.gap))].join(', ')} px`);
    if (new Set(rows.map((r) => r.left)).size > 1) add('C9-icon-gap', 'WARN', 'layout rule: icons share a left edge', `${q} icon left edges ${[...new Set(rows.map((r) => r.left))].join(', ')}`);
  }
  // C10/C11 sidebars and nav (any language): one leading axis for avatar, icons and labels, left-aligned labels, name above secondary text.
  for (const nav of document.querySelectorAll('nav,aside,[role=navigation]')) {
    if (!vis(nav)) continue;
    const own = (e) => [...e.childNodes].filter((n) => n.nodeType === 3 && n.textContent.trim());
    const parts = [...nav.querySelectorAll('*')].filter((e) => vis(e) && (own(e).length || /^(svg|img)$/i.test(e.tagName)) && !e.parentElement.closest('svg')).map((e) => {
      const rg = document.createRange();
      if (own(e).length) { rg.setStartBefore(own(e)[0]); rg.setEndAfter(own(e).at(-1)); } else rg.selectNode(e);
      return rg.getBoundingClientRect();
    });
    // a label that follows an icon or avatar on the same line is not a new leading edge
    const leads = parts.filter((p) => !parts.some((o) => o !== p && o.right <= p.left + 1 && o.top < p.bottom && o.bottom > p.top));
    const rows = [];
    for (const r of leads) { const mid = (r.top + r.bottom) / 2, row = rows.find((x) => Math.abs(x.mid - mid) < 6); if (row) row.left = Math.min(row.left, r.left); else rows.push({ mid, left: r.left }); }
    const lefts = rows.map((r) => Math.round(r.left));
    if (rows.length >= 3 && Math.max(...lefts) - Math.min(...lefts) > 2) add('C10-nav-axis', 'WARN', 'layout rule: one leading axis in a sidebar', `${name(nav)} rows start at ${[...new Set(lefts)].join(', ')} px: avatar, icons and labels should share one left edge`);
    const centred = [...nav.querySelectorAll('a,button,li')].filter((e) => vis(e) && e.textContent.trim() && getComputedStyle(e).textAlign === 'center' && e.getBoundingClientRect().width > 100);
    if (centred.length >= 3) add('C10-nav-axis', 'WARN', 'layout rule: left-align nav labels', `${name(nav)} has ${centred.length} centred nav items: left-align them on one axis`);
    for (const img of nav.querySelectorAll('img')) {
      const ts = [...img.parentElement.querySelectorAll('*')].filter((e) => !e.children.length && e.textContent.trim() && vis(e));
      if (ts.length < 2) continue;
      const [a, b] = ts.map((e) => { const c = getComputedStyle(e); return { px: parseFloat(c.fontSize), w: Number(c.fontWeight), col: parse(c.color) }; });
      if (!(b.px < a.px || b.w < a.w || (a.col && b.col && lum(b.col) > lum(a.col) + 0.05))) add('C11-hierarchy', 'WARN', 'layout rule: secondary text visibly lighter', `${name(nav)} name and secondary line (email, role) look the same: make the second smaller or lighter`);
    }
  }
  return out;
}


// C12-C14 controls: nothing may look like the browser default, and every control needs designed, animated states.
const CTRL = 'button,input:not([type=hidden]),select,textarea,[role=button],a[class*=btn],a[class*=button],summary';
const LOOK = ['borderTopStyle', 'borderTopWidth', 'borderTopColor', 'borderTopLeftRadius', 'backgroundColor', 'boxShadow', 'paddingTop', 'paddingLeft', 'fontFamily'];
const STATE = ['backgroundColor', 'color', 'borderTopColor', 'boxShadow', 'transform', 'filter', 'opacity', 'outlineStyle', 'outlineColor', 'textDecorationLine'];

export async function controls(page, { max = 40 } = {}) {
  const out = [];
  const items = await page.evaluate(({ CTRL, LOOK }) => {
    const ref = document.createElement('iframe'); ref.style.cssText = 'position:fixed;left:-9999px;width:300px;height:200px'; document.body.append(ref);
    const d = ref.contentDocument, pick = (c) => LOOK.map((k) => c[k]).join('|'), seen = new Set(), res = [];
    for (const e of document.querySelectorAll(CTRL)) {
      const r = e.getBoundingClientRect(), cs = getComputedStyle(e);
      if (!r.width || !r.height || cs.visibility === 'hidden') continue;
      const sig = e.tagName + e.type + e.className + cs.backgroundColor; if (seen.has(sig) || res.length >= 40) continue; seen.add(sig);
      let def = false;
      if (/^(BUTTON|INPUT|SELECT|TEXTAREA)$/.test(e.tagName)) {
        const t = d.createElement(e.tagName); if (e.type) try { t.type = e.type; } catch {} d.body.append(t);
        def = pick(getComputedStyle(t)) === pick(cs);
      }
      const check = /^(checkbox|radio)$/.test(e.type) && cs.appearance === 'auto' && cs.accentColor === 'auto';
      e.setAttribute('data-cc', res.length);
      res.push({ i: res.length, name: e.tagName.toLowerCase() + (e.id ? '#' + e.id : '') + (typeof e.className === 'string' && e.className.trim() ? '.' + e.className.trim().split(/\s+/)[0] : ''), def, nativeCheck: check, cursor: cs.cursor, clickable: /^(BUTTON|A)$/.test(e.tagName) || e.getAttribute('role') === 'button', td: cs.transitionDuration, tp: cs.transitionProperty });
    }
    ref.remove(); return res;
  }, { CTRL, LOOK });
  for (const c of items) {
    if (c.def) out.push({ id: 'C12-default-control', severity: 'FAIL', source: 'design rule: no browser-default controls', msg: `${c.name} looks like the browser default (border, background, radius, padding unchanged)` });
    if (c.nativeCheck) out.push({ id: 'C12-default-control', severity: 'WARN', source: 'design rule: no browser-default controls', msg: `${c.name} is a native checkbox/radio: set accent-color or custom appearance` });
    if (c.clickable && c.cursor !== 'pointer') out.push({ id: 'C12-default-control', severity: 'WARN', source: 'design rule', msg: `${c.name} is clickable but the cursor is ${c.cursor}` });
    const longest = Math.max(...c.td.split(',').map((x) => parseFloat(x) * (x.includes('ms') ? 1 : 1000)));
    if (/^all$|,\s*all\b/.test(c.tp) && longest > 0) out.push({ id: 'C14-transition', severity: 'WARN', source: 'performance: transition only the changing properties', msg: `${c.name} uses transition: all` });
    if (longest > 400) out.push({ id: 'C14-transition', severity: 'WARN', source: 'design rule: UI transitions 400 ms or less', msg: `${c.name} transition ${Math.round(longest)} ms` });
  }
  // forced pseudo states through CDP, so hover and pressed can be compared without a real pointer
  const cdp = await page.context().newCDPSession(page);
  await cdp.send('DOM.enable'); await cdp.send('CSS.enable');
  const { root } = await cdp.send('DOM.getDocument', { depth: 0 });
  const read = (i) => page.evaluate(([i, STATE]) => { const e = document.querySelector(`[data-cc="${i}"]`); const c = getComputedStyle(e); return STATE.map((k) => c[k]).join('|'); }, [i, STATE]);
  for (const c of items) {
    const { nodeId } = await cdp.send('DOM.querySelector', { nodeId: root.nodeId, selector: `[data-cc="${c.i}"]` });
    if (!nodeId) continue;
    const base = await read(c.i), res = {};
    for (const st of ['hover', 'active']) {
      await cdp.send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses: [st] });
      await page.waitForTimeout(450); res[st] = (await read(c.i)) !== base;
      await cdp.send('CSS.forcePseudoState', { nodeId, forcedPseudoClasses: [] });
    }
    if (!res.hover) out.push({ id: 'C13-states', severity: 'WARN', source: 'design rule: every control reacts to hover', msg: `${c.name} has no visible hover state` });
    if (!res.active) out.push({ id: 'C13-states', severity: 'WARN', source: 'design rule: every control reacts to press', msg: `${c.name} has no visible pressed state` });
    if (res.hover && /^0s/.test(c.td)) out.push({ id: 'C14-transition', severity: 'WARN', source: 'design rule: states animate, they do not jump', msg: `${c.name} changes on hover with no transition` });
  }
  await cdp.detach();
  return out;
}

// extra(page) lets a design-language specialist (e.g. apple-check) add findings in the same browser session.
export async function check(browser, target, { grid = 4, minTarget = 44, input = 'pointer', cards = 'article,[class*=card],li', extra } = {}) {
  const ctx = await browser.newContext({ viewport: { width: 1920, height: 1080 } });
  const page = await ctx.newPage();
  await page.goto(/^(https?|file):/.test(target) ? target : pathToFileURL(resolve(target)).href);
  await page.waitForTimeout(500);
  const findings = await page.evaluate(inPage, { grid, minTarget, touchInput: input === 'touch', cards, focusable: FOCUSABLE });

  // A3 keyboard focus must change something visible: compare each element's focus ring signature before and after Tab.
  const sigOf = (sel) => page.evaluate((s) => { const e = document.activeElement; const t = s === 'active' ? e : document.querySelectorAll(s)[0]; if (!t) return null; const c = getComputedStyle(t); return [c.outlineStyle, c.outlineWidth, c.boxShadow, c.borderColor, c.backgroundColor].join('|'); }, sel);
  const count = await page.evaluate((s) => document.querySelectorAll(s).length, FOCUSABLE);
  for (let i = 0; i < Math.min(count, 30); i++) {
    const info = await page.evaluate(([s, i]) => { const e = document.querySelectorAll(s)[i]; const c = getComputedStyle(e); return { label: e.tagName + (e.id ? '#' + e.id : '') + ' ' + e.textContent.trim().slice(0, 20), sig: [c.outlineStyle, c.outlineWidth, c.boxShadow, c.borderColor, c.backgroundColor].join('|') }; }, [FOCUSABLE, i]);
    await page.keyboard.press('Tab');
    const after = await sigOf('active');
    if (after !== null && after === info.sig) findings.push({ id: 'C3-focus', severity: 'FAIL', source: 'WCAG 2.4.7', msg: `no visible focus change on ${info.label}` });
  }

  // A2 reduced motion: nothing infinite or long may keep running.
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.waitForTimeout(300);
  const moving = await page.evaluate(() => document.getAnimations().filter((a) => { const t = a.effect.getComputedTiming(); return a.playState === 'running' && (t.iterations === Infinity || t.duration > 500); }).length);
  if (moving) findings.push({ id: 'C2-reduced-motion', severity: 'FAIL', source: 'WCAG 2.3.3', msg: `${moving} animations still run (infinite or over 500 ms) with prefers-reduced-motion` });

  findings.push(...await controls(page));
  if (extra) findings.push(...await extra(page));
  await ctx.close();
  const seen = new Set();
  const uniq = findings.filter((f) => { const k = f.id + f.msg; return seen.has(k) ? false : seen.add(k); });
  return {
    target, grid, input, minTarget, findings: uniq, fail: uniq.filter((f) => f.severity === 'FAIL').length,
    notRun: ['text size at largest accessibility setting', 'screen reader', 'dark mode (run again with the OS theme set)'],
  };
}

async function main() {
  const a = process.argv.slice(2);
  const flag = (k, d) => { const i = a.indexOf('--' + k); return i >= 0 && a[i + 1] ? a[i + 1] : d; };
  const target = a.find((x, i) => !x.startsWith('--') && !a[i - 1]?.startsWith('--'));
  if (!target) { console.error('usage: node engine.mjs <url|file> [--grid 4] [--min-target 44] [--cards "article,.card"] [--out report.json]'); process.exit(2); }
  let browser;
  try {
    browser = await launch(await loadPlaywright());
    const r = await check(browser, target, { grid: Number(flag('grid', 4)), minTarget: Number(flag('min-target', 44)), cards: flag('cards', 'article,[class*=card],li') });
    const json = JSON.stringify(r, null, 2);
    if (flag('out')) writeFileSync(flag('out'), json + '\n'); else console.log(json);
    console.error(`design-check: ${r.fail} FAIL, ${r.findings.length - r.fail} WARN, ${r.notRun.length} NOT RUN`);
    process.exitCode = r.fail ? 1 : 0;
  } catch (e) { console.error('design-check error: ' + e.message); process.exitCode = 2; } finally { if (browser) await browser.close(); }
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) main();
