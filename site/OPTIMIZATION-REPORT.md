# Landing page optimisation report

Measured 2026-11-20 with Lighthouse 13 (headless Chromium 158, default mobile and `--preset=desktop` throttling), axe-core (WCAG 2 A/AA, 2.1 AA, 2.2 AA, best-practice) and Playwright. Before = `design/site/index.html` on feat/283-hero-particle-wave, served by a static server (it loads Google Fonts). After = `site/dist` served by `wrangler dev` (local, HTTP on 127.0.0.1). Scores are from 3 runs; mobile Total Blocking Time is noisy in this lab (software rendering under 4x CPU slowdown), so ranges are given.

## Before / after

| Metric | Before | After |
|---|---|---|
| Lighthouse mobile: Perf / A11y / Best practices / SEO | 99 / 100 / 96 / 100 | **96-97** / 100 / 100 / 100 |
| Lighthouse desktop: Perf / A11y / Best practices / SEO | 99 / 100 / 96 / 100 | **100** / 100 / 100 / 100 |
| First-load transfer (Lighthouse, incl. hero effect) | 130.8 KB (Google Fonts, third party) | **73.8 KB** |
| First-load excluding the hero effect (stream.js, 1.9 KB br) | n/a | about 71.9 KB |
| Third-party requests | 3 (fonts.googleapis, fonts.gstatic, preconnects) | **0** |
| Mobile FCP / LCP | 1.6 s / 1.6 s | 0.8 s / 0.8 s |
| Desktop LCP | 0.5 s | 0.2 s |
| CLS (desktop) | 0.061 | **0** |
| TBT mobile / desktop | 0 ms / 0 ms | 200-220 ms / 10 ms |
| axe violations (1920x1080 and 390 px) | not run | **0** |
| Inline scripts / inline style blocks / `style=` attributes | 1 / 1 / 13 | **0 / 0 / 0** (JSON-LD data block only) |
| CSP violations in console | n/a | 0 |

Compressed sizes (gzip / brotli; Cloudflare picks): index.html 4.7 / 3.9 KB, styles 5.3 / 4.6 KB, app.js 0.9 / 0.7 KB, stream.js 2.2 / 1.9 KB. Fonts are woff2: Plus Jakarta Sans 20.7 KB, Hanken Grotesk 17.1 KB, JetBrains Mono 12.5 KB (the full Latin variable fonts were about 102 KB). Budget under 150 KB first load: met (73.8 KB), also without the effect.

### The gaps, explained

- **Mobile Performance 96-97, not 100.** All of the gap is Total Blocking Time (200-220 ms), from the hero canvas painting under a backdrop-filter glass lens while Lighthouse throttles the CPU 4x with software rendering. With the canvas disabled the same page scores 100 (TBT 0). I cut it from about 350 ms by starting the canvas after `load` and idle, giving the glyph canvas `willReadFrequently`, and dropping the SVG refraction filter on the nav (it sat over the animating canvas). Waiting for the mono font before the first layout was tried and was worse (TBT 480), so it was reverted. Going further needs a change to the hero lens design. Real GPUs do not show this cost.
- **Before's Best Practices 96** came from the third-party font requests; now 100.

## What changed

- **Structure**: index.html, styles.css, app.js (reveal, copy buttons, deferred start), stream.js (canvas, dynamic import). `build.mjs` content-hashes css, js and fonts into `/assets/`, rewrites references and fails the build on inline script/style, event handlers, third-party URLs or missing files. Critical CSS not used: one 4.6 KB stylesheet, FCP is already 0.8 s on mobile.
- **Security** (`_headers`; verified with `wrangler dev` plus curl; final check needs the deployed URL): CSP with `default-src 'self'` and every directive self or none, no unsafe-inline, `frame-ancestors 'none'`, `base-uri 'none'`, `form-action 'none'`, `object-src 'none'`, `upgrade-insecure-requests`; HSTS 2 years with includeSubDomains (no preload, see README); nosniff; X-Frame-Options DENY; `Referrer-Policy: no-referrer`; Permissions-Policy denying 20 unused features; COOP and CORP same-origin. `/assets/*` is `immutable, max-age=31536000`; HTML `no-cache`. `/.well-known/security.txt` has a placeholder contact (no personal address); robots.txt; zero third-party requests; fonts self-hosted, two preloaded, `font-display: swap`. SRI not needed.
- **Accessibility** (WCAG 2.2 AA): skip link; landmarks (footer moved out of main); heading order fixed (Results had no heading); accessible names for the copy buttons and the orbit group; scrollable commands made focusable; focus ring on light mint surfaces switched to dark ink (the glow ring was 1.5:1 there); `scroll-padding` so the sticky nav never covers focus; the dead `href="#"` link removed; a live region announces "Copied". Contrast computed for every text pair: lowest 4.94:1. Also `prefers-contrast: more`, `forced-colors`, `color-scheme: dark` and `theme-color`. The design contract is dark only, so there is no light theme; form controls and scrollbars are dark through `color-scheme`. axe lists 23 / 18 "incomplete" contrast checks (text over glass and canvas); I computed those pairs by hand instead.
- **Performance**: the lens is positioned by CSS, not by JS after load (the JS move caused 0.2 CLS in a first attempt; now 0); the canvas starts after load and idle, pauses when off screen or the tab is hidden, draws one still frame under reduced motion and reacts if the setting changes; on mobile the hero lens moved below the text (the hint text used to sit on the lens at 390 px). The page has no raster images, so AVIF/WebP/srcset/lazy-loading do not apply (icons and the OG image are PNG because crawlers need it).
- **SEO and sharing**: title, description, canonical, Open Graph and Twitter large-image card with a generated 1200x630 PNG (71 KB), JSON-LD SoftwareApplication (name, description, category, url only; no price or OS claim), sitemap.xml, favicon SVG plus 32 / 180 / 192 / 512 PNG, web manifest, `404.html` in the same design (noindex, root-absolute paths so it works at any depth).
- **Skipped on purpose**: `content-visibility` (paint containment would clip the panel that bleeds past the column; the DOM is small), View Transitions and speculation rules (one page, nothing to navigate to), critical CSS (no measurable gain).

## Manual keyboard pass (Playwright, 1920x1080)

Tab order: skip link, logo, Library, Memory, Code, Results, Install, the two hero buttons, three product cards, the "All five corpora" summary, then each install command and its Copy button, footer logo, and back to the start. Every stop had a 2 px visible outline, was in view and was not under the sticky nav. Skip link plus Enter lands on `#main`. Copy buttons are named per command. Not done: a screen-reader pass (NVDA/VoiceOver) and a real-device touch test.

## Motion checks

On screen the canvas ran at the display rate (about 120 callbacks/s in headless; a bare page measures about 100); scrolled off screen: 0/s; `prefers-reduced-motion: reduce`: 0/s, 0 running animations, nothing left hidden by the reveal script.

## Design critique at 1920x1080 and 390 px

I reviewed the screenshots myself; the independent critic agent was not run (flag for Johan). Findings: desktop matches the original apart from the font source; the hero lens overlapped the hint text on mobile (fixed); full-page captures show blocks blank until scrolled because the original reveal-on-scroll is kept (skipped under reduced motion). The background choice and the prototypes were not touched.

## Not verified

Live headers and compression on the deployed URL; Lighthouse over `https://`; HSTS preload; social cards in real crawlers; screen readers.
