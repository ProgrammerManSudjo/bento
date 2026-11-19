# Copies the working setup on this machine into this repository, with the personal bits taken out:
#   - the profile folder in config.yaml becomes {{HOME}}
#   - YASB's screens list becomes ["*"] (every monitor) instead of one monitor's name
#   - komorebi's display_index_preferences (monitor ids of one PC) are dropped
# Run it whenever the live setup changed and you want the repository to follow.

$ErrorActionPreference = 'Stop'
$repo = Split-Path $PSScriptRoot -Parent
$cfg = Join-Path $env:USERPROFILE '.config'

function Copy-Clean([string]$From, [string]$To) {
    if (-not (Test-Path -LiteralPath $From)) { Write-Warning "missing: $From"; return }
    New-Item -ItemType Directory -Force -Path (Split-Path $To) | Out-Null
    Copy-Item -LiteralPath $From -Destination $To -Force
}

function Write-Text([string]$Path, [string]$Text) {
    New-Item -ItemType Directory -Force -Path (Split-Path $Path) | Out-Null
    [System.IO.File]::WriteAllText($Path, $Text, (New-Object System.Text.UTF8Encoding($false)))
}

# --- YASB ---
& (Join-Path $PSScriptRoot 'Export-Template.ps1') | Out-Null
$tpl = Join-Path $repo 'config\yasb\config.yaml.template'
$text = [System.IO.File]::ReadAllText($tpl)
$text = [regex]::Replace($text, 'screens:\s*\[[^\]]*\]', 'screens: ["*"]')
# personal bits that mean nothing on another PC: the dual-boot "Swap OS" menu entry, and the wallpaper folder (the installer puts wallpapers in Pictures\Bento)
$text = [regex]::Replace($text, '(?m)^[ \t]*- \{ title: "Swap OS".*\r?\n', '')
$text = [regex]::Replace($text, 'image_path:\s*"[^"]*"', 'image_path: "{{HOME}}/Pictures/Bento"')
Write-Text $tpl $text
foreach ($f in 'styles.css', 'yasb_colors.css') { Copy-Clean (Join-Path $cfg "yasb\$f") (Join-Path $repo "config\yasb\$f") }
foreach ($f in 'theme_colors.css', 'yasb_mono.css') { Copy-Clean (Join-Path $cfg "yasb\themes\$f") (Join-Path $repo "config\yasb\themes\$f") }

# --- komorebi, whkd, scroll hotkeys ---
$k = [System.IO.File]::ReadAllText((Join-Path $cfg 'komorebi\komorebi.json'))
$k = [regex]::Replace($k, '"display_index_preferences"\s*:\s*\{[^}]*\}\s*,?\s*', '')
# the apps this one person pins to workspaces (launchers, DAWs, chat) are not a sensible default for anyone else: keep the workspaces, drop their rules
$kj = $k | ConvertFrom-Json
foreach ($m in $kj.monitors) { foreach ($w in $m.workspaces) { $w.PSObject.Properties.Remove('initial_workspace_rules') } }
$k = ($kj | ConvertTo-Json -Depth 40)
Write-Text (Join-Path $repo 'config\komorebi\komorebi.json') $k
foreach ($f in 'applications.json', 'komorebi-scroll.ahk', 'game-mode.ps1') { Copy-Clean (Join-Path $cfg "komorebi\$f") (Join-Path $repo "config\komorebi\$f") }
Copy-Clean (Join-Path $cfg 'whkdrc') (Join-Path $repo 'config\whkdrc')

# --- Tacky Borders ---
Copy-Clean (Join-Path $cfg 'tacky-borders\config.yaml') (Join-Path $repo 'config\tacky-borders\config.yaml')

# --- startup script ---
$start = [System.IO.File]::ReadAllText((Join-Path $env:USERPROFILE 'Scripts\stagger-startup.ps1'))
# the personal "non-essentials" (launchers and utilities one person happens to use) stay out of the shared script
$start = [regex]::Replace($start, '(?s)# Non-essentials.*?(?=Mark "all started")', '')
Write-Text (Join-Path $repo 'scripts\stagger-startup.ps1') $start

# --- Rainmeter skin ---
$skin = Join-Path $env:USERPROFILE 'Documents\Rainmeter\Skins\YasbBento'
if (Test-Path -LiteralPath $skin) {
    $dest = Join-Path $repo 'rainmeter\YasbBento'
    if (Test-Path -LiteralPath $dest) { Remove-Item -LiteralPath $dest -Recurse -Force }
    Copy-Item -LiteralPath $skin -Destination $dest -Recurse -Force
}

# --- the settings app (source only, no build output) ---
$src = Join-Path $cfg 'yasb-settings'
$dst = Join-Path $repo 'settings-app'
if (Test-Path -LiteralPath $src) {
    if (Test-Path -LiteralPath $dst) { Remove-Item -LiteralPath $dst -Recurse -Force }
    New-Item -ItemType Directory -Force -Path $dst | Out-Null
    foreach ($item in 'src', 'assets', '.cargo', 'Cargo.toml', 'Cargo.lock') {
        $p = Join-Path $src $item
        if (Test-Path -LiteralPath $p) { Copy-Item -LiteralPath $p -Destination $dst -Recurse -Force }
    }
}
"Repository updated: $repo"
