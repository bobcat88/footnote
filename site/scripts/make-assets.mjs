// One-off: renders favicon PNGs and the 1200x630 social preview. The outputs are committed; rerun only to change them.
// Needs (not in package.json, it is a design-time tool): npm i sharp playwright-core, and a Chromium path in CHROME_PATH.
import sharp from "sharp"; import { chromium } from "playwright-core"; import { readFileSync, writeFileSync } from "node:fs"; import { pathToFileURL } from "node:url";
const src = new URL("../src/", import.meta.url), svg = readFileSync(new URL("favicon.svg", src));
for (const [n, s] of [["icon-32", 32], ["icon-192", 192], ["icon-512", 512], ["apple-touch-icon", 180]]) await sharp(svg, { density: 600 }).resize(s, s).png({ compressionLevel: 9 }).toFile(new URL(n + ".png", src).pathname.slice(1));
const html = `<style>@font-face{font-family:J;src:url(${pathToFileURL(new URL("fonts/plus-jakarta-sans.woff2", src).pathname.slice(1))});font-weight:300 700}
@font-face{font-family:M;src:url(${pathToFileURL(new URL("fonts/jetbrains-mono.woff2", src).pathname.slice(1))});font-weight:400 600}
body{margin:0;width:1200px;height:630px;background:radial-gradient(60% 80% at 82% 50%,#17382f,#09120b 70%);color:#eaf0eb;font-family:J;display:grid;align-content:center;padding:0 90px;box-sizing:border-box}
.l{font:700 40px J;letter-spacing:-.035em;display:flex;gap:10px;align-items:center}.l i{width:42px;height:42px;border-radius:50%;border:2px solid #5fd3c3;display:grid;place-items:center;font:600 20px M;color:#5fd3c3;font-style:normal}
h1{font-weight:300;font-size:84px;line-height:1.02;letter-spacing:-.045em;margin:48px 0 0;max-width:900px}h1 em{font-style:normal;font-weight:500;color:#5fd3c3}
p{font:400 32px J;color:#a2afa5;margin:28px 0 0}</style>
<div class="l">Footnote<i>1</i></div><h1>What comes into focus is <em>your company's knowledge.</em></h1><p>Local memory your coding agents can cite.</p>`;
const b = await chromium.launch({ executablePath: process.env.CHROME_PATH }), p = await b.newPage({ viewport: { width: 1200, height: 630 } });
await p.setContent(html); await p.evaluate(() => document.fonts.ready);
const png = await p.screenshot(); await b.close();
await sharp(png).png({ palette: true, compressionLevel: 9 }).toFile(new URL("og.png", src).pathname.slice(1));
