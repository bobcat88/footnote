// One-off: renders favicon PNGs and the four 1200x630 social preview cards from pages.mjs.
// The outputs are committed; rerun only to change them.
// Needs (not in package.json, it is a design-time tool): npm i sharp playwright-core, and a Chromium path in CHROME_PATH.
import sharp from "sharp";
import { chromium } from "playwright-core";
import { readFileSync } from "node:fs";
import { pathToFileURL, fileURLToPath } from "node:url";
import { join, dirname } from "node:path";
import { pages } from "../pages.mjs";

const root = dirname(fileURLToPath(import.meta.url));
const src = join(root, "..", "src");
const svg = readFileSync(join(src, "favicon.svg"));
const defs = readFileSync(join(src, "partials", "defs.html"), "utf8");

for (const [n, s] of [["icon-32", 32], ["icon-192", 192], ["icon-512", 512], ["apple-touch-icon", 180]]) {
  await sharp(svg, { density: 600 }).resize(s, s).png({ compressionLevel: 9 }).toFile(join(src, n + ".png"));
}

const fontData = name => `data:font/woff2;base64,${readFileSync(join(src, "fonts", name)).toString("base64")}`;
const fontJ = fontData("plus-jakarta-sans.woff2");
const fontH = fontData("hanken-grotesk.woff2");
const fontM = fontData("jetbrains-mono.woff2");

const markMarkup = mark => mark === "n"
  ? `<span class="badge-n">1</span>`
  : `<svg class="mk"><use href="#${mark}"/></svg>`;

// All card content stays strictly inside the centre 80% safe zone (120px X margin, 63px Y margin).
const cardHtml = p => `<!doctype html><meta charset="utf-8"><style>
@font-face{font-family:J;src:url(${fontJ});font-weight:300 700}
@font-face{font-family:H;src:url(${fontH});font-weight:400 600}
@font-face{font-family:M;src:url(${fontM});font-weight:400 600}
*{box-sizing:border-box;margin:0}
body{width:1200px;height:630px;background:radial-gradient(56% 78% at 78% 50%,#183b31,#09120b 72%);color:#eaf0eb;font-family:J;
  display:grid;grid-template-columns:1fr 220px;gap:48px;align-items:center;padding:72px 132px;overflow:hidden}
.defs{position:absolute;width:0;height:0}
.copy{display:flex;flex-direction:column;justify-content:center}
.eyebrow{font:500 18px M;letter-spacing:.1em;text-transform:uppercase;color:#5fd3c3;display:flex;align-items:center;gap:12px}
.eyebrow i{width:28px;height:28px;border-radius:50%;border:1.5px solid #5fd3c3;display:grid;place-items:center;font:600 14px M;color:#5fd3c3;font-style:normal}
h1{font-weight:300;font-size:66px;line-height:1.05;letter-spacing:-.04em;margin-top:26px}
h1 em{font-style:normal;font-weight:500;color:#5fd3c3}
p{font:400 26px/1.38 H;color:#a2afa5;margin-top:22px;max-width:560px}
.lens{width:220px;height:220px;border-radius:50%;display:grid;place-items:center;
  background:radial-gradient(65% 65% at 35% 30%,rgba(234,240,235,.14),rgba(234,240,235,.04));
  border:1.5px solid rgba(234,240,235,.28);
  box-shadow:inset 0 2px 0 rgba(255,255,255,.35),inset 0 -14px 28px rgba(0,0,0,.35),0 24px 60px rgba(0,0,0,.5)}
.lens svg{width:116px;height:116px}
.mk{fill:none;stroke:#5fd3c3;stroke-width:1.4;stroke-linecap:round;stroke-linejoin:round}
.badge-n{font:600 92px/1 M;color:#5fd3c3}
</style>
${defs}
<div class="copy">
  <div class="eyebrow"><i>1</i><span>${p.og.card[0]}</span></div>
  <h1>${p.og.card[1]}</h1>
  <p>${p.og.card[2]}</p>
</div>
<div class="lens">${markMarkup(p.mark)}</div>`;

const chrome = process.env.CHROME_PATH || "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe";
const b = await chromium.launch({ executablePath: chrome });
const page = await b.newPage({ viewport: { width: 1200, height: 630 } });
for (const p of pages) {
  await page.setContent(cardHtml(p));
  await page.evaluate(() => document.fonts.ready);
  const png = await page.screenshot();
  const outName = p.slug === "index" ? "og.png" : `og-${p.slug}.png`;
  const outPath = join(src, outName);
  await sharp(png).png({ palette: true, compressionLevel: 9 }).toFile(outPath);
  console.log("wrote", outName);
}
await b.close();
