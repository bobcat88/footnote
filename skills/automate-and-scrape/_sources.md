---
skill_hash: 64a4f205406ae0e8
reviewed: 2026-11-21
---
# automate-and-scrape: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| robots.txt matching: longest match, Allow on tie, 4xx = allowed, 5xx = disallowed | [RFC 9309](https://www.rfc-editor.org/rfc/rfc9309.html) |
| Retry-After | [RFC 9110 §10.2.3](https://www.rfc-editor.org/rfc/rfc9110.html#section-10.2.3) |
| 429 Too Many Requests | [RFC 6585 §4](https://www.rfc-editor.org/rfc/rfc6585.html#section-4) |
| Adaptive crawl delay | [Scrapy AutoThrottle](https://docs.scrapy.org/en/latest/topics/autothrottle.html) |
| Role-based locators, auto-waiting, storage state | [Playwright locators](https://playwright.dev/docs/locators) · [Playwright auth](https://playwright.dev/docs/auth) |
| Apps Script batching, quotas, locks | [Best practices](https://developers.google.com/apps-script/guides/support/best-practices) · [Quotas](https://developers.google.com/apps-script/guides/services/quotas) · [LockService](https://developers.google.com/apps-script/reference/lock/lock-service) |
| Open-data portals expose CKAN / DCAT / SPARQL | [data.europa.eu](https://data.europa.eu/en) · [CKAN API](https://docs.ckan.org/en/latest/api/) |

## Distilled from
High-install public browser-automation and crawling skills, surveyed 2026-11-21 (Apache-2.0); kept: snapshot refs over selectors, saving large output to files, reusing saved auth state, cheapest-path escalation. None of them covered robots.txt or rate limits, hence robots-check.
