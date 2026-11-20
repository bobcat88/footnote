import { useEffect, useState } from "react";
import { footnote, inApp, num } from "./data.js";

// Mini mode (DESIGN.md section 11, D2 "Lens"): one glass lens per product and data drifting between them, in a small
// always-on-top window. Lively on purpose; still under reduced motion. The full window stays one click away.

/** Swap between the full window and mini mode. Outside the app (screenshots), the page just changes route. */
export async function switchTo(target) {
  if (!inApp) {
    const q = location.hash.includes("?") ? "?" + location.hash.split("?")[1] : "";
    location.hash = `/${target === "mini" ? "mini" : "overview"}${q}`;
    return;
  }
  const { Window, getCurrentWindow } = await import("@tauri-apps/api/window");
  const other = await Window.getByLabel(target === "mini" ? "mini" : "main");
  await other?.show();
  await other?.setFocus();
  await getCurrentWindow().hide();
}

export default function Mini() {
  const [status, setStatus] = useState(null);
  const [missing, setMissing] = useState(false);
  useEffect(() => {
    let live = true;
    const read = () =>
      footnote("status").then(
        (s) => live && (setStatus(s), setMissing(false)),
        (e) => live && setMissing(String(e) === "ENGINE_MISSING"),
      );
    read();
    const timer = setInterval(read, 30_000); // a status read is cheap; every 30 s keeps the numbers current
    return () => { live = false; clearInterval(timer); };
  }, []);
  const p = status?.products;
  // SVG motion ignores the CSS reduced-motion rule, so the moving dots are only drawn when motion is welcome.
  const still = matchMedia("(prefers-reduced-motion: reduce)").matches;
  const lens = (cls, mark, name, value, sub) => (
    <div className={`glass ${cls}`}>
      <span className="mark" aria-hidden="true">{mark}</span>
      <span className="lname">{name}</span>
      <b>{value ?? "–"}</b>
      <span className="lsub">{sub ?? ""}</span>
    </div>
  );
  return (
    <div className="mini" data-tauri-drag-region>
      <svg className="flow" viewBox="0 0 380 200" aria-hidden="true">
        <path id="f1" d="M60 104 C120 104 130 94 190 94" />
        <path id="f2" d="M320 104 C260 104 250 94 190 94" />
        {!still && (
          <>
            <circle r="2.5"><animateMotion dur="2.4s" repeatCount="indefinite"><mpath href="#f1" /></animateMotion></circle>
            <circle r="2.5"><animateMotion dur="2.8s" begin="-1.2s" repeatCount="indefinite"><mpath href="#f2" /></animateMotion></circle>
          </>
        )}
      </svg>
      {lens("lib", "≡", "Library", p ? (p.library?.built && p.library.enabled !== false ? "On" : "Off") : null)}
      {lens("mem", "◎", "Memory", p?.memory && num(p.memory.memories), p?.memory && `${num(p.memory.recalls_last_7_days)} recalls`)}
      {lens("code", "{x}", "Code", p?.code && num(p.code.graphs), p?.code && (p.code.graphs === 1 ? "graph" : "graphs"))}
      <div className="minibar">
        {missing ? <span className="warn">Engine not found</span> : <span className="dimmed">Footnote One</span>}
        <button className="btn" onClick={() => switchTo("main")}>Open full window</button>
      </div>
    </div>
  );
}
