#requires -Version 5.1
<#
.SYNOPSIS
    Register the Footnote Docker daemon with every agent found for this user: MCP server and memory hooks (#392).

.DESCRIPTION
    Detects Claude Code, Codex, Gemini CLI and Antigravity (also its T3 Code provider folders and $GEMINI_HOME), and
    points each of them at the long-lived Compose daemon of this user:

        MCP server:   <absolute path of docker> exec -i <container> footnote --db <db> mcp
        memory hooks: <absolute path of docker> exec -i <container> footnote hook

    The hooks are the ones `footnote install --memory` writes on a host (same files, events and timeouts): the memory
    pack at session start, recall on each prompt, and the save request or handoff at stop.

    - An existing entry is left as it is: a Footnote memory hook already there (this script's, or the native
      `footnote hook`) is not doubled. -Force replaces it, -Remove deletes what this script added.
    - Claude Code's MCP server goes through its own command (`claude mcp add --scope user`), so ~/.claude.json is never
      edited here; its hooks live in ~/.claude/settings.json. Every edited file is backed up once
      (<file>.footnote-backup) and replaced atomically. A file that the agent rewrote in the meantime is not overwritten;
      a JSON file with comments is left to you.
    - Nothing goes over the network. -DryRun shows what would change and writes nothing.
    - Anyone who can use this Docker engine can read every memory in the container: see SECURITY.md.

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\docker-agents.ps1 -DryRun

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\docker-agents.ps1 -Container footnote-work
#>
[CmdletBinding()]
param(
    [string]$Container = $(if ($env:FOOTNOTE_CONTAINER_NAME) { $env:FOOTNOTE_CONTAINER_NAME } else { 'footnote' }),
    [string]$Database = '/state/memory.db',
    [string]$Name = 'footnote-memory',
    [string]$HomeDir = $(if ($env:USERPROFILE) { $env:USERPROFILE } else { $HOME }),
    [switch]$DryRun,
    [switch]$Force,
    [switch]$Remove
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# Values end up in JSON, TOML and command lines: accept only what Docker and Footnote accept, nothing to escape.
if ($Container -notmatch '^[A-Za-z0-9][A-Za-z0-9_.-]{0,127}$') { throw "invalid -Container '$Container'" }
if ($Database -notmatch '^/[A-Za-z0-9._/-]+$' -or $Database -match '(^|/)\.\.?(/|$)') {
    throw "-Database must be an absolute container path such as /state/memory.db"
}
if ($Name -notmatch '^[A-Za-z0-9_-]{1,64}$') { throw "invalid -Name '$Name'" }
if ($Force -and $Remove) { throw '-Force and -Remove cannot be combined' }
if (-not (Test-Path -LiteralPath $HomeDir -PathType Container)) { throw "home folder not found: $HomeDir" }

$dockerCmd = Get-Command docker -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
if (-not $dockerCmd -and -not $Remove) { throw 'docker is not on PATH: install Docker Desktop (or Docker Engine) first' }
# GUI agents do not inherit the shell PATH: register the absolute path, with forward slashes (no JSON/TOML escaping).
$docker = if ($dockerCmd) { $dockerCmd.Source -replace '\\', '/' } else { 'docker' }
if ($docker -match '["\r\n]') { throw "unsupported characters in the docker path: $docker" }
$mcpArgs = @('exec', '-i', $Container, 'footnote', '--db', $Database, 'mcp')
# Agents run a hook command through a shell (bash for Claude Code on Windows): quote the path only when it has a space.
$hookExe = if ($docker.Contains(' ')) { "`"$docker`"" } else { $docker }
$hookCommand = "$hookExe exec -i $Container footnote hook"
$claudeEvents = [ordered]@{ SessionStart = 15; UserPromptSubmit = 10; Stop = 5 }
$geminiEvents = [ordered]@{ SessionStart = 15000; BeforeAgent = 10000; AfterAgent = 5000 }
$utf8 = New-Object System.Text.UTF8Encoding $false
$results = New-Object System.Collections.Generic.List[object]

# Windows PowerShell 5.1 turns a native command's redirected stderr into terminating errors under 'Stop':
# run such commands under 'Continue' and judge them by $LASTEXITCODE.
function Invoke-Native([scriptblock]$Block) {
    $saved = $ErrorActionPreference
    $ErrorActionPreference = 'Continue'
    try { & $Block } finally { $ErrorActionPreference = $saved }
}

function Add-Result([string]$Agent, [string]$Target, [string]$Status) {
    $s = if ($DryRun -and $Status -match '^(registered|replaced|removed)') { "$Status (dry run)" } else { $Status }
    $results.Add([pscustomobject]@{ Agent = $Agent; Target = $Target; Status = $s })
}

function Get-FileText([string]$Path) {
    if (Test-Path -LiteralPath $Path -PathType Leaf) { return [IO.File]::ReadAllText($Path) }
    return $null
}

# (modified time, length): notices an agent that rewrites its file while this script works on it.
function Get-FileStamp([string]$Path) {
    if (Test-Path -LiteralPath $Path -PathType Leaf) {
        $item = Get-Item -LiteralPath $Path
        return "$($item.LastWriteTimeUtc.Ticks):$($item.Length)"
    }
    return ''
}

function Set-FileAtomically([string]$Path, [string]$Text, [string]$Stamp) {
    if ((Get-FileStamp $Path) -ne $Stamp) {
        throw "$Path changed while it was being updated (the agent is probably running): close it and run this again"
    }
    $dir = Split-Path -Parent $Path
    if ($dir -and -not (Test-Path -LiteralPath $dir)) { New-Item -ItemType Directory -Path $dir -Force | Out-Null }
    $tmp = "$Path.footnote-tmp"
    [IO.File]::WriteAllText($tmp, $Text, $utf8)
    if (Test-Path -LiteralPath $Path -PathType Leaf) {
        $backup = "$Path.footnote-backup"
        if (-not (Test-Path -LiteralPath $backup)) { Copy-Item -LiteralPath $Path -Destination $backup }
        # [NullString]: a plain $null would reach .NET as "" (an invalid backup path).
        [IO.File]::Replace($tmp, $Path, [NullString]::Value)
    } else {
        [IO.File]::Move($tmp, $Path)
    }
}

# True when the text has a `//` or `/*` comment outside a JSON string. A glob inside a string, such as a permission
# "Read(src/**)" in Claude Code's settings.json, is not a comment.
function Test-JsonComment([string]$Text) {
    $inString = $false
    for ($i = 0; $i -lt $Text.Length - 1; $i++) {
        $c = $Text[$i]
        if ($inString) {
            if ($c -eq [char]'\') { $i++ } elseif ($c -eq [char]'"') { $inString = $false }
        } elseif ($c -eq [char]'"') {
            $inString = $true
        } elseif ($c -eq [char]'/' -and ($Text[$i + 1] -eq [char]'/' -or $Text[$i + 1] -eq [char]'*')) {
            return $true
        }
    }
    return $false
}

# The JSON object of a settings file ({} when missing or empty), or $null when the file is left to the user (comments).
function Read-JsonObject([string]$Agent, [string]$Path, [string]$What) {
    $raw = Get-FileText $Path
    if ($null -eq $raw -or $raw.Trim() -eq '') { return (New-Object PSObject) }
    # Rewriting would drop comments: leave such a file to the user.
    if (Test-JsonComment $raw) {
        Add-Result $Agent $Path "skipped: the file has comments; add the $What yourself (DOCKER.md)"
        return $null
    }
    $convert = @{ InputObject = $raw }
    if ((Get-Command ConvertFrom-Json).Parameters.ContainsKey('DateKind')) { $convert.DateKind = 'String' }
    try { $root = ConvertFrom-Json @convert } catch { throw "$Path is not valid JSON: $($_.Exception.Message)" }
    if ($root -isnot [System.Management.Automation.PSCustomObject]) { throw "$Path does not hold a JSON object" }
    return $root
}

function Write-JsonObject([string]$Path, [object]$Root, [string]$Stamp) {
    if (-not $DryRun) { Set-FileAtomically $Path ((ConvertTo-Json -InputObject $Root -Depth 100) + "`n") $Stamp }
}

# A property's value ($null when absent). An array value comes back unrolled: wrap the call in @() to get an array.
function Get-JsonField([object]$Obj, [string]$Field) {
    $p = $Obj.PSObject.Properties[$Field]
    if ($p) { return $p.Value }
    return $null
}

function Set-JsonField([object]$Obj, [string]$Field, [object]$Value) {
    if ($Obj.PSObject.Properties[$Field]) { $Obj.$Field = $Value }
    else { $Obj | Add-Member -NotePropertyName $Field -NotePropertyValue $Value }
}

# A Footnote memory hook: this script's `docker exec -i <container> footnote hook`, the native `footnote hook`, or the
# Python hook it replaced (the same rule as `footnote install`).
function Test-MemoryHook([string]$Command) {
    $c = $Command.Trim()
    if ($c.Contains('memory_hook.py')) { return $true }
    if ($c -match ' exec -i [A-Za-z0-9][A-Za-z0-9_.-]* footnote hook$') { return $true }
    if (-not $c.EndsWith(' hook')) { return $false }
    $exe = ($c.Substring(0, $c.Length - 5).Trim().Trim('"', "'") -replace '\\', '/')
    $leaf = ($exe -split '/')[-1]
    return ($leaf -eq 'footnote' -or $leaf -eq 'footnote.exe')
}

function Test-DockerHook([string]$Command) {
    return ($Command.Trim() -match ' exec -i [A-Za-z0-9][A-Za-z0-9_.-]* footnote hook$')
}

# `mcpServers.<Name>` in a JSON settings file (Gemini CLI, Antigravity).
function Set-JsonServer([string]$Agent, [string]$Path) {
    $stamp = Get-FileStamp $Path
    $root = Read-JsonObject $Agent $Path "`"$Name`" entry under mcpServers"
    if ($null -eq $root) { return }
    $servers = Get-JsonField $root 'mcpServers'
    if ($null -ne $servers -and $servers -isnot [System.Management.Automation.PSCustomObject]) {
        throw "$Path`: mcpServers is not an object"
    }
    $present = $null -ne $servers -and $null -ne $servers.PSObject.Properties[$Name]
    if ($Remove) {
        if (-not $present) { Add-Result $Agent $Path 'mcp: not registered'; return }
        $servers.PSObject.Properties.Remove($Name)
        $status = 'removed'
    } else {
        if ($present -and -not $Force) { Add-Result $Agent $Path 'mcp: already registered (-Force replaces it)'; return }
        if ($null -eq $servers) {
            $servers = New-Object PSObject
            Set-JsonField $root 'mcpServers' $servers
        }
        if ($present) { $servers.PSObject.Properties.Remove($Name) }
        $servers | Add-Member -NotePropertyName $Name -NotePropertyValue ([pscustomobject][ordered]@{ command = $docker; args = $mcpArgs })
        $status = if ($present) { 'replaced' } else { 'registered' }
    }
    Write-JsonObject $Path $root $stamp
    Add-Result $Agent $Path "$status (mcp)"
}

# The memory hook in each listed event of a Claude-format `hooks` object (Claude Code, Codex) or of Gemini CLI's
# settings (`matcher: "*"`, millisecond timeouts).
function Set-EventHooks([string]$Agent, [string]$Path, [System.Collections.IDictionary]$Events, [switch]$Gemini) {
    $stamp = Get-FileStamp $Path
    $root = Read-JsonObject $Agent $Path 'memory hooks'
    if ($null -eq $root) { return }
    $hooks = Get-JsonField $root 'hooks'
    if ($null -ne $hooks -and $hooks -isnot [System.Management.Automation.PSCustomObject]) { throw "$Path`: hooks is not an object" }
    if ($null -eq $hooks) {
        if ($Remove) { Add-Result $Agent $Path 'hooks: not registered'; return }
        $hooks = New-Object PSObject
        Set-JsonField $root 'hooks' $hooks
    }
    $other = 0
    foreach ($event in $Events.Keys) {
        foreach ($group in @(Get-JsonField $hooks $event)) {
            if ($null -eq $group) { continue }
            foreach ($h in @(Get-JsonField $group 'hooks')) {
                if ($null -eq $h) { continue }
                $cmd = [string](Get-JsonField $h 'command')
                if ((Test-MemoryHook $cmd) -and $cmd -ne $hookCommand) { $other++ }
            }
        }
    }
    if ($other -gt 0 -and -not $Force -and -not $Remove) {
        Add-Result $Agent $Path 'hooks: another Footnote memory hook is registered (-Force replaces it)'
        return
    }
    $changed = $false
    $added = $false
    foreach ($event in $Events.Keys) {
        $groups = New-Object System.Collections.ArrayList
        $found = $false
        foreach ($group in @(Get-JsonField $hooks $event)) {
            if ($null -eq $group) { continue }
            $inner = @(Get-JsonField $group 'hooks')
            $kept = New-Object System.Collections.ArrayList
            foreach ($h in $inner) {
                if ($null -eq $h) { continue }
                $cmd = [string](Get-JsonField $h 'command')
                if (Test-MemoryHook $cmd) {
                    if ($Remove) {
                        if (Test-DockerHook $cmd) { $changed = $true; continue }
                    } elseif ($cmd -ne $hookCommand) {
                        Set-JsonField $h 'command' $hookCommand
                        $changed = $true
                    }
                    $found = $true
                }
                [void]$kept.Add($h)
            }
            if ($kept.Count -eq 0 -and $inner.Count -gt 0) { continue } # every hook of this group was ours: drop the group
            Set-JsonField $group 'hooks' @($kept)
            [void]$groups.Add($group)
        }
        if (-not $found -and -not $Remove) {
            $hook = [pscustomobject][ordered]@{ type = 'command'; command = $hookCommand; timeout = $Events[$event] }
            $group = if ($Gemini) { [pscustomobject][ordered]@{ matcher = '*'; hooks = @($hook) } }
                     else { [pscustomobject][ordered]@{ hooks = @($hook) } }
            [void]$groups.Add($group)
            $changed = $true
            $added = $true
        }
        if ($groups.Count -gt 0) { Set-JsonField $hooks $event @($groups) }
        elseif ($hooks.PSObject.Properties[$event]) { $hooks.PSObject.Properties.Remove($event) }
    }
    if (-not $changed) {
        Add-Result $Agent $Path $(if ($Remove) { 'hooks: not registered' } else { 'hooks: already registered' })
        return
    }
    Write-JsonObject $Path $root $stamp
    $status = if ($Remove) { 'removed' } elseif ($added -and $other -eq 0) { 'registered' } else { 'replaced' }
    Add-Result $Agent $Path "$status (hooks: $(@($Events.Keys) -join ', '))"
}

# Antigravity's named `"<Name>"` entry in <cfg>/hooks.json (flat handler arrays), the same entry `footnote install` writes.
function Set-AgyHooks([string]$Agent, [string]$Path) {
    $stamp = Get-FileStamp $Path
    $root = Read-JsonObject $Agent $Path "`"$Name`" hook entry"
    if ($null -eq $root) { return }
    $handler = { param($t) , @([pscustomobject][ordered]@{ type = 'command'; command = $hookCommand; timeout = $t }) }
    $desired = [pscustomobject][ordered]@{
        SessionStart = (& $handler 15); PreInvocation = (& $handler 10); Stop = (& $handler 5)
    }
    $current = Get-JsonField $root $Name
    $json = { param($v) ConvertTo-Json -InputObject $v -Depth 20 -Compress }
    if ($Remove) {
        $ours = $null -ne $current -and (& $json $current).Contains('footnote hook') -and (& $json $current) -match ' exec -i '
        if (-not $ours) { Add-Result $Agent $Path 'hooks: not registered'; return }
        $root.PSObject.Properties.Remove($Name)
        Write-JsonObject $Path $root $stamp
        Add-Result $Agent $Path 'removed (hooks)'
        return
    }
    if ($null -ne $current -and (& $json $current) -eq (& $json $desired)) { Add-Result $Agent $Path 'hooks: already registered'; return }
    if ($null -ne $current -and -not $Force) {
        Add-Result $Agent $Path "hooks: a `"$Name`" entry is already there (-Force replaces it)"
        return
    }
    Set-JsonField $root $Name $desired
    Write-JsonObject $Path $root $stamp
    Add-Result $Agent $Path "$(if ($null -ne $current) { 'replaced' } else { 'registered' }) (hooks: SessionStart, PreInvocation, Stop)"
}

# `[mcp_servers.<Name>]` in Codex's config.toml (with its subtables, such as `.env`).
function Set-CodexServer([string]$Path) {
    $stamp = Get-FileStamp $Path
    $raw = Get-FileText $Path
    if ($null -eq $raw) { $raw = '' }
    $eol = if ($raw.Contains("`r`n")) { "`r`n" } else { "`n" }
    $lines = New-Object System.Collections.Generic.List[string]
    if ($raw.Length -gt 0) { $lines.AddRange([string[]]($raw -split "`r?`n")) }
    $own = @("[mcp_servers.$Name]", "[mcp_servers.`"$Name`"]")
    $sub = @("[mcp_servers.$Name.", "[mcp_servers.`"$Name`".")
    $start = -1
    for ($i = 0; $i -lt $lines.Count; $i++) {
        if ($own -ccontains $lines[$i].Trim()) { $start = $i; break }
    }
    $present = $start -ge 0
    if ($Remove -and -not $present) { Add-Result 'codex' $Path 'mcp: not registered'; return }
    if ($present -and -not $Force -and -not $Remove) {
        Add-Result 'codex' $Path 'mcp: already registered (-Force replaces it)'
        return
    }
    if ($present) {
        $end = $start + 1
        while ($end -lt $lines.Count) {
            $t = $lines[$end].Trim()
            $ours = $false
            foreach ($p in $sub) { if ($t.StartsWith($p)) { $ours = $true } }
            if ($t.StartsWith('[') -and -not $ours) { break }
            $end++
        }
        $lines.RemoveRange($start, $end - $start)
    }
    if (-not $Remove) {
        while ($lines.Count -gt 0 -and $lines[$lines.Count - 1].Trim() -eq '') { $lines.RemoveAt($lines.Count - 1) }
        if ($lines.Count -gt 0) { $lines.Add('') }
        $quoted = ($mcpArgs | ForEach-Object { "`"$_`"" }) -join ', '
        $lines.Add("[mcp_servers.$Name]")
        $lines.Add("command = `"$docker`"")
        $lines.Add("args = [$quoted]")
    }
    $status = if ($Remove) { 'removed' } elseif ($present) { 'replaced' } else { 'registered' }
    if (-not $DryRun) { Set-FileAtomically $Path ((($lines -join $eol).TrimEnd()) + $eol) $stamp }
    Add-Result 'codex' $Path "$status (mcp)"
}

# Claude Code's MCP server: its own command keeps ~/.claude.json consistent (Claude Code rewrites that file while it runs).
function Set-ClaudeServer {
    $claude = Get-Command claude -ErrorAction SilentlyContinue | Select-Object -First 1
    $target = 'claude mcp (user scope)'
    if (-not $claude) {
        Add-Result 'claude-code' $target "skipped: the claude command is not on PATH; run: claude mcp add $Name --scope user -- `"$docker`" $($mcpArgs -join ' ')"
        return
    }
    Push-Location -LiteralPath $HomeDir
    try {
        Invoke-Native { & $claude mcp get $Name *> $null }
        $present = $LASTEXITCODE -eq 0
        if ($Remove -and -not $present) { Add-Result 'claude-code' $target 'mcp: not registered'; return }
        if ($present -and -not $Force -and -not $Remove) {
            Add-Result 'claude-code' $target 'mcp: already registered (-Force replaces it)'
            return
        }
        $status = if ($Remove) { 'removed' } elseif ($present) { 'replaced' } else { 'registered' }
        if (-not $DryRun) {
            if ($present) {
                Invoke-Native { & $claude mcp remove --scope user $Name *> $null }
                if ($LASTEXITCODE -ne 0) { throw "claude mcp remove --scope user $Name failed (exit $LASTEXITCODE)" }
            }
            if (-not $Remove) {
                Invoke-Native { & $claude mcp add --scope user $Name -- $docker @mcpArgs *> $null }
                if ($LASTEXITCODE -ne 0) { throw "claude mcp add --scope user $Name failed (exit $LASTEXITCODE)" }
            }
        }
        Add-Result 'claude-code' $target "$status (mcp)"
    } finally {
        Pop-Location
    }
}

# Antigravity config folders, found as `footnote install` finds them: ~/.gemini/config, T3 Code's isolated provider
# folders (~/.t3/userdata/providers/antigravity/<id>/config), and $GEMINI_HOME/config when it is inside the home folder.
function Get-AntigravityConfigDirs {
    $out = New-Object System.Collections.Generic.List[object]
    $add = {
        param([string]$Label, [string]$Cfg)
        if (-not ($out | Where-Object { $_.Path -ieq $Cfg })) { $out.Add([pscustomobject]@{ Agent = $Label; Path = $Cfg }) }
    }
    $g = Join-Path $HomeDir '.gemini'
    if (@('config', 'antigravity', 'antigravity-cli') | Where-Object { Test-Path -LiteralPath (Join-Path $g $_) }) {
        & $add 'antigravity' (Join-Path $g 'config')
    }
    $t3 = Join-Path $HomeDir '.t3\userdata\providers\antigravity'
    if (Test-Path -LiteralPath $t3 -PathType Container) {
        foreach ($dir in (Get-ChildItem -LiteralPath $t3 -Directory | Sort-Object Name)) {
            $cfg = Join-Path $dir.FullName 'config'
            if ((Test-Path -LiteralPath $cfg -PathType Container) -or (Test-Path -LiteralPath (Join-Path $dir.FullName 'antigravity-acp') -PathType Container)) {
                & $add 'antigravity (t3-code)' $cfg
            }
        }
    }
    if ($env:GEMINI_HOME -and (Test-Path -LiteralPath $env:GEMINI_HOME -PathType Container)) {
        $gh = (Resolve-Path -LiteralPath $env:GEMINI_HOME).Path.TrimEnd('\', '/')
        $hd = (Resolve-Path -LiteralPath $HomeDir).Path.TrimEnd('\', '/')
        if ($gh -ieq $hd -or $gh.StartsWith("$hd\", [StringComparison]::OrdinalIgnoreCase) -or $gh.StartsWith("$hd/", [StringComparison]::OrdinalIgnoreCase)) {
            & $add 'antigravity (GEMINI_HOME)' (Join-Path $gh 'config')
        }
    }
    return , $out
}

$claudeDir = if ($env:CLAUDE_CONFIG_DIR) { $env:CLAUDE_CONFIG_DIR } else { Join-Path $HomeDir '.claude' }
$codexHome = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $HomeDir '.codex' }
$geminiSettings = Join-Path (Join-Path $HomeDir '.gemini') 'settings.json'
$found = $false
$claudeHooks = $false

if ((Get-Command claude -ErrorAction SilentlyContinue) -or (Test-Path -LiteralPath $claudeDir)) {
    $found = $true
    Set-ClaudeServer
    Set-EventHooks 'claude-code' (Join-Path $claudeDir 'settings.json') $claudeEvents
    $claudeHooks = $true
}
if ((Get-Command codex -ErrorAction SilentlyContinue) -or (Test-Path -LiteralPath $codexHome)) {
    $found = $true
    Set-CodexServer (Join-Path $codexHome 'config.toml')
    Set-EventHooks 'codex' (Join-Path $codexHome 'hooks.json') $claudeEvents
}
if ((Get-Command gemini -ErrorAction SilentlyContinue) -or (Test-Path -LiteralPath $geminiSettings)) {
    $found = $true
    Set-JsonServer 'gemini-cli' $geminiSettings
    Set-EventHooks 'gemini-cli' $geminiSettings $geminiEvents -Gemini
}
foreach ($agy in (Get-AntigravityConfigDirs)) {
    $found = $true
    Set-JsonServer $agy.Agent (Join-Path $agy.Path 'mcp_config.json')
    Set-AgyHooks $agy.Agent (Join-Path $agy.Path 'hooks.json')
}
if (-not $found) { Write-Warning 'no agent found (Claude Code, Codex, Gemini CLI, Antigravity): nothing to do' }

# One line per entry, the status before the path: a long path (T3 Code folders) must not squeeze the status.
$agentWidth = [Math]::Max(5, (@($results | ForEach-Object { $_.Agent.Length }) + 0 | Measure-Object -Maximum).Maximum)
foreach ($r in $results) { Write-Host ("{0} {1} {2}" -f $r.Agent.PadRight($agentWidth), $r.Status, "[$($r.Target)]") }

if (-not $Remove -and $dockerCmd) {
    $running = (Invoke-Native { & $dockerCmd.Source inspect -f '{{.State.Running}}' $Container 2>$null }) -eq 'true'
    if (-not $running) {
        Write-Warning "container '$Container' is not running: start it with 'docker compose up -d' (agents connect to it)"
    } else {
        $requests = @(
            '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"docker-agents","version":"1"}}}'
            '{"jsonrpc":"2.0","id":2,"method":"tools/list","params":{}}'
        ) -join "`n"
        $reply = Invoke-Native { $requests | & $dockerCmd.Source @mcpArgs 2>$null | Select-Object -Last 1 }
        try { $tools = @((ConvertFrom-Json -InputObject $reply).result.tools).Count } catch { $tools = 0 }
        if ($tools -gt 0) {
            Write-Host "Checked: '$Container' answers over MCP with $tools tools."
        } else {
            Write-Warning "'$Container' is running but did not answer tools/list: check 'docker logs $Container'"
        }
        # Claude Code's handoff at Stop reads the size of the session from its transcript.
        $transcripts = Invoke-Native { & $dockerCmd.Source inspect -f '{{range .Mounts}}{{if eq .Destination "/host/claude-projects"}}{{.Type}}{{end}}{{end}}' $Container 2>$null }
        if ($claudeHooks -and $transcripts -ne 'bind') {
            Write-Host "Note: '$Container' cannot read Claude Code's session files, so the handoff of a long session at Stop is off. Set FOOTNOTE_CLAUDE_PROJECTS (DOCKER.md, Section 3.1)."
        }
    }
}
if (-not $DryRun -and ($results | Where-Object { $_.Status -match '^(registered|replaced|removed)' })) {
    Write-Host 'Restart each agent to load the change.'
}
