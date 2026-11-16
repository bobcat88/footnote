# Footnote Alias evaluation, v1

Written 16 November 2026, before any detector has run. The decision rule is the one proposed in the
[desk study](../../docs/research/ALIAS-DESK-STUDY.md) and confirmed by Johan the same day.

## Data: `bench/datasets/alias-v1/`

Built by [alias_dataset.py](alias_dataset.py) (seed 20261116): 312 French and English business texts (CRM notes, support tickets,
HR messages, invoices, security alerts, e-mails), 26 templates per language, 6 fillings each. Every value is fictional: common
names, RFC 2606 e-mail domains, documentation IP ranges, fiction phone ranges where they exist, and French identifiers with valid
check digits (NIR key, SIREN/SIRET Luhn, IBAN mod 97, card Luhn), which the script verifies. Spans are recorded while the text is
assembled. Each text also carries look-alikes that must stay untouched: order numbers, 16-digit tracking numbers that fail Luhn,
amounts, versions, company names and meeting dates.

| Half | Texts | Templates | Use |
|---|---:|---|---|
| dev | 156 | 13 per language | choose detectors and thresholds, as often as needed |
| heldout | 156 | the other 13 per language | one confirmation run per decision |

The split is by template (no template in both halves), redrawn until every label appears in both.
SHA-256: `dev.jsonl` a0f0c22719dc64f869f5578150e087b723c8a642946b6946b79105c5fd515461, `heldout.jsonl`
9b8cb55861a4a307e8f3a7ba62ac9adc328179dafe00aa6141a695d96e182d9b (LF line endings, as git stores them; the script regenerates the same bytes).

## Measures

- **Found:** a gold value counts as found when every letter and digit of it lies inside predicted spans, whatever label they
  carry (for pseudonymisation, masking it under the wrong type still protects it). Partial cover counts as missed: half an IBAN
  is a leak.
- **Recall** per label and per group; **precision** = predicted spans that overlap a gold value / all predicted spans;
  **look-alikes hit** = look-alikes overlapped by a prediction / all look-alikes. Precision and look-alikes are reported, not
  judged, in v1.
- Per language, and speed per text on the CPU.

## Decision rule (held-out, one run)

- **Direct identifiers** (EMAIL, PHONE, IBAN, CARD, NIR, SIREN, SIRET, IP): recall **≥ 0.95** over the group.
- **Names** (PERSON, first names alone included): recall **≥ 0.90**.
- ADDRESS and DOB: reported.

**Choice on dev:** for names and addresses, the model and threshold with the highest dev PERSON recall among those with overall
precision ≥ 0.5; at equal recall (within 0.01), the faster one. Candidates: GLiNER2-PII, urchade/gliner_multi_pii-v1, OpenAI
Privacy Filter (reference). The direct identifiers go to the Rust checksum rules first; a model only adds to them.

## Before any candidate runs

The model candidates need third-party Python packages (`gliner`, `gliner2`, `transformers`). Each gets the usual security review
first: real upstream repository, code read without running it, no install hooks.

## Result 1 (16 November 2026): the Rust rules cover the direct identifiers

Detector: `footnote alias-scan` (crate `footnote-alias`, build `--features alias`), rules with check digits only, no model. Scorer:
[alias_eval.py](alias_eval.py). Developed on dev, then one held-out run.

| Half | Direct identifiers recall | FR / EN | Names | Precision | Look-alikes hit | Time per text |
|---|---:|---|---:|---:|---|---:|
| dev | 1.000 | 1.000 / 1.000 | 0.000 | 1.000 | 0/144 | 0.10 ms |
| **held-out** | **1.000** | 1.000 / 1.000 | 0.000 | 1.000 | 0/156 | 0.09 ms |

- **Direct identifiers: pass** (rule ≥ 0.95), every label at 1.000 on held-out, nothing flagged that was not personal data.
- **Names: not yet** (0.000): rules do not guess names by design; the model stage decides that part.
- Read with care: the same author wrote the generator and the rules, so the formats match by construction. The useful evidence is
  that check digits reject every look-alike (Luhn-failing tracking numbers included) at no measurable cost; real texts with
  unusual formats (an IBAN split across lines, a phone with a missing digit) are the open risk.

## Result 2 (16 November 2026): rules + OpenAI Privacy Filter pass both parts

Model: [openai/privacy-filter](https://huggingface.co/openai/privacy-filter) (Apache-2.0), ONNX 4-bit (`onnx/model_q4.onnx`,
902 MB with data), revision `7ffa9a04`, run with the lab's existing onnxruntime and tokenizers (no new package), 4 threads, decoded
with the card's constrained BIOES path at the default operating point ([alias_pf.py](alias_pf.py)). Only candidate run so far.

Dev, choosing the combination (any number of runs):

| Detector | Direct ids | Names (FR / EN) | Precision | Look-alikes hit |
|---|---:|---|---:|---|
| model alone | 0.967 | 0.978 (0.962 / 1.000) | 0.846 | 60/144 |
| rules + model | 1.000 | 0.978 | 0.893 | 60/144 |
| **rules + model without its account guesses** ("alias") | 1.000 | 0.978 | 0.940 | 26/144 |

The model's `account_number` label took order numbers (26/30) and Luhn-failing tracking numbers (12/12); the rules already own
account-like identifiers with check digits, so "alias" drops that label. Remaining dev look-alikes: meeting dates read as dates
(17/24) and some amounts read as phones.

**Held-out, one run, "alias":**

| Direct identifiers | Names | Names FR / EN | Precision | Look-alikes hit | Time per text (CPU) |
|---:|---:|---|---:|---|---:|
| **1.000** | **0.913** | 0.917 / 0.910 | 0.917 | 33/156 | 63 ms model + 0.1 ms rules |

- **Direct identifiers: pass** (≥ 0.95). **Names: pass** (≥ 0.90), narrowly.
- 11 of the 12 missed names are first names alone in a greeting or reminder ("Merci Camille", "Reminder: James"); one full name
  was missed. Over-masking: meeting dates and some amounts.
- Next candidate if names need a margin: GLiNER2-PII (best recall on SPY). This held-out half is now spent for that question;
  a new comparison needs a v2 set.

## Limits

One author wrote the templates and the data is synthetic: realistic structure, but none of the mess of real text (typos, signatures,
forwarded threads, OCR). Six fillings per template inflate the effective sample less than 312 suggests. A v2 should add
real-looking texts written by someone else.
