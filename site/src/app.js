const motion = matchMedia("(prefers-reduced-motion: reduce)");

const canvas = document.getElementById("stream");
// The decorative canvas starts once the page has loaded and the browser is idle, so text, fonts and layout get the main thread first.
if (canvas) {
  const start = () => import("./stream.js").then(m => { m.initStream(canvas, motion); canvas.classList.add("on"); });
  const idle = () => (window.requestIdleCallback || setTimeout)(start, { timeout: 1500 });
  document.readyState === "complete" ? idle() : addEventListener("load", idle, { once: true });
}

// Reveal on scroll, staggered inside each group. Skipped entirely under reduced motion, so nothing is ever hidden.
if (!motion.matches && "IntersectionObserver" in window) {
  const io = new IntersectionObserver(es => es.forEach(e => { if (e.isIntersecting) { e.target.classList.add("in"); io.unobserve(e.target); } }), { threshold: .15 });
  document.querySelectorAll(".cards,.intro,.blocks,.list").forEach(g => [...g.children].forEach((n, i) => { n.classList.add("rv"); n.style.setProperty("--i", i % 3); io.observe(n); }));
}

const live = document.getElementById("live");
document.querySelectorAll(".copy").forEach(b => b.addEventListener("click", () => {
  const code = b.previousElementSibling;
  const done = msg => { b.textContent = msg === "Copied" ? "Copied" : "Select and copy"; live.textContent = msg; setTimeout(() => { b.textContent = "Copy"; live.textContent = ""; }, 1600); };
  navigator.clipboard.writeText(code.textContent).then(() => done("Copied"))
    .catch(() => { const r = document.createRange(); r.selectNodeContents(code); const s = getSelection(); s.removeAllRanges(); s.addRange(r); done("Copy failed, text selected"); });
}));
