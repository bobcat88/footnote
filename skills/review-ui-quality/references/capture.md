# Capture: safety, provenance and the script

## Before any screenshot of a real app
1. **Classify the surface.** A browser page is captured with Playwright. A native window (Win32, WinUI, a packaged Tauri or WebView2 app) needs a native capture route; a Playwright install in the project doesn't make one.
2. **Attaching to a running app?** Prove the tool can attach without launching, navigating, reloading or refreshing data. Otherwise review a dev build on fixtures.
3. **Make the data safe.** Synthetic fixtures, or redaction verified before capture (Playwright's `mask` option for known regions). Rendered pages can show account values or trigger requests. If neither is available, stop and say so.
4. **Bind each capture to what rendered.** Record per screenshot: file SHA-256, source commit, build or deployed URL, route, viewport, OS scale and browser zoom, theme, fixture id, state, timestamp.
5. **Fail closed.** Attach, safety or provenance unverified: mark the cell `NOT RUN`, don't judge it. Never infer behaviour from config files, old screenshots or the presence of a tool.

`deviceScaleFactor` emulates pixel density in the browser; it says nothing about native Windows DPI behaviour (see [windows-native.md](windows-native.md)).

## Capture and gate script (Playwright Test)
Adapt routes, states and themes. The size stays 1920×1080 unless the acceptance plan names more.

```ts
// qloop-ui/capture.spec.ts
import { test } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import fs from 'node:fs';

const screens = ['dashboard', 'projects', 'settings'];
const states  = ['empty', 'normal', 'heavy', 'error', 'loading'];
const themes  = ['dark', 'light'] as const;
const PASS    = process.env.QLOOP_PASS ?? '1';
const dir     = `qloop-ui/pass-${PASS}`;

for (const screen of screens) for (const state of states) for (const theme of themes)
  test(`${screen} ${state} ${theme}`, async ({ page }) => {
    const errors: string[] = [];
    page.on('console', m => m.type() === 'error' && errors.push(m.text()));
    page.on('pageerror', e => errors.push(String(e)));
    page.on('requestfailed', r => errors.push(`failed: ${r.url()}`));            // G1

    await page.setViewportSize({ width: 1920, height: 1080 });
    await page.emulateMedia({ colorScheme: theme });
    await page.goto(`/#/${screen}?mock=1&state=${state}`);
    await page.waitForLoadState('networkidle');

    fs.mkdirSync(`${dir}/screens`, { recursive: true });
    await page.screenshot({ path: `${dir}/screens/${screen}__${state}__1920x1080__${theme}.png` });

    const overflow = await page.evaluate(() => ({                                 // G2
      pageScroll: document.documentElement.scrollWidth > document.documentElement.clientWidth,
      clipped: [...document.querySelectorAll<HTMLElement>('*')].filter(el =>
        el.children.length === 0 && el.textContent?.trim() &&
        el.scrollWidth > el.clientWidth + 1 && !el.title).length,              // spilled or cut, no tooltip
    }));
    const placeholder = await page.evaluate(() =>                                 // G6
      /\b(lorem|undefined|null|NaN)\b|\[object Object\]/i.test(document.body.innerText));
    const axe = await new AxeBuilder({ page }).analyze();                         // G4
    const serious = axe.violations.filter(v => ['serious', 'critical'].includes(v.impact ?? ''));

    fs.appendFileSync(`${dir}/gates.jsonl`, JSON.stringify({
      screen, state, theme, errors, overflow, placeholder, axe: serious.map(v => v.id),
    }) + '\n');
  });
```

G5: tab through every focusable element and record order and focus visibility (`design-check` does the visibility part). G7: rerun a screen with `reducedMotion: 'reduce'` and check nothing moves longer than the limit. G8: compare computed styles of a sample of elements with the `DESIGN.md` tokens.

## Sources
[Playwright screenshots](https://playwright.dev/docs/screenshots) · [Playwright accessibility testing](https://playwright.dev/docs/accessibility-testing) · [emulateMedia](https://playwright.dev/docs/api/class-page#page-emulate-media) · [Tauri mocking](https://v2.tauri.app/develop/tests/mocking/) · [prefers-reduced-motion (MDN)](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion)
