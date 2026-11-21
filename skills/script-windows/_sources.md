---
skill_hash: 524439e4e382657b
reviewed: 2026-11-21
---
# script-windows: sources

Maintainer file, not synced to agents.

| Claim | Source |
|---|---|
| Strict mode, error action | [about_Preference_Variables](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_preference_variables) |
| `$PSNativeCommandUseErrorActionPreference` | [about_Preference_Variables](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_preference_variables#psnativecommanduseerroractionpreference) |
| Default encodings 5.1 vs 7 | [about_Character_Encoding](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_character_encoding) |
| Stop-parsing token `--%` | [about_Parsing](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_parsing) |
| Execution policy is not a security boundary | [about_Execution_Policies](https://learn.microsoft.com/en-us/powershell/module/microsoft.powershell.core/about/about_execution_policies) |
| Long paths | [Maximum path length limitation](https://learn.microsoft.com/en-us/windows/win32/fileio/maximum-file-path-limitation) |
| winget flags | [winget install](https://learn.microsoft.com/en-us/windows/package-manager/winget/install) |
| PSScriptAnalyzer, Pester | [PSScriptAnalyzer](https://learn.microsoft.com/en-us/powershell/utility-modules/psscriptanalyzer/overview) · [Pester](https://pester.dev/docs/quick-start) |

## Field evidence
- This repository's Windows work: `&` cut from URLs in PowerShell, UTF-16 files from 5.1 redirection, native exit codes ignored by try/catch.
