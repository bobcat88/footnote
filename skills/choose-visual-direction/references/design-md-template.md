# DESIGN.md and PRODUCT.md templates

Copy both into the project root. Fill every section. Write "N/A - reason" rather than deleting a section. Date every decision.

---

## File 1: PRODUCT.md

```markdown
# PRODUCT
Last reviewed: YYYY-MM-DD · Owner: <name>

## 1. Users and context
- Primary user: <role>, <experience level>
- Context of use: <desktop/web>, <screen size and window minimum>, <expert or occasional>, <language(s)>

## 2. Top three tasks
1. <task> — success looks like <observable result>
2. <task> — success looks like <observable result>
3. <task> — success looks like <observable result>

## 3. Tone
- Is: <adjective>, <adjective>, <adjective>
- Is not: <anti-adjective>, <anti-adjective>, <anti-adjective>

## 4. Success metric
<one measurable outcome, for example "time to first exported report under 60 s">

## 5. Non-goals
- <what this product will deliberately not do>

## 6. Accessibility floor
WCAG AA unless stated. Keyboard-complete. Reduced motion honoured. Exceptions: <none or list>
```

---

## File 2: DESIGN.md

```markdown
# DESIGN
Last reviewed: YYYY-MM-DD · Approved by: <human name> on YYYY-MM-DD · Status: draft | approved

## 1. Tokens source
Hand-written | extracted from <URL> with <tool> on <date> (draft until steps 1-2 and 11 pass)

## 2. References
| # | Product or source | URL | Date viewed | Evidence type | Borrowed | Rejected and why |
|---|---|---|---|---|---|---|
| 1 | <name> | <url> | YYYY-MM-DD | live app / demo / official capture / marketing / concept | <pattern> | <reason> |
| 2 | <name> | <url> | YYYY-MM-DD | <type> | <pattern> | <reason> |
| 3 | <name> | <url> | YYYY-MM-DD | <type> | <pattern> | <reason> |
Blocked access: <list, or none>

## 3. Color roles
| Role | Light | Dark | Contrast vs surface (light / dark) |
|---|---|---|---|
| surface | #... | #... | - |
| surface-raised | #... | #... | - |
| text-primary | #... | #... | ≥ 4.5:1 / ≥ 4.5:1 |
| text-muted | #... | #... | ≥ 4.5:1 / ≥ 4.5:1 |
| accent | #... | #... | ≥ 3:1 (UI) |
| success | #... | #... | <measured> |
| warning | #... | #... | <measured> |
| danger | #... | #... | <measured> |
| focus | #... | #... | ≥ 3:1 |

## 4. Typography
- Families: <primary>, <optional second, two max>
- Scale:
| Name | Size | Line height | Weight | Use |
|---|---|---|---|---|
| caption | | | | |
| body | | | | |
| title | | | | |
| display | | | | |
- Tabular numerals: <yes/no, where>

## 5. Space and shape
- Base unit: <4 or 8> px. Allowed steps: <list>
- Radii (max two): <values>
- Borders and elevation: <one rule>
- Icon set: <name>, stroke width <value>

## 6. Layout and density
- Grid and max content width: <value>
- Minimum window size: <W × H>
- Resize behaviour: <reflow rule, what collapses first>
- Density per context: <expert tools: ..., marketing: ...>

## 7. Components and states
| Component | Default | Hover | Active | Focus | Disabled | Loading | Error |
|---|---|---|---|---|---|---|---|
| Button | | | | | | | |
| Input | | | | | | | |
| Table row | | | | | | | |
<Anything not in this table needs a decision-log entry first.>

## 8. Screen states
| Screen type | Empty copy and action | Loading | Error copy and recovery | Partial | Heavy data |
|---|---|---|---|---|---|
| List | | | | | |
| Detail | | | | | |
| Form | | | | | |
Long strings: <ellipsis | wrap | tooltip, chosen per component>

## 9. Motion
- Duration range: <e.g. 120-250 ms>
- Easing tokens: <name → curve>
- Reduced motion: <what replaces each animation>
- May animate: <list>
- Must never animate: <list>

## 10. Keyboard and desktop
- Shortcuts: <list>
- Focus order and Escape behaviour: <rule>
- Drag regions and native menus: <rule>
- OS theme: <follows system | fixed>

## 11. Banned list (minimum five)
<!-- Shared slop patterns live in review-ui-quality/references/slop-patterns.md and apply through G3. List only this project's own bans here. -->
- Never <item>
- Never <item>
- Never <item>
- Never <item>
- Never <item>

## 12. Decision log
| Date | Decision | Reason | Rejected alternative |
|---|---|---|---|
| YYYY-MM-DD | <choice> | <reason> | <alternative> |
```

---

## Completeness check (run before any UI build)
- [ ] PRODUCT.md: users, top three tasks, tone, success metric filled
- [ ] DESIGN.md: sections 2 to 12 filled, all dated
- [ ] Every color role has light and dark values and a measured contrast
- [ ] Type scale has named steps; spacing unit and steps defined
- [ ] Minimum window size defined
- [ ] Banned list has at least five items
- [ ] Decision log has at least one entry
- [ ] Status is `approved` by a human, or the build is marked draft

## How the gates use these files
- G8 (token use) checks computed styles against section 3 to 5.
- G3 and G4 check contrast results recorded in section 3.
- G3 also applies the shared catalogue (review-ui-quality/references/slop-patterns.md). A pattern the project accepts on purpose (for example an accent posture or an exempt illustration) is recorded in the decision log (section 12), never by editing the catalogue.
- Critic category C9 (consistency) uses section 7 and the reference screenshot.
- Critic category C5 (states) uses section 8.
- Critic category C6 (motion) uses section 9.
- Critic category C3 (color) uses sections 3 and 11.
- Anti-inflation rule 4 (builder cannot grade) means the builder never edits the banned list or decision log to pass a gate.
