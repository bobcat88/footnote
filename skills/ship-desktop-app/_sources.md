---
skill_hash: dab8d4205a7af076
reviewed: 2026-11-21
---
# ship-desktop-app: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| IPC, commands, channels | [Tauri IPC](https://v2.tauri.app/concept/inter-process-communication/) |
| Capabilities per window | [Tauri capabilities](https://v2.tauri.app/security/capabilities/) |
| CSP | [Tauri CSP](https://v2.tauri.app/security/csp/) |
| Updater keys | [Tauri updater](https://v2.tauri.app/plugin/updater/) |
| NSIS bundler, image sizes | [Tauri Windows installer](https://v2.tauri.app/distribute/windows-installer/) · [NsisConfig](https://docs.rs/tauri-utils/latest/tauri_utils/config/struct.NsisConfig.html) |
| EV no longer bypasses SmartScreen | [Microsoft: code signing options](https://learn.microsoft.com/en-us/windows/apps/package-and-deploy/code-signing-options) |
| Artifact Signing pricing and eligibility | [Azure pricing](https://azure.microsoft.com/en-us/pricing/details/trusted-signing/) |
| Inno Setup dark mode, styles | [Inno Setup 7 what's new](https://jrsoftware.org/files/is7-whatsnew.htm) |
| Velopack | [velopack.io](https://velopack.io/) |
| WebView bridge plugin | [tauri-connector](https://github.com/dickwu/tauri-connector) |

## Field evidence
- WebView bridge traps (bundled frontend required, focused window on Windows) observed integrating the connector 0.17.10 in a private Tauri app; Windows WebView2 only, macOS and Linux unverified.
