// Footnote landing page. No dependencies, no network. Everything works without this file; it only adds polish.
(() => {
  const reduce = matchMedia("(prefers-reduced-motion: reduce)").matches;

  // Hovering or focusing a footnote numeral highlights its note in the margin.
  const notes = new Map([...document.querySelectorAll(".notes p[id]")].map((p) => [p.id, p]));
  document.querySelectorAll("a.fn").forEach((a) => {
    const note = notes.get(a.getAttribute("href").slice(1));
    if (!note) return;
    const on = () => note.classList.add("hl");
    const off = () => note.classList.remove("hl");
    a.addEventListener("mouseenter", on); a.addEventListener("mouseleave", off);
    a.addEventListener("focus", on); a.addEventListener("blur", off);
  });

  // Bars grow once when their block enters the viewport.
  const targets = document.querySelectorAll("[data-reveal]");
  if (reduce || !("IntersectionObserver" in window)) {
    targets.forEach((t) => t.classList.add("in"));
  } else {
    const io = new IntersectionObserver((entries) => {
      entries.forEach((e) => { if (e.isIntersecting) { e.target.classList.add("in"); io.unobserve(e.target); } });
    }, { threshold: 0.1, rootMargin: "0px 0px -5% 0px" });
    targets.forEach((t) => io.observe(t));
  }

  // Copy buttons.
  document.querySelectorAll("pre .copy").forEach((b) => {
    b.addEventListener("click", async () => {
      const text = b.parentElement.querySelector("code").innerText;
      try { await navigator.clipboard.writeText(text); b.textContent = "Copied"; }
      catch { b.textContent = "Select and copy"; }
      setTimeout(() => (b.textContent = "Copy"), 1800);
    });
  });
})();
