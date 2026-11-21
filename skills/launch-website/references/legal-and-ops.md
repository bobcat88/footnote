# Legal, privacy and operations

A map of what to check, not legal advice: confirm each point against current official guidance for the actual business.

## EU / France
- **Publisher notice:** required details depend on legal form + activity → [Service Public guidance](https://entreprendre.service-public.gouv.fr/vosdroits/F37351). Never invented registry numbers or a private home address
- **Selling online:** consumer info / terms / withdrawal rules depend on B2C vs B2B; don't copy a generic page
- **Personal data:** per form, account, analytics tool → purpose / lawful basis / recipients / retention / rights. Processors need a processing agreement (GDPR Art. 28); a vendor list isn't evidence
- **Cookies:** non-exempt trackers need prior consent / refuse as easy as accept. Inspect real cookies + requests before choice, after refusal, after withdrawal; a banner isn't proof
- **Accessibility law:** European Accessibility Act covers listed services (e-commerce included) with defined exemptions; record the regime and evidence for any exemption

## Email from the site's domain
SPF / DKIM / DMARC per the provider's current rules; read the headers of a real test message. No DNS change or sending without authorisation.

## Ops handover
| Item | Evidence |
|---|---|
| Domain + certificate | owner / auto-renew on / expiry date |
| Hosting + DNS access | named accounts / least privilege / no shared passwords |
| Backups | only where there's state: schedule / location / one restore tested |
| Monitoring | uptime alert to a person / error reporting on |
| Rollback | how, and who can |
| Analytics | owner / consent basis / what's measured |
