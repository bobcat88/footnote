---
skill_hash: eb679d3db2804e56
reviewed: 2026-11-21
---
# launch-website: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| CWV thresholds at p75 | [Web Vitals](https://web.dev/articles/vitals) |
| LCP fixes | [Optimize LCP](https://web.dev/articles/optimize-lcp) |
| INP fixes | [Optimize INP](https://web.dev/articles/optimize-inp) · [Long tasks](https://web.dev/articles/optimize-long-tasks) |
| CSP, Report-Only | [MDN CSP](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CSP) |
| HSTS | [MDN HSTS](https://developer.mozilla.org/en-US/docs/Web/HTTP/Reference/Headers/Strict-Transport-Security) |
| Mixed content | [MDN mixed content](https://developer.mozilla.org/en-US/docs/Web/Security/Mixed_content) |
| 301/308 permanent redirects | [MDN redirections](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/Redirections) |
| Soft 404s | [Google Search: HTTP errors](https://developers.google.com/search/docs/crawling-indexing/http-network-errors#soft-404-errors) |
| Sitemaps don't guarantee indexing | [Google: sitemaps](https://developers.google.com/search/docs/crawling-indexing/sitemaps/overview) |
| Sharing metadata | [Open Graph](https://ogp.me/) |
| lang (3.1.1), alt (1.1.1), reflow, focus | [WCAG 2.2](https://www.w3.org/TR/WCAG22/) |
| French business site notices | [Service Public F37351](https://entreprendre.service-public.gouv.fr/vosdroits/F37351) |
| Processors (Art. 28) | [CNIL GDPR ch. 4](https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre4) |
| Cookie consent | [CNIL cookie FAQ](https://www.cnil.fr/fr/cookies-et-autres-traceurs/regles/cookies/FAQ) |
| European Accessibility Act scope | [DGCCRF](https://www.economie.gouv.fr/dgccrf/les-fiches-pratiques/la-nouvelle-directive-europeenne-accessibilite-pour-des-produits-et-des-services-accessibles-aux-personnes-en-situation) |

## Field evidence
- 2026-11-21: launch-check on a Workers preview found `http://` answering 200 (workers.dev doesn't redirect; HSTS only protects returning visitors). Confirmed with curl.
