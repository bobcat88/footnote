# Footnote landing page

A static page (no framework, no third-party requests) deployed as a Cloudflare Worker with static assets.

```
site/src/      source: index.html, 404.html, styles.css, app.js, stream.js, fonts/, icons, _headers, robots.txt, sitemap.xml, .well-known/
site/build.mjs copies src/ to dist/, hashes css/js/fonts, rewrites references, asserts the CSP rules (no inline script/style, no third-party URL)
site/dist/     build output (git-ignored); this is what wrangler serves
wrangler.jsonc worker name `footnote`, assets = site/dist, not_found_handling = 404-page, no worker script
```

## Build and test locally

```
node site/build.mjs
npx wrangler@latest dev         # http://localhost:8787, headers from _headers apply
curl -sI http://localhost:8787/ # check headers
```

## Deploy (needs Johan's go; not done yet)

```
node site/build.mjs
npx wrangler@latest deploy      # -> https://footnote.johanproust.workers.dev
```

Then verify the headers on the live URL: `curl -sI https://footnote.johanproust.workers.dev/` and an asset under `/assets/` (must say `immutable`). Local `wrangler dev` cannot prove HSTS, the HTTPS upgrade or Cloudflare compression.

## Before the site is public

1. **security.txt**: contact is the Footnote support address, `Expires` is 2027-11-20; move `Expires` forward before then.
2. **Canonical URL**: defaults to `https://footnote.johanproust.workers.dev`. When a custom domain exists, run `SITE_URL=https://example.com node site/build.mjs` and redeploy. The URL feeds canonical, Open Graph, JSON-LD, sitemap, robots and security.txt.
3. **HSTS preload**: the header is `max-age=63072000; includeSubDomains` (2 years). Do not add `preload` on `workers.dev`. With a custom domain, add `; preload` in `src/_headers` and submit it at hstspreload.org only once every subdomain is HTTPS.
4. **Benchmark link**: the results section says the procedure and disclaimers will be published with the repository; replace it with a real link once the repository is public.
5. The Results numbers were not changed; check them against `FINAL-RANKING.md` (issue #327) before launch.

## Changing things

- Fonts: `python site/scripts/subset-fonts.py <dir with node_modules/@fontsource-variable> site/src/fonts` (Latin subset, narrowed weight axis; SIL OFL).
- Icons and social image: `site/scripts/make-assets.mjs` (outputs are committed).
- Every reference uses a root-absolute path so `404.html` works at any depth.
- The page moved from `design/site/index.html` to `site/src/index.html`; the prototypes and lab stay in `design/site/`.

## Noindexed preview

`PREVIEW=1 SITE_URL=https://footnote-preview.johanproust.workers.dev node site/build.mjs && npx wrangler deploy --name footnote-preview` builds a copy with `X-Robots-Tag: noindex, nofollow` and `Disallow: /` in robots.txt. The files in `src/` stay production-ready.
