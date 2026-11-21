#requires -Version 5.1
<#
.SYNOPSIS
    Register the Footnote Docker hub as an MCP server in every agent found for this user (#392).

.DESCRIPTION
    Detects Claude Code, Codex, Gemini CLI and Antigravity, and points each of them at the long-lived Compose daemon:

        <absolute path of docker> exec -i <container> footnote --db <db> mcp

    - An existing entry with the same name is left as it is. -Force replaces it, -Remove deletes it.
    - Claude Code is configured through its own command (`claude mcp add --scope user`), so its configuration file is
      never edited here. The other agents' files are backed up once (<file>.footnote-backup) and replaced atomically.
      A file that the agent rewrote in the meantime is not overwritten.
    - Nothing goes over the network. -DryRun shows what would change and writes nothing.
    - Anyone who can use this Docker engine can read every memory in the hub: see SECURITY.md, "Who can reach the hub".

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\docker-agents.ps1 -DryRun

.EXAMPLE
    powershell -NoProfile -ExecutionPolicy Bypass -File scripts\docker-agents.ps1 -Container footnote-team-a
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
$docker = if ($dockerCmd) { $dockerCmd.Source -replace '\\', '/' } else { '' }
if ($docker -match '["\r\n]') { throw "unsupported characters in the docker path: $docker" }
$mcpArgs = @('exec', '-i', $Container, 'footnote', '--db', $Database, 'mcp')
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

# `mcpServers.<Name>` in a JSON settings file (Gemini CLI, Antigravity).
function Set-JsonServer([string]$Agent, [string]$Path) {
    $stamp = Get-FileStamp $Path
    $raw = Get-FileText $Path
    if ($null -eq $raw -or $raw.Trim() -eq '') {
        if ($Remove) { Add-Result $Agent $Path 'not registered'; return }
        $root = New-Object PSObject
    } else {
        # Rewriting would drop comments: leave such a file to the user.
        if ($raw -match '(?m)^\s*//' -or $raw.Contains('/*')) {
            Add-Result $Agent $Path "skipped: the file has comments; add `"$Name`" under mcpServers yourself"
            return
        }
        $convert = @{ InputObject = $raw }
        if ((Get-Command ConvertFrom-Json).Parameters.ContainsKey('DateKind')) { $convert.DateKind = 'String' }
        try { $root = ConvertFrom-Json @convert } catch { throw "$Path is not valid JSON: $($_.Exception.Message)" }
        if ($root -isnot [System.Management.Automation.PSCustomObject]) { throw "$Path does not hold a JSON object" }
    }
    $servers = $null
    if ($root.PSObject.Properties['mcpServers']) { $servers = $root.mcpServers }
    if ($null -ne $servers -and $servers -isnot [System.Management.Automation.PSCustomObject]) {
        throw "$Path`: mcpServers is not an object"
    }
    $present = $null -ne $servers -and $null -ne $servers.PSObject.Properties[$Name]
    if ($Remove) {
        if (-not $present) { Add-Result $Agent $Path 'not registered'; return }
        $servers.PSObject.Properties.Remove($Name)
        $status = 'removed'
    } else {
        if ($present -and -not $Force) { Add-Result $Agent $Path 'already registered (-Force replaces it)'; return }
        if ($null -eq $servers) {
            $servers = New-Object PSObject
            if ($root.PSObject.Properties['mcpServers']) {
                $root.mcpServers = $servers # "mcpServers": null
            } else {
                $root | Add-Member -NotePropertyName mcpServers -NotePropertyValue $servers
            }
        }
        $entry = [pscustomobject][ordered]@{ command = $docker; args = $mcpArgs }
        if ($present) { $servers.PSObject.Properties.Remove($Name) }
        $servers | Add-Member -NotePropertyName $Name -NotePropertyValue $entry
        $status = if ($present) { 'replaced' } else { 'registered' }
    }
    if (-not $DryRun) {
        Set-FileAtomically $Path ((ConvertTo-Json -InputObject $root -Depth 100) + "`n") $stamp
    }
    Add-Result $Agent $Path $status
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
    if ($Remove -and -not $present) { Add-Result 'codex' $Path 'not registered'; return }
    if ($present -and -not $Force -and -not $Remove) {
        Add-Result 'codex' $Path 'already registered (-Force replaces it)'
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
    Add-Result 'codex' $Path $status
}

# Claude Code: its own command keeps ~/.claude.json consistent (Claude Code rewrites that file while it runs).
function Set-ClaudeServer {
    $claude = Get-Command claude -ErrorAction SilentlyContinue | Select-Object -First 1
    $target = 'claude mcp (user scope)'
    if (-not $claude) {
        Add-Result 'claude-code' $target "skipped: the claude command is not on PATH; run: claude mcp add --scope user $Name -- `"$docker`" $($mcpArgs -join ' ')"
        return
    }
    Push-Location -LiteralPath $HomeDir
    try {
        Invoke-Native { & $claude mcp get $Name *> $null }
        $present = $LASTEXITCODE -eq 0
        if ($Remove -and -not $present) { Add-Result 'claude-code' $target 'not registered'; return }
        if ($present -and -not $Force -and -not $Remove) {
            Add-Result 'claude-code' $target 'already registered (-Force replaces it)'
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
        Add-Result 'claude-code' $target $status
    } finally {
        Pop-Location
    }
}

$gemini = Join-Path $HomeDir '.gemini'
$codexHome = if ($env:CODEX_HOME) { $env:CODEX_HOME } else { Join-Path $HomeDir '.codex' }
$found = $false

if ((Get-Command claude -ErrorAction SilentlyContinue) -or (Test-Path -LiteralPath (Join-Path $HomeDir '.claude'))) {
    $found = $true
    Set-ClaudeServer
}
if ((Get-Command codex -ErrorAction SilentlyContinue) -or (Test-Path -LiteralPath $codexHome)) {
    $found = $true
    Set-CodexServer (Join-Path $codexHome 'config.toml')
}
if ((Get-Command gemini -ErrorAction SilentlyContinue) -or (Test-Path -LiteralPath (Join-Path $gemini 'settings.json'))) {
    $found = $true
    Set-JsonServer 'gemini-cli' (Join-Path $gemini 'settings.json')
}
if (@('config', 'antigravity', 'antigravity-cli') | Where-Object { Test-Path -LiteralPath (Join-Path $gemini $_) }) {
    $found = $true
    Set-JsonServer 'antigravity' (Join-Path $gemini 'config\mcp_config.json')
}
if (-not $found) { Write-Warning 'no agent found (Claude Code, Codex, Gemini CLI, Antigravity): nothing to do' }

$results | Format-Table -AutoSize -Wrap | Out-String -Width 200 | Write-Host

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
    }
}
if (-not $DryRun -and ($results | Where-Object { $_.Status -match '^(registered|replaced|removed)' })) {
    Write-Host 'Restart each agent to load the change.'
}
