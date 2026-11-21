// Apple extras. AP1 Materials: backdrop-filter glass needs a Reduce Transparency override (HIG Materials + Accessibility).
// Same-origin and inline stylesheets only.
// AP1 Materials: every rule using backdrop-filter (glass) needs an opaque override for Reduce Transparency.
// Source: HIG Materials + Accessibility (Reduce Transparency). Reads same-origin and inline stylesheets only.
export async function glassFallback(page) {
  const msgs = await page.evaluate(() => {
    const glass = new Set(), fixed = new Set(), unreadable = [];
    // shortcut: splits on every comma, so :is(a, b) lists get split too; enough for plain selector lists
    const sels = (t) => t.split(',').map((x) => x.trim()).filter(Boolean);
    const walk = (rules, inReduced) => { for (const r of rules) {
      if (r.media) walk(r.cssRules, inReduced || /prefers-reduced-transparency:\s*reduce/.test(r.media.mediaText));
      else if (r.style && r.selectorText) { // style rules also carry cssRules (CSS nesting), so test style first
        const bf = r.style.getPropertyValue('backdrop-filter') || r.style.getPropertyValue('-webkit-backdrop-filter');
        if (bf && bf !== 'none') sels(r.selectorText).forEach((x) => glass.add(x));
        if (inReduced && (bf === 'none' || r.style.background || r.style.backgroundColor)) sels(r.selectorText).forEach((x) => fixed.add(x));
      } } };
    for (const s of document.styleSheets) { try { walk(s.cssRules, false); } catch { unreadable.push(s.href); } }
    const inline = [...document.querySelectorAll('[style*="backdrop-filter"]')].length;
    return { missing: [...glass].filter((x) => !fixed.has(x)), inline, unreadable: unreadable.length };
  });
  const out = [];
  if (msgs.missing.length) out.push({ id: 'AP1-glass-fallback', severity: 'FAIL', source: 'HIG Materials / Reduce Transparency', msg: `no prefers-reduced-transparency override for: ${msgs.missing.slice(0, 5).join(', ')}` });
  if (msgs.inline) out.push({ id: 'AP1-glass-fallback', severity: 'WARN', source: 'HIG Materials', msg: `${msgs.inline} elements use inline backdrop-filter: move to CSS so a reduced-transparency override is possible` });
  if (msgs.unreadable) out.push({ id: 'AP1-glass-fallback', severity: 'WARN', source: 'script limit', msg: `${msgs.unreadable} cross-origin stylesheets could not be read` });
  return out;
}

