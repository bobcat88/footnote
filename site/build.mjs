// Build: src/ -> dist/. Pages are src/pages/<slug>.html (the <main> content) wrapped by one head, nav and footer, so pages cannot drift.
// The same page bodies become the Markdown twins, llms.txt and llms-full.txt. CSS, JS and fonts get content hashes in their names.
// Then it asserts the CSP rules (no inline script or style, nothing third-party) and checks every link.
// Usage: node site/build.mjs     SITE_URL=https://example.com node site/build.mjs     PREVIEW=1 node site/build.mjs
//        CHECK_EXTERNAL=1 node site/build.mjs   (also fetches the external links; needs network)
import { createHash } from "node:crypto";
import { cpSync, rmSync, mkdirSync, readFileSync, writeFileSync, readdirSync, statSync, existsSync } from "node:fs";
import { join, basename, extname, dirname } from "node:path";
import { fileURLToPath } from "node:url";
import assert from "node:assert/strict";
import { pages, GH, SUPPORT } from "./pages.mjs";
import { htmlToMd } from "./md.mjs";

const root = dirname(fileURLToPath(import.meta.url)), src = join(root, "src"), out = join(root, "dist");
const PREVIEW = !!process.env.PREVIEW;
const SITE_URL = (process.env.SITE_URL || (PREVIEW ? "https://preview.footnote.example" : "https://footnote.example")).replace(/\/$/, "");
rmSync(out, { recursive: true, force: true });
cpSync(src, out, { recursive: true });
mkdirSync(join(out, "assets"));
rmSync(join(out, "pages"), { recursive: true }); rmSync(join(out, "partials"), { recursive: true });

const text = p => readFileSync(join(out, p), "utf8");
const part = n => readFileSync(join(src, "partials", n + ".html"), "utf8");
const body = s => readFileSync(join(src, "pages", s + ".html"), "utf8");
const esc = s => s.replace(/&/g, "&amp;").replace(/"/g, "&quot;").replace(/</g, "&lt;");
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

// ---- pages
const image = p => (p.slug === "index" ? "/og.png" : `/og-${p.slug}.png`);
const nav = cur => part("nav").replace(/\{\{c:(\w+)\}\}/g, (_, s) => (s === cur ? ' aria-current="page"' : ""));
function jsonld(p) {
  const url = `%SITE_URL%${p.path}`, app = { "@type": "SoftwareApplication", name: p.name, description: p.description, applicationCategory: "DeveloperApplication", url, isAccessibleForFree: true, license: GH + "/blob/main/LICENSE" };
  if (p.slug === "index") return { "@context": "https://schema.org", ...app, offers: { "@type": "Offer", price: "0", priceCurrency: "USD" } };
  return { "@context": "https://schema.org", "@graph": [{ ...app, isPartOf: { "@type": "SoftwareApplication", name: "Footnote One", url: "%SITE_URL%/" } },
    { "@type": "BreadcrumbList", itemListElement: [{ "@type": "ListItem", position: 1, name: "Footnote One", item: "%SITE_URL%/" }, { "@type": "ListItem", position: 2, name: p.name, item: url }] }] };
}
const page = (p, main) => `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1, viewport-fit=cover">
<title>${esc(p.title)}</title>
<meta name="description" content="${esc(p.description)}">
<meta name="color-scheme" content="dark">
<meta name="theme-color" content="#09120b">
<link rel="canonical" href="%SITE_URL%${p.path}">
<meta property="og:type" content="website">
<meta property="og:site_name" content="Footnote">
<meta property="og:locale" content="en_US">
<meta property="og:title" content="${esc(p.og.title)}">
<meta property="og:description" content="${esc(p.og.description)}">
<meta property="og:url" content="%SITE_URL%${p.path}">
<meta property="og:image" content="%SITE_URL%${image(p)}">
<meta property="og:image:type" content="image/png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:alt" content="${esc(p.og.alt)}">
<meta name="twitter:card" content="summary_large_image">
<meta name="twitter:title" content="${esc(p.og.title)}">
<meta name="twitter:description" content="${esc(p.og.description)}">
<meta name="twitter:image" content="%SITE_URL%${image(p)}">
<meta name="twitter:image:alt" content="${esc(p.og.alt)}">
<link rel="icon" href="/favicon.svg" type="image/svg+xml">
<link rel="icon" href="/icon-32.png" type="image/png" sizes="32x32">
<link rel="apple-touch-icon" href="/apple-touch-icon.png">
<link rel="manifest" href="/manifest.webmanifest">
<link rel="alternate" type="text/markdown" href="%SITE_URL%${p.path === "/" ? "/index" : p.path}.md">
<link rel="preload" href="/fonts/plus-jakarta-sans.woff2" as="font" type="font/woff2" crossorigin>
<link rel="preload" href="/fonts/hanken-grotesk.woff2" as="font" type="font/woff2" crossorigin>
<link rel="stylesheet" href="/styles.css">
<script type="module" src="/app.js"></script>
<script type="application/ld+json">${JSON.stringify(jsonld(p))}</script>
</head>
<body>
${part("defs")}
${nav(p.slug)}
<main id="main" tabindex="-1">
${main}
</main>
${part("footer")}
</body>
</html>
`;
const mains = {};
for (const p of pages) { mains[p.slug] = body(p.slug); writeFileSync(join(out, p.slug + ".html"), page(p, mains[p.slug])); }
writeFileSync(join(out, "404.html"), text("404.html").replace("{{defs}}", part("defs")).replace("{{nav}}", nav("")).replace("{{footer}}", part("footer")));

// ---- Markdown twins and llms files, from the same bodies
const absolute = h => (h.startsWith("/") ? SITE_URL + h : h);
const mdHref = h => { // internal page links point at the Markdown twins
  const m = /^(\/[^#?]*)(#.*)?$/.exec(h); if (!m) return h;
  const p = pages.find(x => x.path === m[1]); return p ? `${SITE_URL}/${p.slug}.md${m[2] || ""}` : SITE_URL + h;
};
const twins = pages.map(p => {
  const md = htmlToMd(mains[p.slug], mdHref);
  return { p, md: `${md}\n\n---\n\nPage: ${SITE_URL}${p.path}\nSource: ${SITE_URL}/llms.txt lists every page.\n` };
});
for (const { p, md } of twins) writeFileSync(join(out, p.slug + ".md"), `${md.replace(/^(# .*)\n/m, `$1\n\n> ${p.description}\n`)}`);
const home = pages[0];
writeFileSync(join(out, "llms.txt"), `# Footnote One

> ${home.og.description} Free, Apache-2.0, runs on your machine. Footnote One is the free product family; the numbers on every page come with their conditions and dates.

## Pages

- [Install and Launcher](${SITE_URL}/index.md#install): ${home.line}
${pages.slice(1).map(p => `- [${p.name}](${SITE_URL}/${p.slug}.md): ${p.line}`).join("\n")}

## Evidence and source

- [Benchmark index on GitHub](${GH}/blob/main/bench/INDEX.md): every report with status, verbatim headline and line link
- [Benchmarks folder on GitHub](${GH}/tree/main/bench): receipts, tables and the scripts that reproduce every number
- [Source repository](${GH}): code, README and issues

## Support

- [GitHub issues](${GH}/issues): bug reports and questions
- [Email](mailto:${SUPPORT}): ${SUPPORT}

## Optional

- [Whole site as one Markdown file](${SITE_URL}/llms-full.txt)
`);
writeFileSync(join(out, "llms-full.txt"), `# Footnote One: full site content\n\nEvery page of ${SITE_URL} as Markdown, in one file. Index: ${SITE_URL}/llms.txt\n\n` + twins.map(({ p, md }) => `<!-- ${SITE_URL}${p.path} -->\n\n${md.replace(/^(# .*)\n/m, `$1\n\n> ${p.description}\n`)}`).join("\n\n"));

// ---- root files
writeFileSync(join(out, "sitemap.xml"), `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">${pages.map(p => `<url><loc>${SITE_URL}${p.path}</loc></url>`).join("")}</urlset>\n`);
for (const f of readdirSync(out).filter(f => /\.(html|xml|txt|webmanifest)$/.test(f) && !f.startsWith("llms"))) writeFileSync(join(out, f), fixRefs(text(f)));
const sec = join(out, ".well-known", "security.txt"); writeFileSync(sec, fixRefs(readFileSync(sec, "utf8")));
function fixRefs(s) { s = s.replaceAll("%SITE_URL%", SITE_URL); for (const [p, n] of hashed) s = s.replaceAll(`"/${p}"`, `"/assets/${n}"`); return s; }

// ---- assertions: what the CSP forbids must not be here; every link resolves
const htmls = readdirSync(out).filter(f => f.endsWith(".html"));
const target = u => { const p = u === "/" ? "/index.html" : u; return [p, p + ".html"].map(x => join(out, x)).find(x => existsSync(x) && statSync(x).isFile()); };
const ids = f => new Set([...readFileSync(f, "utf8").matchAll(/\sid="([^"]+)"/g)].map(m => m[1]));
const external = new Set();
const checkLink = (from, h) => {
  if (/^mailto:/.test(h)) return;
  if (/^https?:\/\//.test(h)) { assert(h === GH || h.startsWith(GH + "/") || h.startsWith(SITE_URL), from + ": third-party URL " + h); if (!h.startsWith(SITE_URL)) external.add(h); if (!h.startsWith(SITE_URL)) return; h = h.slice(SITE_URL.length) || "/"; }
  const m = /^([^#?]*)(?:#(.*))?/.exec(h), path = m[1] || "/", t = target(path.replace(/\.md$/, "") === path ? path : path) ;
  assert(t, from + ": broken link " + h);
  if (m[2] && t.endsWith(".html")) assert(ids(t).has(m[2]), from + ": missing anchor " + h);
};
for (const f of htmls) {
  const h = text(f);
  assert(!/\sstyle\s*=/i.test(h) && !/<style[\s>]/i.test(h), f + ": inline style");
  assert(!/<script(?![^>]*\ssrc=)(?![^>]*type="application\/ld\+json")[^>]*>/i.test(h), f + ": inline script");
  assert(!/\son[a-z]+\s*=/i.test(h.replace(/<script[\s\S]*?<\/script>/g, "")), f + ": inline event handler");
  for (const m of h.matchAll(/(?:href|src)="([^"#][^"]*|#[^"]+)"/g)) { const u = m[1]; if (u.startsWith("#")) assert(ids(join(out, f)).has(u.slice(1)), f + ": missing anchor " + u); else if (/^(?:\/|https?:|mailto:)/.test(u)) { if (u.startsWith("/") && /\.(?:md|txt)$/.test(u) && existsSync(join(out, u))) continue; if (u.startsWith("/") && !u.startsWith("//") && /\.\w+$/.test(u) && !/\.html$/.test(u)) assert(existsSync(join(out, u)), f + ": missing " + u); else checkLink(f, u); } }
  assert((h.match(/<h1[\s>]/g) || []).length === (f === "404.html" ? 1 : 1), f + ": needs exactly one h1");
}
for (const f of readdirSync(out).filter(f => f.endsWith(".md") || f === "llms.txt" || f === "llms-full.txt")) for (const m of readFileSync(join(out, f), "utf8").matchAll(/\]\(([^)\s]+)\)/g)) {
  const u = m[1]; if (u.startsWith(SITE_URL)) { const [p, hash] = u.slice(SITE_URL.length).split("#"); assert(existsSync(join(out, p)), f + ": broken link " + u); if (hash && p.endsWith(".md")) assert(ids(join(out, p.replace(/\.md$/, ".html").replace(/^\/index/, "/index"))).has(hash), f + ": missing anchor " + u); } else if (/^https?:/.test(u)) { assert(u === GH || u.startsWith(GH + "/"), f + ": third-party URL " + u); external.add(u); }
}
assert(!/https?:\/\//.test(css), "css has an absolute URL");
for (const n of hashed.values()) assert(statSync(join(out, "assets", n)).size > 0);
for (const p of pages) { const f = join(out, image(p)); assert(existsSync(f), "missing social card " + image(p)); assert(statSync(f).size < 300 * 1024, image(p) + " over 300 KB"); }
if (process.env.CHECK_EXTERNAL) for (const u of external) { const r = await fetch(u, { method: "HEAD", redirect: "follow" }).catch(e => ({ status: String(e) })); console.log(r.status, u); }

// Preview deploys (PREVIEW=1) are kept out of search engines; production files in src/ are untouched.
const llmsRef = `\n# LLM-readable index: ${SITE_URL}/llms.txt\n`;
if (PREVIEW) {
  writeFileSync(join(out, "robots.txt"), "User-agent: *\nDisallow: /\n" + llmsRef);
  writeFileSync(join(out, "_headers"), readFileSync(join(out, "_headers"), "utf8").replace(/^\/\*\r?$/m, "/*\n  X-Robots-Tag: noindex, nofollow"));
}
console.log("built", out, "->", SITE_URL, PREVIEW ? "(preview, noindex)" : "", "\n" + [...hashed.values()].join("\n"), "\nexternal links (not fetched unless CHECK_EXTERNAL=1):", [...external].length);
