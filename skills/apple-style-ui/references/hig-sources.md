# Apple UI/UX research sources

Initial research: 2026-10-31; WCAG criterion levels, target-size guidance, Apple SF font licensing, notification/Live Activity privacy guidance, and WidgetKit rendering-mode scope re-verified online: 2026-11-02. Primary-source document and WWDC transcript review; this is not a complete visual teardown of every Apple product, and it does not claim access to Apple’s internal design process. Refresh sources and platform-specific API details before each implementation.

## Primary references

1. **Apple Human Interface Guidelines — Materials**  
   https://developer.apple.com/design/human-interface-guidelines/materials  
   Machine-readable text: https://developer.apple.com/tutorials/data/design/human-interface-guidelines/materials.md  
   Establishes Liquid Glass as a functional layer for controls/navigation; advises against using it in the content layer, recommends sparing custom use, distinguishes regular and clear variants, and calls for legibility/context adaptation.

2. **Apple — Liquid Glass technology overview**  
   https://developer.apple.com/documentation/technologyoverviews/liquid-glass  
   Describes material qualities and cross-platform design intent. Use system components/APIs where available; do not equate CSS blur with native material behavior.

3. **Apple — Adopting Liquid Glass**  
   https://developer.apple.com/documentation/technologyoverviews/adopting-liquid-glass  
   Adoption guidance for platform components and hierarchy. Check API availability and platform version in implementation.

4. **WWDC25 — Build an AppKit app with the new design (session 310)**  
   https://developer.apple.com/videos/play/wwdc2025/310/  
   Apple-hosted session transcript reviewed. It emphasizes floating toolbar/sidebar hierarchy, scroll-edge effects, system components, restricting custom glass to key controls, content legibility, and grouping nearby glass elements for coherent appearance.

5. **Apple Human Interface Guidelines — Widgets**  
   https://developer.apple.com/design/human-interface-guidelines/widgets  
   Machine-readable text: https://developer.apple.com/tutorials/data/design/human-interface-guidelines/widgets.md  
   Widgets present timely, glanceable information and focused actions; design depends on size, context, rendering mode and appearance; avoid app-like complexity and deep-link to relevant app content.

6. **Apple Human Interface Guidelines — Motion**  
   https://developer.apple.com/design/human-interface-guidelines/motion  
   Machine-readable text: https://developer.apple.com/tutorials/data/design/human-interface-guidelines/motion.md  
   Motion should support the experience, be optional, realistic, brief, precise, interruptible/cancelable, and not slow frequent app interactions. System motion can adapt to input/accessibility settings.

7. **Apple Human Interface Guidelines — Typography**  
   https://developer.apple.com/design/human-interface-guidelines/typography  
   Machine-readable text: https://developer.apple.com/tutorials/data/design/human-interface-guidelines/typography.md  
   Covers system fonts, variable/optical sizing, semantic text styles, accessibility text scaling, platform default/minimum sizes, and adaptive layout.
   Apple Fonts and San Francisco font terms: https://developer.apple.com/fonts/ — the page publishes a separate SF-font license; its currently displayed text restricts downloaded SF files to permitted UI mockups and prohibits embedding. Use system-font APIs for product UI; re-check the live license and target OS.

8. **Apple Human Interface Guidelines — Accessibility**  
    https://developer.apple.com/design/human-interface-guidelines/accessibility  
    Machine-readable text: https://developer.apple.com/tutorials/data/design/human-interface-guidelines/accessibility.md  
    Covers perceptibility, adaptability, larger text, platform sizes, contrast, non-color state, and assistive technologies. The current HIG table lists control target sizes in points: iOS/iPadOS 44×44 default / 28×28 minimum; macOS 28×28 / 20×20; tvOS 66×66 / 56×56; visionOS 60×60 / 28×28; watchOS 44×44 / 28×28. Pair Apple’s guidance with current WCAG for web requirements; do not convert these point values into CSS-pixel conformance claims.

9. **Apple Human Interface Guidelines — Layout**  
   https://developer.apple.com/design/human-interface-guidelines/layout  
   Machine-readable text: https://developer.apple.com/tutorials/data/design/human-interface-guidelines/layout.md  
   Covers hierarchy, alignment, grouping, progressive disclosure, adaptive windows, size classes, localization/RTL, and Dynamic Type.

10. **Apple Human Interface Guidelines — Color**  
    https://developer.apple.com/design/human-interface-guidelines/color  
    Machine-readable text: https://developer.apple.com/tutorials/data/design/human-interface-guidelines/color.md  
    Covers semantic/system colors, appearance/high-contrast variation, color consistency and Liquid Glass tinting.

11. **Apple Human Interface Guidelines — SF Symbols**  
    https://developer.apple.com/design/human-interface-guidelines/sf-symbols  
    Machine-readable text: https://developer.apple.com/tutorials/data/design/human-interface-guidelines/sf-symbols.md  
    Covers matching system font weights, symbol rendering modes, availability/fallback and usage terms. Verify the current license before use outside Apple-platform apps.

12. **Apple Human Interface Guidelines — Windows**  
    https://developer.apple.com/design/human-interface-guidelines/windows  
    Machine-readable text: https://developer.apple.com/tutorials/data/design/human-interface-guidelines/windows.md  
    Recommends adaptive window sizing and system-defined frames/controls rather than custom imitation of window chrome.

## Research conclusions and limits

- Strongest verified pattern: a crisp separation between content and functional controls; material and motion reinforce hierarchy and continuity instead of decorating every component.
- Current HIG guidance is platform-aware, adaptive, and accessibility-sensitive. Fixed CSS recipes, generic 8pt spacing claims, one universal Apple type scale, or a single “Apple easing curve” are not supported by this review.
- On 2026-11-02, the Apple HIG Materials page was visually inspected at 1920×1080: its introductory diagram and prose reinforce the functional-layer/content-layer distinction; this is documentation artwork, not an app workflow. The Widgets article shell did not render in the browser, but its machine-readable HIG text loaded and was reviewed; widget illustrations were not visually inspected. This remains a selective research pass, not a complete visual teardown. For a target redesign, inspect actual product examples at a comparable task/state/size before making fidelity claims.
- Liquid Glass is system-managed optical behavior (including adaptive appearance and interaction). CSS `backdrop-filter` can approximate translucency only; it does not reproduce native refraction, sampling, grouping, system settings, or framework transitions.
- Apple’s HIG and WCAG overlap but are not the same specification. Record which requirement comes from which source and apply platform-specific text/target guidance rather than flattening everything into one number. WCAG 2.2 SC 2.5.8 is the AA minimum (24×24 CSS px, with exceptions); SC 2.5.5 is the 44×44 CSS px enhanced target at AAA. See https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html and https://www.w3.org/WAI/WCAG22/Understanding/target-size-enhanced.html.

## WCAG 2.2 references

| Criterion | Level | Official explanation |
|---|---|---|
| 1.4.3 Contrast (Minimum) | AA | https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html |
| 1.4.11 Non-text Contrast | AA | https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html |
| 2.1.1 Keyboard | A | https://www.w3.org/WAI/WCAG22/Understanding/keyboard.html |
| 2.4.7 Focus Visible | AA | https://www.w3.org/WAI/WCAG22/Understanding/focus-visible.html |
| 2.4.11 Focus Not Obscured (Minimum) | AA | https://www.w3.org/WAI/WCAG22/Understanding/focus-not-obscured-minimum.html |
| 2.4.13 Focus Appearance | AAA | https://www.w3.org/WAI/WCAG22/Understanding/focus-appearance.html |
| 2.5.8 Target Size (Minimum) | AA | https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html |
| 2.5.5 Target Size (Enhanced) | AAA | https://www.w3.org/WAI/WCAG22/Understanding/target-size-enhanced.html |

## Privacy and surface-exposure sources

- **Apple Human Interface Guidelines — Notifications**: https://developer.apple.com/design/human-interface-guidelines/notifications and machine-readable text https://developer.apple.com/tutorials/data/design/human-interface-guidelines/notifications.md. Explicitly advises against including sensitive, personal, or confidential information in notifications because others may see them.
- **Apple Human Interface Guidelines — Live Activities**: https://developer.apple.com/design/human-interface-guidelines/live-activities and machine-readable text https://developer.apple.com/tutorials/data/design/human-interface-guidelines/live-activities.md. Warns that Live Activities may be visible to casual observers and recommends innocuous summaries or redaction for sensitive information.

## WidgetKit rendering references

- **WidgetRenderingMode API**: https://developer.apple.com/documentation/widgetkit/widgetrenderingmode.md. Defines `fullColor`, `accented`, and `vibrant`; available modes and rendering behavior are context-dependent (including platform/OS, API version, widget family, destination, and user settings). Verify the specific destination/OS against current API documentation.
- **Optimizing your widget for accented rendering mode and Liquid Glass**: https://developer.apple.com/documentation/widgetkit/optimizing-your-widget-for-accented-rendering-mode-and-liquid-glass.md. Explains how current tinted/clear contexts affect widget rendering and how to adapt layout and assets for each mode.
