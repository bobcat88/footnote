// Reads from the engine. In the app, through the Tauri `footnote` command (read-only allowlist in src-tauri/src/main.rs).
// In a plain browser, only with `?mock=1&state=ok|empty|missing|loading`: invented sample data for the QLoop screenshots.
import { invoke } from "@tauri-apps/api/core";
import { fixtures } from "./fixtures.js";

const params = new URLSearchParams(location.search || location.hash.split("?")[1] || "");
export const inApp = "__TAURI_INTERNALS__" in window;
const mockState = params.get("mock") ? params.get("state") || "ok" : null;

export function footnote(...args) {
  if (inApp) return invoke("footnote", { args });
  if (mockState === "loading") return new Promise(() => {});
  if (mockState === "missing") return Promise.reject("ENGINE_MISSING");
  if (mockState) {
    const key = args.join(" ");
    const set = fixtures[mockState] ?? fixtures.ok;
    const hit = Object.entries(set).find(([k]) => key === k || key.startsWith(k + " "));
    return hit ? Promise.resolve(structuredClone(hit[1])) : Promise.reject(`no sample for: footnote ${key}`);
  }
  return Promise.reject("ENGINE_MISSING");
}

/** Run several engine reads at once; resolves to { data } or { error }. */
export async function load(reads) {
  try {
    const values = await Promise.all(reads.map((r) => footnote(...r)));
    return { data: values };
  } catch (e) {
    return { error: String(e) };
  }
}

const nf = new Intl.NumberFormat("en-GB");
export const num = (n) => (typeof n === "number" ? nf.format(n) : "–");

const MONTHS = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];
/** `2026-11-16` as `16 Nov`. */
export const shortDate = (iso) => {
  const [, m, d] = iso.split("-").map(Number);
  return `${d} ${MONTHS[m - 1]}`;
};

/** `today`, `yesterday`, or a short date, from a local `YYYY-MM-DD`. */
export function seen(iso, now = new Date()) {
  const day = (d) => `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;
  if (iso === day(now)) return "today";
  const y = new Date(now);
  y.setDate(now.getDate() - 1);
  return iso === day(y) ? "yesterday" : shortDate(iso);
}

export const AGENTS = {
  "claude-code": "Claude Code",
  codex: "Codex",
  grok: "Grok",
  "deepseek-harness": "DeepSeek Harness",
  gemini: "Gemini CLI",
};

export const mb = (bytes) => `${num(Math.round(bytes / 1e6))} MB`;
