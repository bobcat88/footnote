// write-modern-css extras (any design language): K1 layout-property animation, K3 tokens, K4 vh, K5 layout shift, K6 frames, K7 WebGL gate.
export const WEBGL_OK = ['tech', 'gaming', 'creative', 'luxury', 'culture', 'product'];
export const WEBGL_NO = ['content', 'commerce', 'docs', 'gov', 'saas'];

export function cssExtras(page, { siteType, perf = false } = {}) {
  return (async () => {
    const r = await page.evaluate(() => {
      const LAYOUT = /^(width|height|top|left|right|bottom|margin(-\w+)?|padding(-\w+)?|font-size|line-height|border-width)$/;
      const o = { layoutAnim: new Set(), colors: new Set(), vars: 0, vh: false, fontNoDisplay: 0, unreadable: 0, imgs: [], webgl: null };
      const walk = (rules) => { for (const r of rules) {
        if (r.type === CSSRule.KEYFRAMES_RULE) { for (const k of r.cssRules) for (const p of k.style) if (LAYOUT.test(p)) o.layoutAnim.add(`@keyframes ${r.name} ${p}`); }
        else if (r.type === CSSRule.FONT_FACE_RULE) { if (!r.style.getPropertyValue('font-display')) o.fontNoDisplay++; }
        else if (r.style && r.selectorText !== undefined) {
          const tp = r.style.getPropertyValue('transition-property') || r.style.getPropertyValue('transition');
          for (const p of tp.split(/[\s,]+/)) if (LAYOUT.test(p)) o.layoutAnim.add(`${r.selectorText} transition ${p}`);
          for (const p of r.style) {
            const v = r.style.getPropertyValue(p);
            if (p.startsWith('--')) o.vars++;
            if (/^(color|background(-color)?|border(-\w+)?-color|fill|stroke)$/.test(p)) for (const m of v.matchAll(/#[0-9a-f]{3,8}\b|rgba?\([^)]*\)|hsla?\([^)]*\)/gi)) o.colors.add(m[0].toLowerCase().replace(/\s/g, ''));
            if (/^(height|min-height)$/.test(p) && /(^|\s)100vh\b/.test(v) && !/d?[sl]?vh/.test(v.replace('100vh', ''))) o.vh = true;
          }
          if (r.cssRules?.length) walk(r.cssRules);
        } else if (r.cssRules) walk(r.cssRules);
      } };
      for (const s of document.styleSheets) { try { walk(s.cssRules); } catch { o.unreadable++; } }
      for (const i of document.querySelectorAll('img')) {
        const rect = i.getBoundingClientRect(), cs = getComputedStyle(i);
        if (!(i.getAttribute('width') && i.getAttribute('height')) && cs.aspectRatio === 'auto') o.imgs.push(i.getAttribute('src') || 'img');
      }
      const scripts = [...document.scripts].map((s) => s.src + ' ' + s.textContent).join('\n');
      const canvases = document.querySelectorAll('canvas').length;
      if (canvases && /webgl|three(\.module|\.min)?\.js|babylon|pixi|\bogl\b|regl/i.test(scripts)) {
        o.webgl = {
          fallback: [...document.querySelectorAll('canvas')].some((c) => c.textContent.trim() || c.getAttribute('aria-label') || c.closest('picture,[data-fallback]')),
          reduced: /prefers-reduced-motion/.test(scripts) || /prefers-reduced-motion/.test([...document.styleSheets].map((s) => { try { return [...s.cssRules].map((r) => r.cssText).join(''); } catch { return ''; } }).join('')),
          pause: /IntersectionObserver|visibilitychange/.test(scripts),
        };
      }
      return { ...o, layoutAnim: [...o.layoutAnim], colors: o.colors.size };
    });
    const f = [], add = (id, severity, source, msg) => f.push({ id, severity, source, msg });
    if (r.layoutAnim.length) add('K1-layout-anim', 'WARN', 'Chrome motion guidance: animate opacity/transform', `animates layout properties (jank, layout shift): ${r.layoutAnim.slice(0, 4).join('; ')}`);
    if (r.colors > 12) add('K3-colors', 'WARN', 'project token discipline', `${r.colors} distinct literal colours in CSS; define tokens and reuse them`);
    else if (!r.vars && r.colors > 6) add('K3-no-tokens', 'WARN', 'project token discipline', `${r.colors} literal colours and no custom properties`);
    if (r.vh) add('K4-vh', 'WARN', 'mobile viewport units', '100vh height without dvh/svh: jumps under mobile browser chrome');
    if (r.fontNoDisplay) add('K5-font-display', 'WARN', 'Core Web Vitals (CLS)', `${r.fontNoDisplay} @font-face without font-display`);
    if (r.imgs.length) add('K5-img-size', 'WARN', 'Core Web Vitals (CLS)', `images without width/height or aspect-ratio: ${r.imgs.slice(0, 3).join(', ')}`);
    if (perf) {
      const p95 = await page.evaluate(() => new Promise((res) => { const d = []; let l = performance.now(), n = 0; const t = (now) => { d.push(now - l); l = now; if (++n < 90) requestAnimationFrame(t); else res(d.slice(1).sort((a, b) => a - b)[Math.floor(d.length * 0.95) - 1]); }; requestAnimationFrame(t); }));
      if (p95 > 33) add('K6-frames', 'WARN', '30 fps floor', `p95 frame time ${Math.round(p95)} ms over 90 idle frames`);
    }
    if (r.webgl) {
      if (WEBGL_NO.includes(siteType)) add('K7-webgl-site-type', 'FAIL', `WebGL gate: not for ${siteType} sites`, `WebGL on a ${siteType} site; use CSS/SVG or a static image`);
      else if (!siteType) add('K7-webgl-site-type', 'WARN', 'WebGL gate', 'WebGL found but --site-type not given; allowed: ' + WEBGL_OK.join(', '));
      if (!r.webgl.fallback) add('K7-webgl-fallback', 'WARN', 'WebGL gate: poster/HTML fallback', 'canvas has no fallback content, aria-label or <picture> poster');
      if (!r.webgl.reduced) add('K7-webgl-reduced', 'WARN', 'WCAG 2.3.3, WebGL gate', 'no prefers-reduced-motion handling found next to WebGL code');
      if (!r.webgl.pause) add('K7-webgl-pause', 'WARN', 'WebGL gate: battery and GPU', 'no IntersectionObserver/visibilitychange pause for the render loop');
    }
    return f;
  })();
}
