// Material 3 extras: M1-M4 compare computed styles to the profile tokens (profiles/material.json).
export async function tokenChecks(page, { tokens: T }) {
  const r = await page.evaluate((T) => {
    const out = [], name = (e) => e.tagName.toLowerCase() + (typeof e.className === 'string' && e.className.trim() ? '.' + e.className.trim().split(/\s+/)[0] : '');
    const near = (v, list, tol = 0.5) => list.some((x) => Math.abs(x - v) <= tol);
    const bad = { shape: new Set(), type: new Set(), duration: new Set(), easing: new Set(), disabled: [] };
    for (const e of document.querySelectorAll('button,input:not([type=hidden]),select,textarea,[role=button],[class*=card],[class*=chip],dialog')) {
      const r = e.getBoundingClientRect(); if (!r.width) continue;
      const c = getComputedStyle(e), rad = parseFloat(c.borderTopLeftRadius) || 0;
      if (rad && rad < r.height / 2 - 1 && !near(rad, T.shapeRadiiPx)) bad.shape.add(`${name(e)} ${rad}px`);
      for (const d of c.transitionDuration.split(',')) { const ms = parseFloat(d) * (d.includes('ms') ? 1 : 1000); if (ms && !near(ms, T.durationsMs, 1)) bad.duration.add(`${name(e)} ${Math.round(ms)}ms`); }
      if (parseFloat(c.transitionDuration)) for (const f of c.transitionTimingFunction.split(/,\s*(?=cubic|linear|ease|steps)/)) if (!T.easings.includes(f.trim())) bad.easing.add(`${name(e)} ${f.trim()}`);
      if (e.disabled && Math.abs(parseFloat(c.opacity) * (1) - T.disabled.contentOpacity) > 0.02 && !/rgba\(.*0\.38\)/.test(c.color)) bad.disabled.push(name(e));
    }
    for (const e of document.querySelectorAll('body *')) {
      if (![...e.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim())) continue;
      const px = parseFloat(getComputedStyle(e).fontSize); if (!near(px, T.typeSizesPx)) bad.type.add(`${px}px`);
    }
    return Object.fromEntries(Object.entries(bad).map(([k, v]) => [k, [...v]]));
  }, T);
  const f = [], w = (id, source, msg) => f.push({ id, severity: 'WARN', source, msg });
  if (r.shape.length) w('M1-shape', 'M3 shape scale', `radii off the scale ${T.shapeRadiiPx.join('/')}: ${r.shape.slice(0, 4).join(', ')}`);
  if (r.type.length) w('M2-type', 'M3 type scale', `font sizes off the scale ${T.typeSizesPx.join('/')}: ${r.type.slice(0, 6).join(', ')}`);
  if (r.duration.length) w('M3-duration', 'M3 motion durations', `transition durations off the token set: ${r.duration.slice(0, 4).join(', ')}`);
  if (r.easing.length) w('M3-easing', 'M3 motion easings', `easing off the token set: ${r.easing.slice(0, 3).join(', ')}`);
  if (r.disabled.length) w('M4-disabled', 'M3 disabled 0.38', `disabled controls not at 0.38 content opacity: ${r.disabled.slice(0, 4).join(', ')}`);
  return f;
}
