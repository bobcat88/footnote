// The stream: the company's data flows in from the left, funnels into the glass lens and settles into the "1" of the Footnote lens: it comes into focus.
// Draws only while the canvas is on screen, the tab is visible and the visitor has not asked for reduced motion (then it paints one still frame).
export function initStream(c, motion) {
  const x = c.getContext("2d");
  let W, H, k, cx, cy, R, flow = [], rings = [], raf = 0, onScreen = true;

  // One dot per sampled point of the glyph, drawn from an offscreen canvas in the same mono, semibold.
  function glyphDots() {
    const g = document.createElement("canvas"), S = 160; g.width = g.height = S;
    const q = g.getContext("2d", { willReadFrequently: true });
    q.fillStyle = "#fff"; q.textAlign = "center"; q.textBaseline = "middle";
    q.font = "600 " + S * .95 + 'px "JetBrains Mono", ui-monospace, monospace';
    q.fillText("1", S / 2, S / 2 + S * .04);
    const d = q.getImageData(0, 0, S, S).data, pts = [];
    for (let y = 0; y < S; y += 3) for (let i = 0; i < S; i += 3) if (d[(y * S + i) * 4 + 3] > 128) pts.push([(i - S / 2) / S, (y - S / 2) / S]);
    return pts.map(p => ({ tx: p[0], ty: p[1], u: Math.random(), v: Math.random() * Math.PI * 2, w: Math.random() * Math.PI * 2, f: .6 + Math.random() }));
  }

  function layout() {
    k = Math.min(devicePixelRatio || 1, 2);
    W = c.clientWidth; H = c.clientHeight; c.width = W * k; c.height = H * k;
    const narrow = W < 1000;
    cx = narrow ? W * 0.5 : W * 0.74; cy = narrow ? H * 0.91 : H * 0.5; R = Math.min(narrow ? 120 : 190, W * 0.2);
    flow = Array.from({ length: Math.round(W * 0.55) }, () => ({ p: Math.random(), y: Math.random() * 2 - 1, s: .5 + Math.random() }));
    rings = glyphDots();
    if (!motion.matches) return; // moving: the loop repaints; still: paint once now
    frame(0);
  }

  function frame(t) {
    raf = 0;
    const reduce = motion.matches;
    x.setTransform(k, 0, 0, k, 0, 0); x.clearRect(0, 0, W, H);
    const T = reduce ? 0 : t;
    for (const d of flow) {
      const p = (d.p + T * 0.000045 * d.s) % 1;          // 0 at the left edge, 1 at the lens
      const px = p * (cx - R * 0.9);
      const spread = (1 - p * p) * H * 0.42 + R * 0.85 * p * p;   // wide on the left, funnel into the lens
      const py = cy + d.y * spread + Math.sin(p * 7 + d.y * 4 + T * 0.0006) * 12 * (1 - p);
      x.fillStyle = `rgba(95,211,195,${0.12 + 0.5 * p})`;
      x.fillRect(px, py, 1.8, 1.8);
    }
    // Scripted loop inside the lens: a mess of dots that gathers into the "1", holds, then lets go again.
    const L = 12000, ph = reduce ? 5600 : T % L, ease = u => u < 0 ? 0 : u > 1 ? 1 : u * u * (3 - 2 * u), size = R * 1.45;
    for (const g of rings) {
      const lag = g.u * 0.3;                             // scatter, so they arrive like a swarm rather than a sheet
      const e = ease((ph - 2000) / 3000 - lag) * (1 - ease((ph - 9800) / 1600 - lag * 0.5)), wob = 1 - e;
      const ra = Math.sqrt(g.u) * R * .92, ca = g.v + T * 0.0003 * g.f;
      const chx = Math.cos(ca) * ra + Math.sin(T * 0.0011 * g.f + g.w) * R * .12 * wob, chy = Math.sin(ca) * ra + Math.cos(T * 0.0009 * g.f + g.v) * R * .12 * wob;
      const ox = g.tx * size + Math.sin(T * 0.002 * g.f + g.w) * .6 * e, oy = g.ty * size;     // held glyph breathes slightly
      x.fillStyle = `rgba(${150 + 60 * e | 0},236,${224 + 24 * e | 0},${0.35 + 0.6 * e})`;
      x.fillRect(cx + chx + (ox - chx) * e - 1.1, cy + chy + (oy - chy) * e - 1.1, 2.2, 2.2);
    }
    const hold = ease((ph - 5200) / 1200) * (1 - ease((ph - 9600) / 1200));   // a soft glow once it reads as one
    if (hold > 0) { const gl = x.createRadialGradient(cx, cy, 0, cx, cy, R * .7); gl.addColorStop(0, `rgba(150,236,224,${.16 * hold})`); gl.addColorStop(1, "rgba(95,211,195,0)"); x.fillStyle = gl; x.fillRect(cx - R, cy - R, 2 * R, 2 * R); }
    for (const d of flow) {                             // and on, past the lens, slower and calmer
      const p = (d.p * 0.6 + T * 0.00002 * d.s) % 1;
      const px = cx + R * 0.95 + p * (W - cx - R);
      if (px > W) continue;
      x.fillStyle = `rgba(95,211,195,${0.28 * (1 - p)})`;
      x.fillRect(px, cy + d.y * R * 0.8 * (1 + p * 0.6), 1.6, 1.6);
    }
    schedule();
  }

  function schedule() { if (!raf && onScreen && !document.hidden && !motion.matches) raf = requestAnimationFrame(frame); }

  new IntersectionObserver(([e]) => { onScreen = e.isIntersecting; if (onScreen) schedule(); else { cancelAnimationFrame(raf); raf = 0; } }).observe(c);
  document.addEventListener("visibilitychange", schedule);
  motion.addEventListener("change", () => motion.matches ? frame(0) : schedule());
  addEventListener("resize", layout);
  // The glyph is sampled from the web font, so redo the layout once it has loaded (measured: waiting for it before the first layout was worse for blocking time).
  layout(); schedule();
  document.fonts.load('600 160px "JetBrains Mono"', "1").then(layout, () => {});
}
