# Studio: config, tokens and reading the lock

## studio.json
```json
{
  "project": "Quorra",
  "kind": "page",
  "subtitle": "Three directions for a voice-agent test platform. Pick one, tune it, lock it.",
  "preview": { "nav": [], "headline": "", "lede": "", "cta": "", "kpis": [], "rows": [], "series": [], "features": [] },
  "directions": [
    { "id": "D1", "name": "Signal", "line": "one-line idea", "refs": ["reference names"],
      "image": "directions/D1/shot-top.png", "tokens": { "...": "see below" } }
  ]
}
```
- `kind`: `app` or `page` (sets the live preview layout). `preview` is optional copy for that preview; the example shows every key it reads (`nav`, `headline`, `lede`, `cta`, `cta2`, `badge`, `caption`, `kpis`, `rows`, `series`, `features`).
- 2 to 4 `directions`. `image` paths resolve from the config's folder; images over 350 KB are shrunk with ffmpeg when it's installed, then embedded, so `studio.html` is one file.
- Full example: `scripts/studio/examples/quorra.json`.

## tokens.json
| Key | Values |
|---|---|
| `theme` | `light` or `dark` |
| `accent` | `#rrggbb` |
| `warmth` | −1 (cool) to 1 (warm) neutrals |
| `tint` | 0–6, how much accent tints the neutrals |
| `radius` | 0–28 px |
| `density` | 0.8–1.3 (spacing multiplier) |
| `scale` | 1.125–1.5 (type ratio) |
| `display`, `body` | Hanken Grotesk, Geist, Instrument Sans, Manrope, Figtree, Plus Jakarta Sans, Sora, Bricolage Grotesque, DM Sans, Outfit, Montserrat, Fraunces, Newsreader, Source Serif 4, JetBrains Mono |
| `dweight` | 100–900, display weight |
| `motion` | `none`, `subtle`, `lively` |

`build.py` rejects a config whose tokens are out of range. `node scripts/studio/selfcheck.mjs` tests the page's colour maths (WCAG contrast, OKLCH conversion).

## Reading the lock
The page saves a snapshot: `direction`, `locked`, `notes`, `tokens`, the derived `palette` (hex), `type` scale, `spacing`, `contrast` results, `kind`, `updatedAt`.
- **Host with a page database** (a Claude artifact published with the `db` capability): it saves on every change to collection `choice`, document `current`. Read that document.
- **Anywhere else** (opened as a local file): pressing Lock downloads `choice.json`. Ask the user to drop it in `.design-studio/`, then read it.

## Contrast thresholds the studio checks
Text 4.5:1, large text and UI parts 3:1, per [WCAG 2.2 SC 1.4.3](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html) and [1.4.11](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html). Colours are built in [OKLCH](https://www.w3.org/TR/css-color-4/#ok-lab) so equal steps look equal.
