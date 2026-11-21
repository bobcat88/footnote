---
name: launch-website
description: Use before a website goes live or is handed to a client, and when auditing a live site: HTTPS and redirects, security headers and CSP, metadata and sharing previews, broken links, real 404s, robots and sitemap, Core Web Vitals, accessibility, cookies and consent, legal notices, backups and monitoring. Runs launch-check to crawl the site.
license: MIT
---

# Launch a website

Record first: publisher + jurisdiction / audience / site type / accounts or payments / personal data / browsers + devices. Only checks that apply are gates.

- **Ship** (prototype, MVP, internal): every applicable check has evidence / no FAIL from launch-check
- **Best** (client delivery, real product): + tested on production / ops owner named / post-launch measurement plan. Only the client waives a Best check, in writing, with the reason
- Unrun = pending, never pass

## Run
`node <skills>/launch-website/scripts/launch-check/launch-check.mjs <url> [--max 50] [--external] [--out report.json]` → crawls same-origin pages, exit 1 on FAIL. Tests: `node --test scripts/launch-check/tests`.

| Id | Check | Level |
|---|---|---|
| L1 | `http://` → `https://` by 301/308 / no redirect chains | FAIL / WARN |
| L2 | HSTS / CSP / `frame-ancestors` or XFO / `nosniff` / Referrer-Policy | WARN |
| L3 | title / `lang` / viewport (FAIL) · description / canonical / `og:title` + `og:image` / one h1 / unique titles (WARN) | FAIL / WARN |
| L4 | internal links < 400 / `--external` checks outbound | FAIL / WARN |
| L5 | robots.txt / sitemap.xml / `noindex` left from staging | WARN |
| L6 | HTML gzip or br | WARN |
| L7 | missing page → 404, not soft 200 | FAIL |
| L8 | no `http://` resources on https pages | FAIL |
| L9 | every `<img>` has `alt` (`alt=""` = decorative) | FAIL |

Then `design-check` (review-ui-quality) for contrast / focus / targets, and its loop for visual acceptance.

## Not scriptable
- **Speed:** good = LCP ≤ 2.5 s / INP ≤ 200 ms / CLS ≤ 0.1 at p75 of real visits. Lab gives LCP + CLS; INP needs field data (a new site has none: say so, plan collection). LCP → no lazy hero, discover it early / INP → split long tasks, less JS / CLS → reserve media + font space
- **Accessibility:** WCAG 2.2 AA / keyboard through every flow / visible focus / labels + errors / 200% zoom / reflow at 320 px
- **Security:** server-side validation / rate limits on mail + signup / no secrets in bundle or repo / least-privilege hosting + DNS / CSP in Report-Only first, never `unsafe-inline` to pass a test
- **Legal + ops:** [references/legal-and-ops.md](references/legal-and-ops.md)

Never as a side effect: change DNS / submit real forms / send mail / create accounts / publish.

## Report
Pass / fail / n.a. per check + script report + URLs + build revision + open decisions with owners.
