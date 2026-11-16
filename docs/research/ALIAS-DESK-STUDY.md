# Footnote Alias: desk study

16 November 2026. Desk work only: model cards, papers and source code read, nothing installed or run. Product goals and the legal
anchor are in [BACKLOG.md, Footnote Alias design notes](../roadmap/BACKLOG.md#footnote-alias-design-notes).

## What Alias has to do

Find personal data in French and English text before it reaches an AI, replace it with stable pseudonyms keyed with the company's
secret, and put the real values back in the answer. Detection quality decides the product: a missed name is a leak, and a false
alarm on every line makes people turn it off.

## Two kinds of personal data, two detectors

| Kind | Examples | Best detector |
|---|---|---|
| Identifiers with a fixed format | email, phone, IBAN, card number, NIR (social security), SIREN/SIRET, IP address, postal code | Rules with checksum validation: IBAN and NIR use mod 97, card numbers and SIREN/SIRET use Luhn. Close to 100 % precision, a few microseconds, no model. |
| Free-form mentions | names, street addresses, dates of birth in prose, usernames | A model. No rule finds "Marie Dupont" or "chez ma voisine du 3e". |

So the engine is a set of rules plus a small model, and the model runs only for the free-form kinds.

## Candidates

### Models (free-form kinds)

| Model | Size | Licence | Languages | Evidence | ONNX |
|---|---|---|---|---|---|
| [fastino/gliner2-privacy-filter-PII-multi](https://huggingface.co/fastino/gliner2-privacy-filter-PII-multi) (GLiNER2-PII) | 205M | Apache-2.0 | en fr es de it pt nl | Best average F1 on SPY, 0.477 (legal 0.473, medical 0.480), recall 0.72, precision 0.35 ([arXiv 2605.09973](https://arxiv.org/abs/2605.09973)). Over-predicts names. 42 labels chosen at query time. Trained only on 4,910 synthetic texts. | Not official; community exports exist ([okasi](https://huggingface.co/okasi/gliner2-privacy-filter-pii-multi-onnx), with q4/q8 SPY scores), unverified |
| [openai/privacy-filter](https://huggingface.co/openai/privacy-filter) | 1.5B (50M active) | Apache-2.0 | No language list; OpenAI warns quality varies outside the training distribution | SPY F1 0.380 in the GLiNER2-PII paper (high recall, low precision) | Yes |
| [urchade/gliner_multi_pii-v1](https://huggingface.co/urchade/gliner_multi_pii-v1) | ~200M | Apache-2.0 | en fr de es pt it | SPY F1 0.398, the most precise of the four (0.47 to 0.52) | No (export documented upstream) |
| [OpenMed-PII-French-EuroMed-Large-210M](https://huggingface.co/OpenMed/OpenMed-PII-French-EuroMed-Large-210M-v1) | 210M | Apache-2.0 | fr | Reports micro F1 0.976 on the French part of AI4Privacy, its own training distribution; 52 downloads | No |
| nvidia/gliner-PII | | "other" | en | SPY F1 0.400 | No |

No French-only benchmark compares these. SPY is legal and medical English-style text; every French figure above is the author's own.

### Frameworks

| Tool | Status | Why not as is |
|---|---|---|
| [Microsoft Presidio](https://github.com/microsoft/presidio) (MIT, active) | The reference architecture: recognizers (regex + checksum + context words) plus an NER model, then anonymizer operators. | Python. Its country-specific recognizers cover 18 countries (UK, Spain, Italy, Germany, Poland…) and **not France**: no NIR, SIREN/SIRET or French phone format. Reversible pseudonymisation only through LangChain's experimental wrapper or encryption. |
| [LLM Guard](https://github.com/protectai/llm-guard) (MIT) | Anonymize/Deanonymize scanners with a vault, on Presidio plus ai4privacy fine-tunes (mDeBERTa for multilingual). | **Archived** on GitHub. Python. |
| [gline-rs](https://docs.rs/crate/gline-rs/latest) (Rust, 1.0) | GLiNER inference on `ort` + `tokenizers` + `ndarray`, the same stack as Footnote's encoders; span and token modes. | Supports GLiNER v1 models. GLiNER2 changed the architecture, so GLiNER2-PII support must be checked before relying on it. |

### French test data

- **AI4Privacy pii-masking-300k/400k/500k**: include French, but under a custom licence ("other"); read it before any use, commercial
  use may be excluded.
- **TypicaAI pii-masking-60k_fr**: French, no licence declared, so not usable as is.
- Neither replaces a hand-made set of real-looking French business text (emails, CRM notes, support tickets, HR messages),
  which is what Alias will see.

## Recommendation

1. **Rules first, in Rust, France-specific.** Email, French and international phone, IBAN (mod 97), card (Luhn), NIR (key mod 97,
   Corsica 2A/2B), SIREN/SIRET (Luhn), IP, French postal code with a city nearby. This is the part no tool ships for France, and
   it is cheap, deterministic and testable.
2. **One model for names and addresses**, chosen by a measurement on French text, not by the cards. Shortlist: GLiNER2-PII (best
   recall), urchade multi PII (best precision), and OpenAI Privacy Filter as an outside reference. Run through `ort` like the
   embedders. A GLiNER v1 model works with gline-rs today; GLiNER2 needs a check or our own decoder.
3. **Pseudonyms, not masks**: `Marie Dupont` becomes `Personne_7f3a` (HMAC of the value with the company key), the same everywhere,
   so the AI can still tell people apart and the answer can be restored. The mapping stays on the machine (design notes).
4. **Before building, a French evaluation set and a written decision rule.** Draft rule, for Johan to confirm: recall on direct
   identifiers ≥ 0.95 overall and ≥ 0.90 on names, measured on a hand-annotated French + English business set of at least
   300 texts, with precision reported; the model is chosen on a development half and confirmed once on the other half.

## Open questions

- Which labels a company wants on by default (a city alone is rarely personal; a city plus a birth date may be).
- Where Alias sits: a filter inside Footnote's MCP and hook (protects what agents send), a local proxy in front of the AI API
  (protects every call), or both. The proxy covers more but is a network component that IT security will review.
- Latency budget for underline-while-typing: rules are instant; a 200M model on CPU needs measuring on real lengths.
