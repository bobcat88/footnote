---
name: script-windows
description: Use when writing or fixing PowerShell or batch scripts, automating Windows setup (winget, registry, services, scheduled tasks, unattended installs), or debugging Windows-only failures: paths, quoting, encoding, exit codes, elevation, execution policy, PowerShell 5.1 vs 7 differences.
license: MIT
---

# Script Windows

## Run
- Lint: `Invoke-ScriptAnalyzer -Path . -Recurse -Severity Warning,Error` (module PSScriptAnalyzer)
- Test: `Invoke-Pester -Output Detailed` (Pester 5)
- Which shell: `$PSVersionTable.PSVersion`; 5.1 = Windows PowerShell (`powershell.exe`), 7+ = `pwsh.exe`

## Script header
- `Set-StrictMode -Version Latest` / `$ErrorActionPreference = 'Stop'` / `#Requires -Version 7.2` or `-RunAsAdministrator` when needed
- Native commands don't throw: check `$LASTEXITCODE` after each, or set `$PSNativeCommandUseErrorActionPreference = $true` (7.4+)
- `try { } catch { }` needs `-ErrorAction Stop` on cmdlets that only write non-terminating errors
- Return objects, not formatted text; `Write-Output` for data / `Write-Verbose`, `Write-Warning` for messages

## Paths and quoting
- `Join-Path` / `-LiteralPath` for names with `[ ]` / always quote paths with spaces
- Single quotes = literal / double quotes expand `$var` and `$(…)` / backtick is the escape character
- Passing tricky args to a native exe: an argument array (`& $exe @args`), or `--%` to stop parsing
- `&` in URLs breaks unquoted on cmd and PowerShell: quote the URL
- Long paths (> 260): `LongPathsEnabled` registry key + app manifest, or the `\\?\` prefix

## Encoding
- 5.1: `>` and `Out-File` write UTF-16 LE; `Set-Content` writes the ANSI code page → pass `-Encoding utf8` (5.1 adds a BOM)
- 7+: UTF-8 without BOM by default
- Console output from native tools: `[Console]::OutputEncoding = [Text.UTF8Encoding]::new()`
- Line endings: `.gitattributes` decides; `.ps1` files with non-ASCII text are safest as UTF-8 with BOM for 5.1

## System changes
- Check elevation: `([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole('Administrators')`
- Idempotent: test state before changing it (`Get-ItemProperty`, `Get-Service`, `winget list --id`) so a rerun is safe
- Installs: `winget install --id <Id> -e --silent --accept-package-agreements --accept-source-agreements`
- Registry: `HKLM:` / `HKCU:` drives; export the key first (`reg export`) before editing
- Scheduled tasks: `Register-ScheduledTask` with an explicit principal, working directory and log file
- `Get-CimInstance`, not `Get-WmiObject` (removed in 7)
- Unattended install: `autounattend.xml` validated in Windows SIM; test in a VM snapshot first

## Safety
- Execution policy isn't a security boundary; sign scripts that ship (`Set-AuthenticodeSignature`)
- No secrets in scripts: SecretManagement module, or Credential Manager
- Destructive cmdlets: `-WhatIf` run first; `-Confirm:$false` only in reviewed automation
