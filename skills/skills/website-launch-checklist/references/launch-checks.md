# Conditional website launch checks



## France and EU applicability

Identify the publisher's legal form and activity before selecting required notices. Use the current [Service Public business-site guidance](https://entreprendre.service-public.gouv.fr/vosdroits/F37351) and its branch for the actual business category; do not publish a generic template containing invented registry details or personal addresses. For sales, confirm the applicable consumer/business information and terms instead of assuming every withdrawal rule applies to every transaction.

Map personal-data collection, purpose, lawful basis, recipients, retention and rights information with the responsible owner. Verify provider roles and processing agreements where required under [GDPR Article 28](https://www.cnil.fr/fr/reglement-europeen-protection-donnees/chapitre4); a list of vendors alone is not compliance evidence.

Under the [CNIL cookie guidance](https://www.cnil.fr/fr/cookies-et-autres-traceurs/regles/cookies/FAQ), non-exempt trackers require prior consent and accepting/refusing should be comparably simple. An analytics product name or a banner is not proof of exemption. Inspect actual configuration, cookies and requests before a choice, after refusal and after withdrawal; assess exemptions against current conditions.

The [DGCCRF accessibility guidance](https://www.economie.gouv.fr/dgccrf/les-fiches-pratiques/la-nouvelle-directive-europeenne-accessibilite-pour-des-produits-et-des-services-accessibles-aux-personnes-en-situation) identifies covered product/service categories and exemptions. Record the site's applicable regime and evidence, including any claimed exemption; do not treat a general small-business label as automatic exemption from all accessibility duties. Applicable legal statements and conformance obligations require a scoped review.

## Security and operational readiness

Local review policy: verify HTTPS behavior, a valid certificate and renewal ownership; least-privilege account access; no shipped credentials; server-side input validation; abuse controls proportionate to exposed forms; dependencies and deployment settings. A grep match is a lead, not proof of a credential, and no match is not proof of absence.

Use a [Content Security Policy](https://developer.mozilla.org/en-US/docs/Web/HTTP/Guides/CSP) matching the actual content sources; test before enforcing changes that could break the site. A header scanner grade is supporting evidence, not an application security verdict. Avoid broad unsafe allowances to make a test pass.

Confirm backup/restore where persistent state exists, error reporting, availability alerts, domain/certificate ownership and rollback responsibilities. For sending domains, verify the current provider authentication requirements and test actual message headers; do not alter DNS or send mail without task authorization. A static brochure site need not acquire a database backup service simply to satisfy a checklist.

## Accessibility

Use the project's agreed [WCAG 2.2](https://www.w3.org/TR/WCAG22/) target and applicable legal standard. Check keyboard access, visible focus, traps, labels, errors, meaningful alternatives, zoom/reflow and the relevant input methods. Automated scans supplement manual testing; they do not establish complete conformance.

WCAG contrast criteria generally require 4.5:1 for normal text and 3:1 for large text, with defined exceptions; relevant non-text UI contrast has its own criterion. [Target size 2.5.8](https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html) uses 24 by 24 CSS pixels with spacing and other exceptions. Do not rewrite that as an unconditional physical-pixel size rule. Preserve accessible behavior when virtualizing or deferring content.

## Search and sharing

Check indexing intent, canonical URLs, page titles/descriptions, language metadata, actual response statuses and broken links. A private/staging page should not be made indexable just to pass SEO tooling. Use a sitemap where useful; [Google's sitemap guidance](https://developers.google.com/search/docs/crawling-indexing/sitemaps/overview) does not guarantee discovery or indexing. Follow [Google's structured-data content policies](https://developers.google.com/search/docs/appearance/structured-data/sd-policies) rather than inventing markup that the visible content does not support.

Validate sharing metadata and image previews on the intended platforms. No fixed Lighthouse SEO score or social-image size is a universal release requirement; use actual acceptance criteria.

## Performance and evidence

Separate lab and field data. Define page/resource budgets for this product, including whether bytes mean transferred or decoded content. Measure supported device/network conditions, warm/cold paths and actual interactions. Retain build/input IDs, raw samples, errors, test scope and uncertainty. Do not predict conversion uplift from a third-party case study.

Test consent-dependent resource behavior, usable loading/error states and primary business actions. Collect field metrics after launch where the data and privacy basis permit it. Missing field traffic remains missing evidence, not a fabricated pass.

## Review status

Primary sources retrieved 2026-11-17. Technical and applicability guidance is bounded to the claims above; this reference is not a legal certification of a particular site. Independent source coverage and parent gap closure are recorded in the project source-review ledger; behavioral benefit remains experimental.
