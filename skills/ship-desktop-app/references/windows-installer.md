# Windows installers and signing

Versions and prices move: re-check before deciding (last checked 2026-11-21).

| Tool | Look | Updates | Pick when |
|---|---|---|---|
| Tauri NSIS | Images: header 150×57, sidebar 164×314, icon; hooks in `.nsh` | Tauri updater | Default for Tauri; a full custom `.nsi` loses upstream template fixes |
| Inno Setup 7 | `WizardStyle=modern dynamic` follows dark/light; custom style files, dark images | None | Short branded wizard |
| Velopack | One-click + splash, no wizard | Built in, delta | Rust-native packaging; questions move to first run |
| MSI (WiX) | Dated | None | GPO / Intune only; MSIs from different WiX majors don't upgrade each other cleanly |
| winget | n/a | winget | Manifest → signed installer, once signing works |

Installer ≤ 3 pages (location / options / finish); the app's first run carries the brand.

## Signing
- EV and OV both earn SmartScreen reputation per file over downloads (EV bypass ended 2024)
- Azure Artifact Signing (ex-Trusted Signing): cheap managed signing; individual eligibility limited by country, check organisation eligibility
- Otherwise OV cert from a CA, key on hardware token or cloud HSM
- Sign + timestamp every exe and the installer; keep the publisher name stable
