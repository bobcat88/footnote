// Tiny build: copy src/ to dist/, put content hashes in the names of css, js and fonts (so they can be cached for a year),
// rewrite the references, then assert the result obeys the CSP (no inline script or style, nothing third-party).
// Usage: node site/build.mjs     SITE_URL=https://example.com node site/build.mjs
import { createHash } from "node:crypto";
import { cpSync, rmSync, mkdirSync, readFileSync, writeFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, basename, extname, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";

const root = dirname(fileURLToPath(import.meta.url)), src = join(root, "src"), out = join(root, "dist");
const SITE_URL = (process.env.SITE_URL || "https://footnote.johanproust.workers.dev").replace(/\/$/, "");
rmSync(out, { recursive: true, force: true });
cpSync(src, out, { recursive: true });
mkdirSync(join(out, "assets"));

const text = p => readFileSync(join(out, p), "utf8");
const hash = b => createHash("sha256").update(b).digest("hex").slice(0, 10);
const hashed = new Map(); // source path ("styles.css") -> output name ("styles.ab12cd34ef.css")
const rename = (path, buf) => { const e = extname(path), n = basename(path, e) + "." + hash(buf) + e; writeFileSync(join(out, "assets", n), buf); rmSync(join(out, path)); hashed.set(path, n); return n; };

// Leaves first, so a file's hash covers the hashed names it points to: fonts, then stream.js, app.js and styles.css.
for (const f of readdirSync(join(out, "fonts"))) rename("fonts/" + f, readFileSync(join(out, "fonts", f)));
rmSync(join(out, "fonts"), { recursive: true });
rename("stream.js", Buffer.from(text("stream.js")));
rename("app.js", Buffer.from(text("app.js").replace('"./stream.js"', `"./${hashed.get("stream.js")}"`)));
let css = text("styles.css"); for (const [p, n] of hashed) if (p.startsWith("fonts/")) css = css.replaceAll(p, n);
rename("styles.css", Buffer.from(css));

// Pages and root files: point at /assets/<hashed>, fill in the site URL.
for (const f of readdirSync(out).filter(f => /\.(html|xml|txt|webmanifest)$/.test(f))) writeFileSync(join(out, f), fixRefs(text(f)));
const sec = join(out, ".well-known", "security.txt"); writeFileSync(sec, fixRefs(readFileSync(sec, "utf8")));
function fixRefs(s) { s = s.replaceAll("%SITE_URL%", SITE_URL); for (const [p, n] of hashed) s = s.replaceAll(`"/${p}"`, `"/assets/${n}"`); return s; }

// Assertions: what the CSP forbids must not be here.
for (const f of readdirSync(out).filter(f => f.endsWith(".html"))) {
  const h = text(f);
  assert(!/\sstyle\s*=/i.test(h) && !/<style[\s>]/i.test(h), f + ": inline style");
  assert(!/<script(?![^>]*\ssrc=)(?![^>]*type="application\/ld\+json")[^>]*>/i.test(h), f + ": inline script");
  assert(!/\son[a-z]+\s*=/i.test(h.replace(/<script[\s\S]*?<\/script>/g, "")), f + ": inline event handler");
  assert(!/(?:href|src)="https?:\/\/(?!footnote\.)/i.test(h.replaceAll(SITE_URL, "")), f + ": third-party URL");
  for (const m of h.matchAll(/(?:href|src)="(\/[^"#?]*)"/g)) if (m[1] !== "/") assert(existsSync(join(out, m[1])), f + ": missing " + m[1]);
}
assert(!/https?:\/\//.test(css), "css has an absolute URL");
for (const n of hashed.values()) assert(statSync(join(out, "assets", n)).size > 0);
// Preview deploys (PREVIEW=1) are kept out of search engines; production files in src/ are untouched.
if (process.env.PREVIEW) {
  writeFileSync(join(out, "robots.txt"), "User-agent: *\nDisallow: /\n");
  writeFileSync(join(out, "_headers"), readFileSync(join(out, "_headers"), "utf8").replace(/^\/\*\r?$/m, "/*\n  X-Robots-Tag: noindex, nofollow"));
}
console.log("built", out, "->", SITE_URL, "\n" + [...hashed.values()].join("\n"));
