# Footnote One website

A static four-page site (Launcher/Install at `/`, plus `/library`, `/memory`, `/code` marked soon — no framework, no third-party requests) deployed as a Cloudflare Worker with static assets.

```
site/pages.mjs         single source of metadata (titles, descriptions, OG/Twitter tags, social card copy, llms.txt lines)
site/src/pages/*.html  <main> bodies for index, library, memory, code
site/src/partials/     shared defs.html, nav.html, footer.html (so pages cannot drift)
site/md.mjs            HTML-to-Markdown converter used by build.mjs
site/build.mjs         builds dist/: wraps pages, emits /llms.txt, /llms-full.txt, /*.md twins, sitemap.xml; hashes css/js/fonts; asserts CSP, links, anchors, single h1 and social cards (<300 KB)
site/dist/             build output (git-ignored); this is what wrangler serves
wrangler.jsonc         worker name `footnote`, assets = site/dist, not_found_handling = 404-page, no worker script
```

## Build and test locally

```sh
node site/build.mjs
npx wrangler@latest dev          # http://localhost:8787, headers from _headers apply
curl -sI http://localhost:8787/  # check CSP, HSTS, Link: </llms.txt>; rel="llms-txt", nosniff
curl -sI http://localhost:8787/index.md # Content-Type: text/markdown; charset=utf-8
```

## Noindexed preview deploy

```sh
PREVIEW=1 node site/build.mjs
npx wrangler@latest deploy --name footnote-preview
```

`PREVIEW=1` adds `X-Robots-Tag: noindex, nofollow` to `dist/_headers` and switches `dist/robots.txt` to `Disallow: /` (with the `llms.txt` pointer). The files in `src/` stay production-ready.

## Production deploy (manual step)

```sh
SITE_URL=https://<canonical-domain> node site/build.mjs
npx wrangler@latest deploy
```

Then verify live headers on `/` (HSTS, CSP, `Link: </llms.txt>; rel="llms-txt"`), `/assets/*` (`immutable`), `/index.md` (`text/markdown; charset=utf-8` + `nosniff`), `/llms.txt`, `/llms-full.txt`, and `/og.png` (`200 image/png`).

## Before the site is public

1. **security.txt**: contact is `johan.velocity.lab@pm.me`, `Expires` is `2027-11-20`; move `Expires` forward before then.
2. **Canonical URL**: `src/` keeps `%SITE_URL%` placeholders; `build.mjs` defaults to `https://footnote-preview.johanproust.workers.dev`, the canonical address until a custom domain exists. Then pass `SITE_URL=https://<domain>` (or change the default) and update the repository homepage.
3. **HSTS preload**: `_headers` sets `max-age=63072000; includeSubDomains`. Do not add `preload` on `workers.dev`. With a custom domain, add `; preload` and submit at hstspreload.org only once every subdomain is HTTPS.
4. **Screen-reader pass**: automated axe-core and keyboard Tab checks are verified; a manual NVDA/VoiceOver pass is still open.

## Changing things

- Fonts: `python site/scripts/subset-fonts.py <dir with node_modules/@fontsource-variable> site/src/fonts` (Latin subset, narrowed weight axis; SIL OFL).
- Icons and social cards (`og.png`, `og-library.png`, `og-memory.png`, `og-code.png`): `site/scripts/make-assets.mjs` (outputs are committed).
- Every reference uses a root-absolute path so `404.html` works at any depth.
