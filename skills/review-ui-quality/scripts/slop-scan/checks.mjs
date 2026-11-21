// In-page half of slop-scan. Playwright serialises this function, so it must stay self-contained:
// no imports, no closures over module scope. One entry point, many modes, shared helpers.
export function inPage(arg) {
  const W = document.documentElement.clientWidth;
  const P = (e) => {
    const parts = [];
    for (let n = e; n && n.nodeType === 1 && n !== document.documentElement; n = n.parentElement) {
      const same = [...n.parentElement.children].filter((c) => c.tagName === n.tagName);
      parts.unshift(n.tagName.toLowerCase() + (n.id ? '#' + n.id : '') + (same.length > 1 ? ':' + (same.indexOf(n) + 1) : ''));
    }
    return parts.join('>');
  };
  const shown = (e) => { const cs = getComputedStyle(e); return e.getClientRects().length > 0 && cs.display !== 'none' && cs.visibility !== 'hidden'; };
  const ownText = (e) => [...e.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim().length >= 3);
  const FIELDS = 'input:not([type=hidden]):not([type=checkbox]):not([type=radio]):not([type=submit]):not([type=button]),select,textarea';
  const fields = () => [...document.querySelectorAll(FIELDS)].filter(shown).slice(0, 20);
  const outermost = (list) => list.filter((e) => !list.some((o) => o !== e && o.contains(e)));
  const jsCands = () => outermost([...document.querySelectorAll('[aria-roledescription="carousel" i],[class*=carousel i],[class*=slider i],[class*=ticker i],[class*=rotator i],marquee')].filter(shown)).slice(0, 6);
  const cssCands = () => [...document.querySelectorAll('body *')].filter((e) => {
    const cs = getComputedStyle(e);
    return cs.animationName !== 'none' && cs.animationIterationCount.split(',').some((x) => x.trim() === 'infinite') &&
      e.innerText && e.innerText.trim().length >= 12 && e.getBoundingClientRect().width >= 200;
  }).slice(0, 6);
  const center = (e) => { e.scrollIntoView({ block: 'center' }); const r = e.getBoundingClientRect(); return { x: r.left + r.width / 2, y: r.top + r.height / 2 }; };
  const hue = (rgb) => {
    const m = /rgba?\((\d+),\s*(\d+),\s*(\d+)/.exec(rgb); if (!m) return null;
    const [r, g, b] = [m[1], m[2], m[3]].map((v) => v / 255);
    const mx = Math.max(r, g, b), mn = Math.min(r, g, b), d = mx - mn;
    if (d < 0.25 || mx < 0.3) return null;
    const h = mx === r ? ((g - b) / d + 6) % 6 : mx === g ? (b - r) / d + 2 : (r - g) / d + 4;
    return h * 60;
  };

  switch (arg.mode) {
    case 'collect': {
      const out = { sl02: [], sl03: [], sl05: [], sl32: [], sl33: [], jsCands: [], cssCands: [], fieldCount: fields().length };
      // SL-02: transition: all with a real duration (the default computed value is "all 0s", which is fine)
      for (const e of document.querySelectorAll('*')) {
        const cs = getComputedStyle(e);
        if (cs.transitionProperty.split(',').some((x) => x.trim() === 'all') && cs.transitionDuration.split(',').some((x) => parseFloat(x) > 0)) out.sl02.push({ el: P(e), detail: 'transition: ' + cs.transition });
      }
      // SL-03: height mismatch against an adjacent button, and "disabled" by class or opacity without the attribute
      for (const f of fields()) {
        for (const s of [f.previousElementSibling, f.nextElementSibling]) {
          if (!s || !s.matches('button,[type=submit],a[role=button]') || !shown(s)) continue;
          const a = f.getBoundingClientRect(), b = s.getBoundingClientRect();
          if (Math.abs(a.height - b.height) > 1 && a.top < b.bottom && b.top < a.bottom) out.sl03.push({ el: P(f), detail: `input height ${a.height.toFixed(0)}px differs from adjacent button ${b.height.toFixed(0)}px` });
        }
      }
      for (const e of document.querySelectorAll('button,a,input,select,[role=button]')) {
        if (!shown(e)) continue;
        // class only: opacity + pointer-events: none is also a normal mid-fade state (navs, sliders), so it is not a signal
        if (typeof e.className === 'string' && /disabled/i.test(e.className) && !e.disabled && e.getAttribute('aria-disabled') !== 'true') out.sl03.push({ el: P(e), detail: 'disabled by class only: no disabled attribute and no aria-disabled' });
      }
      // SL-05: placeholder names and startup cliches in rendered text and placeholders
      const text = document.body.innerText + '\n' + [...document.querySelectorAll('[placeholder]')].map((e) => e.getAttribute('placeholder')).join('\n');
      const hits = new Set(text.match(/\b(?:Jane|John)\s+Doe\b|\bAcme\b|\bNexus\b|\bFoo\s?Bar\b|\bTest User\b/gi) || []);
      out.sl05 = [...hits];
      // SL-04 candidates
      out.jsCands = jsCands().map((c) => ({
        el: P(c),
        hasPause: [...c.querySelectorAll('button,[role=button]'), ...(c.parentElement ? c.parentElement.querySelectorAll('button,[role=button]') : [])]
          .some((b) => /pause|stop/i.test((b.getAttribute('aria-label') || '') + ' ' + b.textContent)),
      }));
      out.cssCands = cssCands().map((c) => ({ el: P(c) }));
      if (arg.web) {
        // SL-32: text gutters at the viewport width, per top-level block
        let kids = [...document.body.children].filter(shown);
        for (let d = 0; d < 3 && kids.length === 1 && !ownText(kids[0]); d++) {
          const next = [...kids[0].children].filter(shown);
          if (!next.length) break;
          kids = next;
        }
        for (const blk of kids) {
          let l = Infinity, r = -Infinity;
          const walker = document.createTreeWalker(blk, NodeFilter.SHOW_TEXT);
          for (let n; (n = walker.nextNode());) {
            if (!n.textContent.trim() || !n.parentElement || !shown(n.parentElement)) continue;
            // skip screen-reader-only text (clipped to a point) and text inside a real horizontal scroller (carousel rail)
            let sr = false;
            for (let a = n.parentElement; a && a !== document.body; a = a.parentElement) {
              const s = getComputedStyle(a), b = a.getBoundingClientRect();
              if (s.clip !== 'auto' || s.clipPath.startsWith('inset(50%') || ((b.width <= 2 || b.height <= 2) && s.overflow !== 'visible') || (s.overflowX !== 'visible' && a.scrollWidth > a.clientWidth + 1)) { sr = true; break; }
            }
            if (sr) continue;
            const rg = document.createRange(); rg.selectNodeContents(n);
            // text outside the viewport belongs to a horizontal scroller, not to the container
            for (const q of rg.getClientRects()) { if (q.width > 0 && q.left >= -1 && q.right <= W + 1) { l = Math.min(l, q.left); r = Math.max(r, q.right); } }
          }
          if (l !== Infinity && Math.min(l, W - r) < 120) out.sl32.push({ el: P(blk), detail: `text gutters ${Math.round(l)}px left, ${Math.round(W - r)}px right at ${W}px wide (need 120px or more)` });
        }
        // SL-33: three small round dots coloured red, yellow and green inside one parent (a hand-drawn window bar)
        for (const p of document.querySelectorAll('body *')) {
          const hues = [...p.children].filter((c) => { const r = c.getBoundingClientRect(); const cs = getComputedStyle(c); const rad = parseFloat(cs.borderTopLeftRadius) || 0; return r.width > 0 && r.width <= 24 && r.height <= 24 && (cs.borderTopLeftRadius.includes('%') ? rad >= 40 : rad >= r.width * 0.4); }).map((c) => hue(getComputedStyle(c).backgroundColor)).filter((h) => h !== null);
          const has = (a, b) => hues.some((h) => h >= a && h <= b);
          if ((has(0, 20) || has(340, 360)) && has(35, 65) && has(95, 150)) out.sl33.push({ el: P(p), detail: 'red, yellow and green dots in one row (traffic lights)' });
        }
      }
      return out;
    }
    case 'measure': {
      const e = fields()[arg.i]; const r = e.getBoundingClientRect(); const cs = getComputedStyle(e);
      return { el: P(e), w: r.width, h: r.height, bw: [cs.borderTopWidth, cs.borderRightWidth, cs.borderBottomWidth, cs.borderLeftWidth].map(parseFloat) };
    }
    case 'focus': fields()[arg.i].focus(); return null;
    case 'blur': fields()[arg.i].blur(); return null;
    case 'sig': {
      const e = jsCands()[arg.i];
      return e.innerText + '|' + [...e.querySelectorAll('*')].slice(0, 80).map((c) => c.className + getComputedStyle(c).transform).join(',') + '|' + e.scrollLeft;
    }
    case 'rect': return center(arg.kind === 'js' ? jsCands()[arg.i] : (cssCands()[arg.i].parentElement || cssCands()[arg.i]));
    case 'anim': return getComputedStyle(cssCands()[arg.i]).animationPlayState;
    case 'vis': {
      const out = [];
      for (const e of document.querySelectorAll('body *')) {
        if (!ownText(e) || !shown(e)) continue;
        let op = 1; for (let n = e; n; n = n.parentElement) op *= parseFloat(getComputedStyle(n).opacity);
        out.push({ p: P(e), v: op > 0.05 });
        if (out.length >= 3000) break;
      }
      return out;
    }
    case 'page': return { text: document.body.innerText, ipc: window.__slopIpcCalls || 0 };
  }
  return null;
}
