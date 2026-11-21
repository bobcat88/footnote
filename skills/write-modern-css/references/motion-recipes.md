# Motion recipes

Native CSS for the transitions most interfaces need. All of them use the tokens below, move only `transform` and `opacity` (one marked exception), and fall back to a plain fade under reduced motion, because `--shift` and `--from-scale` collapse to nothing.

Pick by the element first, then the verb: a surface growing from its trigger is a dropdown; a centred surface with no anchor is a dialog; a message arriving from an edge is a toast; content changing height in place is an accordion; two views of the same item are a page morph.

```css
:root {
  --ease-out: cubic-bezier(0.2, 0, 0, 1);    /* entering: fast start, soft landing */
  --ease-in:  cubic-bezier(0.4, 0, 1, 1);    /* leaving: gets out of the way */
  --ease-std: cubic-bezier(0.4, 0, 0.2, 1);  /* on-screen changes */
  --dur-1: 100ms; --dur-2: 150ms; --dur-3: 200ms; --dur-4: 250ms;
  --shift: 6px; --from-scale: 0.96;
}
@media (prefers-reduced-motion: reduce) {
  :root { --shift: 0px; --from-scale: 1; }
}

/* Press: every button and clickable card */
.btn { transition: transform var(--dur-1) var(--ease-std), background-color var(--dur-1) var(--ease-std); }
.btn:active { transform: scale(0.97); }

/* Dropdown or menu on the popover API: grows from its trigger, enter slower than exit */
.menu[popover] {
  transform-origin: var(--origin, top left);
  opacity: 0;
  transform: translateY(calc(-1 * var(--shift))) scale(var(--from-scale));
  transition: opacity var(--dur-2) var(--ease-in), transform var(--dur-2) var(--ease-in),
              overlay var(--dur-2) allow-discrete, display var(--dur-2) allow-discrete;
}
.menu[popover]:popover-open {
  opacity: 1; transform: none;
  transition-timing-function: var(--ease-out); transition-duration: var(--dur-3);
}
@starting-style {
  .menu[popover]:popover-open { opacity: 0; transform: translateY(calc(-1 * var(--shift))) scale(var(--from-scale)); }
}

/* Dialog: native <dialog> with showModal(), so focus trapping and Escape come free */
dialog.sheet {
  opacity: 0; transform: scale(var(--from-scale));
  transition: opacity var(--dur-2) var(--ease-in), transform var(--dur-2) var(--ease-in),
              overlay var(--dur-2) allow-discrete, display var(--dur-2) allow-discrete;
}
dialog.sheet[open] { opacity: 1; transform: none; transition-timing-function: var(--ease-out); transition-duration: var(--dur-3); }
@starting-style { dialog.sheet[open] { opacity: 0; transform: scale(var(--from-scale)); } }
dialog.sheet::backdrop { background: rgb(0 0 0 / 0); transition: background-color var(--dur-3) var(--ease-std), overlay var(--dur-3) allow-discrete, display var(--dur-3) allow-discrete; }
dialog.sheet[open]::backdrop { background: rgb(0 0 0 / 0.4); }
@starting-style { dialog.sheet[open]::backdrop { background: rgb(0 0 0 / 0); } }

/* Toast: rises from the bottom edge; script adds .is-in, then .is-out before removal */
.toast { opacity: 0; transform: translateY(calc(2 * var(--shift))); transition: opacity var(--dur-4) var(--ease-out), transform var(--dur-4) var(--ease-out); }
.toast.is-in { opacity: 1; transform: none; }
.toast.is-out { opacity: 0; transform: translateY(var(--shift)); transition-timing-function: var(--ease-in); transition-duration: var(--dur-2); }

/* Tooltip: waits before showing, leaves at once */
.tip { opacity: 0; transform: translateY(calc(var(--shift) / 2)); transition: opacity var(--dur-1) var(--ease-in), transform var(--dur-1) var(--ease-in); }
.has-tip:is(:hover, :focus-visible) .tip { opacity: 1; transform: none; transition: opacity var(--dur-2) var(--ease-out) 400ms, transform var(--dur-2) var(--ease-out) 400ms; }

/* Accordion: the exception that animates layout, because the content below must move.
   Set `inert` on the panel while collapsed so its links can't take focus. */
.acc-panel { display: grid; grid-template-rows: 0fr; transition: grid-template-rows var(--dur-4) var(--ease-std); }
.acc-panel > * { overflow: hidden; min-height: 0; }
.acc-trigger[aria-expanded="true"] + .acc-panel { grid-template-rows: 1fr; }
.acc-trigger .chevron { transition: transform var(--dur-3) var(--ease-std); }
.acc-trigger[aria-expanded="true"] .chevron { transform: rotate(180deg); }

/* Sliding tab underline: script sets --x and --w (px) from the active tab's box */
.tabs { position: relative; }
.tabs .indicator {
  position: absolute; left: 0; bottom: 0; width: 1px; height: 2px; transform-origin: left;
  transform: translateX(var(--x, 0)) scaleX(var(--w, 0));
  transition: transform var(--dur-4) var(--ease-std);
}

/* Skeleton: slow pulse, then the content fades in over it */
.skeleton { background: var(--surface-raised); border-radius: 6px; animation: pulse 1.4s var(--ease-std) infinite alternate; }
@keyframes pulse { to { opacity: 0.55; } }
.loaded { animation: appear var(--dur-4) var(--ease-out); }
@keyframes appear { from { opacity: 0; } }
@media (prefers-reduced-motion: reduce) { .skeleton { animation: none; } }

/* Number that updates: no jitter, a small settle on change (script re-adds .tick) */
.num { font-variant-numeric: tabular-nums; }
.num.tick { animation: tick var(--dur-3) var(--ease-out); }
@keyframes tick { from { opacity: 0.4; transform: translateY(calc(var(--shift) / -2)); } }

/* Page or list-to-detail morph: view transitions */
@view-transition { navigation: auto; }                 /* same-origin page loads */
::view-transition-group(*) { animation-duration: var(--dur-4); animation-timing-function: var(--ease-std); }
@media (prefers-reduced-motion: reduce) {
  ::view-transition-group(*), ::view-transition-old(*), ::view-transition-new(*) { animation: none; }
}
```

```js
// In-page state change with a morph, where supported. Give the moving element the same
// view-transition-name in both states (unique per page).
const swap = (update) => document.startViewTransition ? document.startViewTransition(update) : update();
```

## Notes
- `@starting-style`, `transition-behavior: allow-discrete` and the popover API let entry and exit animate without script timers. In browsers without them the element still opens and closes, just without the animation.
- Exits are shorter than entries: the user already decided.
- A celebration (confetti, a bouncing check) belongs only where the outcome isn't already visible.
- Check support in [design-trends.md](design-trends.md) (Table B) before relying on a newer feature.

## Sources
[@starting-style](https://developer.mozilla.org/en-US/docs/Web/CSS/@starting-style) · [transition-behavior](https://developer.mozilla.org/en-US/docs/Web/CSS/transition-behavior) · [Popover API](https://developer.mozilla.org/en-US/docs/Web/API/Popover_API) · [&lt;dialog&gt;](https://developer.mozilla.org/en-US/docs/Web/HTML/Element/dialog) · [View Transition API](https://developer.mozilla.org/en-US/docs/Web/API/View_Transition_API) · [prefers-reduced-motion](https://developer.mozilla.org/en-US/docs/Web/CSS/@media/prefers-reduced-motion) · [Entry and exit animations (developer.chrome.com)](https://developer.chrome.com/blog/entry-exit-animations) · [WCAG 2.3.3](https://www.w3.org/WAI/WCAG22/Understanding/animation-from-interactions.html)
