param($src, $name, [switch]$py, $work = "$env:TEMP\footnote-score")
# Compiler graph (rust-analyzer / scip-typescript / scip-python) vs parser rules on a scratch copy of one project (#216).
# -py strips Cargo.toml from the copy so a mixed Rust and Python project is graded by scip-python.
# Then: python -I score_by_language.py <work> <name>:<group> ...
$orig = $env:PATH; $exe = "$PSScriptRoot\..\..\..\target\release\footnote.exe"; $d = "$work\$name"
Remove-Item -Recurse -Force $d, "$d-comp", "$d-ts" -ErrorAction SilentlyContinue
robocopy $src $d /E /XD target node_modules .git dist build .venv venv __pycache__ .research external dependencies Archive.ignore /NFL /NDL /NJH /NJS /NP | Out-Null
if ($py) { Get-ChildItem $d -Recurse -Filter Cargo.toml | Remove-Item }
if (-not (Get-ChildItem $d -File | ? Name -in 'tsconfig.json','pyproject.toml','setup.py','setup.cfg','requirements.txt')) { New-Item "$d\requirements.txt" -ItemType File | Out-Null }
$env:FOOTNOTE_CODE_RULES = $null; $env:FOOTNOTE_STATE_DIR = "$d-comp"
$c = & $exe code-index $d | ConvertFrom-Json
# Parser only: no indexer on PATH.
$env:PATH = "C:\Windows\System32;C:\Program Files\Git\cmd"; $env:FOOTNOTE_STATE_DIR = "$d-ts"; $env:FOOTNOTE_CODE_RULES = "$d.tsv"
& $exe code-index $d | Out-Null
$env:PATH = $orig; $env:FOOTNOTE_CODE_RULES = $null; $env:FOOTNOTE_STATE_DIR = $null
"=== $name  indexer=$($c.indexer) files=$($c.files) refs=$($c.references) origin=$($c.origin)"
if ($c.origin -eq "compiler") { python -I "$PSScriptRoot\score_rules.py" $c.db "$d.tsv" }
