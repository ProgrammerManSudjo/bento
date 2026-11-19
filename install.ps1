<#
.SYNOPSIS
  Bento: installs the monochrome komorebi + YASB desktop, with menus.

.DESCRIPTION
  Asks whether to install the whole suite or to pick the pieces, then sets up Scoop and what you chose (komorebi, whkd, YASB, AutoHotkey, Tacky Borders,
  Rainmeter, AltSnap, fastfetch, Spicetify, Windhawk), copies the configuration (anything it would overwrite is backed up first), installs the Bento Settings
  app, wallpapers, and registers the start at sign-in. Safe to run again. Run it as a normal user, not as administrator.

  One line, on a fresh PC:   irm https://raw.githubusercontent.com/ProgrammerManSudjo/bento/main/install.ps1 | iex

.PARAMETER Preset     Full (the whole suite), Core (komorebi, YASB, the settings app) or Custom (the menu). Without it you are asked.
.PARAMETER Yes        Do not stop to ask anything (uses the preset; Windhawk is skipped because it needs a manual download).
.PARAMETER DryRun     Show what would happen and change nothing.
.PARAMETER SettingsExe  A bento-settings.exe you built yourself (otherwise it is downloaded from the Releases page).
#>
[CmdletBinding()]
param(
    [ValidateSet('Full', 'Core', 'Custom')][string]$Preset,
    [switch]$Yes,
    [switch]$DryRun,
    [switch]$NoStart,
    [string]$SettingsExe,
    [string]$Repo = 'ProgrammerManSudjo/bento'
)

$ErrorActionPreference = 'Stop'
$BentoVersion = 'v1'

# --- run from the web: fetch the package first --------------------------------------------------------------------------------------------
$root = $PSScriptRoot
if (-not $root -or -not (Test-Path -LiteralPath (Join-Path $root 'config'))) {
    Write-Host 'Bento: fetching the package...' -ForegroundColor DarkGray
    $tmp = Join-Path $env:TEMP ('bento-' + [guid]::NewGuid().ToString('N').Substring(0, 8))
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    $zip = Join-Path $tmp 'bento.zip'
    try { Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/$Repo/releases/latest/download/bento.zip" -OutFile $zip }
    catch { Invoke-WebRequest -UseBasicParsing -Uri "https://github.com/$Repo/archive/refs/heads/main.zip" -OutFile $zip }
    Expand-Archive -LiteralPath $zip -DestinationPath $tmp -Force
    $inner = Get-ChildItem -LiteralPath $tmp -Directory | Where-Object { Test-Path (Join-Path $_.FullName 'install.ps1') } | Select-Object -First 1
    $dir = if ($inner) { $inner.FullName } else { $tmp }
    $forward = @{}
    foreach ($k in 'Preset', 'SettingsExe', 'Repo') { if ($PSBoundParameters.ContainsKey($k)) { $forward[$k] = $PSBoundParameters[$k] } }
    foreach ($k in 'Yes', 'DryRun', 'NoStart') { if ($PSBoundParameters.ContainsKey($k)) { $forward[$k] = $true } }
    & (Join-Path $dir 'install.ps1') @forward
    return
}

. (Join-Path $root 'scripts\lib\ui.ps1')
if ($Yes) { $script:Interactive = $false }

$cfg = Join-Path $env:USERPROFILE '.config'
$stamp = Get-Date -Format 'yyyyMMdd-HHmmss'
$notes = New-Object System.Collections.Generic.List[string]
$did = New-Object System.Collections.Generic.List[string]

function Do-It([string]$What, [scriptblock]$Action) {
    if ($DryRun) { Write-Skip "[dry run] $What"; return }
    Write-Info $What
    & $Action
}
function Note([string]$Text) { $notes.Add($Text); Write-Warn2 $Text }

# --- 0. the screen, the checks -------------------------------------------------------------------------------------------------------------
try { Clear-Host } catch { }
Show-Banner $BentoVersion
if ($env:OS -ne 'Windows_NT') { throw 'This is for Windows.' }
$isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
if ($isAdmin) { throw 'Run this from a normal (not administrator) PowerShell: Scoop will not install elevated.' }
$build = [Environment]::OSVersion.Version.Build
if ($build -lt 22000) { Write-Warn2 'This is made for Windows 11. It may work on Windows 10, but the taskbar parts are untested there.' }
Write-Info "Signed in as $env:USERNAME on Windows build $build."
if ($DryRun) { Write-Warn2 'Dry run: nothing will be changed.' }

# --- 1. what to install --------------------------------------------------------------------------------------------------------------------
$catalog = @(
    @{ Id = 'komorebi'; Label = 'komorebi + whkd + AutoHotkey'; Hint = 'Tiling windows, workspaces, hotkeys and the scroll shortcuts'; Core = $true }
    @{ Id = 'yasb'; Label = 'YASB bar'; Hint = 'The floating status bar, with the JetBrainsMono Nerd Font for its icons'; Core = $true }
    @{ Id = 'settings'; Label = 'Bento Settings + configuration'; Hint = 'The settings app (bar, widgets, Lucent) and the whole monochrome config'; Core = $true }
    @{ Id = 'tacky'; Label = 'Tacky Borders'; Hint = 'Coloured, animated window borders' }
    @{ Id = 'rainmeter'; Label = 'Rainmeter + the Bento widget'; Hint = 'The clock, calendar and uptime tiles on the desktop' }
    @{ Id = 'altsnap'; Label = 'AltSnap'; Hint = 'Move and resize any window with Alt + drag' }
    @{ Id = 'fastfetch'; Label = 'fastfetch'; Hint = 'System info in the terminal' }
    @{ Id = 'spices'; Label = 'Spicetify + Comfy theme'; Hint = 'Spotify, themed by Lucent (needs Spotify from spotify.com)' }
    @{ Id = 'wallpapers'; Label = 'Wallpapers'; Hint = 'Six original dot-matrix wallpapers in Pictures\Bento' }
    @{ Id = 'windhawk'; Label = 'Windhawk'; Hint = 'Taskbar and window mods (a manual download: I will wait for you)' }
)

if (-not $Preset) {
    if ($Yes) { $Preset = 'Full' } else {
        $Preset = Select-One -Title 'How much of it?' -Options @(
            @{ Id = 'Full'; Label = 'The whole suite'; Hint = 'Everything below, nothing to decide. Recommended.' }
            @{ Id = 'Custom'; Label = 'Let me choose'; Hint = 'Pick the pieces one by one' }
            @{ Id = 'Core'; Label = 'Just the core'; Hint = 'komorebi, YASB and the settings app' }
        )
    }
}
switch ($Preset) {
    'Full' { $chosen = @($catalog | ForEach-Object { $_.Id }) }
    'Core' { $chosen = @($catalog | Where-Object { $_.Core } | ForEach-Object { $_.Id }) }
    default {
        $items = @($catalog | ForEach-Object { @{ Id = $_.Id; Label = $_.Label; Hint = $_.Hint; On = $true; Locked = [bool]$_.Core } })
        $chosen = Select-Many -Title 'Pick what to install' -Items $items
    }
}
if ($Yes -and ($chosen -contains 'windhawk')) { $chosen = @($chosen | Where-Object { $_ -ne 'windhawk' }); Write-Info 'Windhawk skipped (it needs a manual download; run without -Yes to be guided).' }
function Wants([string]$Id) { return ($chosen -contains $Id) }

if (-not $Yes -and -not $DryRun) {
    Write-Host ''
    if (-not (Confirm-Key 'Go ahead and install?' $true)) { Write-Host '    Stopped. Nothing was changed.'; return }
}

# --- 2. Scoop ------------------------------------------------------------------------------------------------------------------------------
Write-Step 'Scoop'
if (Get-Command scoop -ErrorAction SilentlyContinue) {
    Write-Ok 'Scoop is already installed.'
} else {
    Do-It 'Installing Scoop (it installs into your user folder, no administrator needed)' {
        Set-ExecutionPolicy -Scope CurrentUser -ExecutionPolicy RemoteSigned -Force
        Invoke-RestMethod -Uri 'https://get.scoop.sh' | Invoke-Expression
    }
}
$scoopRoot = if ($env:SCOOP) { $env:SCOOP } else { Join-Path $env:USERPROFILE 'scoop' }
$env:PATH = "$scoopRoot\shims;$env:PATH"

function Scoop-Has([string]$Name) { return (Test-Path -LiteralPath (Join-Path $scoopRoot "apps\$Name\current")) }
function Scoop-Install([string]$Name) {
    if (Scoop-Has $Name) { Write-Ok "$Name (already installed)"; return }
    Do-It "Installing $Name" {
        scoop install $Name 2>&1 | Out-Null
        if ($LASTEXITCODE -ne 0) { Note "scoop could not install $Name" } else { Write-Ok $Name; $did.Add($Name) }
    }
}

if (-not $DryRun -and -not (Get-Command git -ErrorAction SilentlyContinue)) { Scoop-Install 'git' }   # buckets are git repositories
Write-Step 'Buckets'
if (-not $DryRun) {
    $have = (scoop bucket list | Out-String)
    if ($have -notmatch 'extras') { scoop bucket add extras 2>&1 | Out-Null; Write-Ok 'extras' } else { Write-Ok 'extras (already added)' }
    if ((Wants 'yasb') -and $have -notmatch 'nerd-fonts') { scoop bucket add nerd-fonts 2>&1 | Out-Null; Write-Ok 'nerd-fonts' }
} else { Write-Skip '[dry run] scoop bucket add extras / nerd-fonts' }

# --- 3. Programs ---------------------------------------------------------------------------------------------------------------------------
Write-Step 'Programs'
if (Wants 'komorebi') { foreach ($p in 'komorebi', 'whkd', 'autohotkey') { Scoop-Install $p } }
if (Wants 'yasb') { Scoop-Install 'yasb' }
if (Wants 'tacky') { Scoop-Install 'tacky-borders' }
if (Wants 'rainmeter') { Scoop-Install 'rainmeter' }
if (Wants 'altsnap') { Scoop-Install 'altsnap' }
if (Wants 'fastfetch') { Scoop-Install 'fastfetch' }
if (Wants 'spices') { Scoop-Install 'spicetify-cli' }

if (Wants 'yasb') {
    Write-Step 'Font'
    $fontOk = $false
    foreach ($f in 'JetBrainsMono-NF-Propo', 'JetBrainsMono-NF') {
        if ($DryRun) { Write-Skip "[dry run] scoop install nerd-fonts/$f"; $fontOk = $true; break }
        if (Scoop-Has $f) { Write-Ok "$f (already installed)"; $fontOk = $true; break }
        scoop install "nerd-fonts/$f" 2>&1 | Out-Null
        if ($LASTEXITCODE -eq 0) { Write-Ok $f; $fontOk = $true; break }
    }
    if (-not $fontOk) { Note 'Could not install the JetBrainsMono Nerd Font. Install it by hand (nerdfonts.com) or the bar icons will look wrong.' }
}

# --- 4. Windhawk (by hand) ---------------------------------------------------------------------------------------------------------------
if (Wants 'windhawk') {
    Write-Step 'Windhawk'
    $wh = @("$env:ProgramFiles\Windhawk\windhawk-cli.exe", "${env:ProgramFiles(x86)}\Windhawk\windhawk-cli.exe") | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
    if ($wh) { Write-Ok 'Windhawk is already installed.' }
    elseif ($DryRun) { Write-Skip '[dry run] would wait for you to install Windhawk' }
    else {
        Write-Info 'Windhawk changes the taskbar and windows, so it asks for administrator permission and has to be installed by you.'
        Write-Host ''
        Write-Host ('    ' + (White 'Download it here:  ') + (Soft 'https://windhawk.net'))
        Write-Host ''
        try { Start-Process 'https://windhawk.net' } catch { }
        $installed = $false
        while (-not $installed) {
            if (Confirm-Key 'Press y once Windhawk is installed (n skips it)' $true) {
                $wh = @("$env:ProgramFiles\Windhawk\windhawk-cli.exe", "${env:ProgramFiles(x86)}\Windhawk\windhawk-cli.exe") | Where-Object { Test-Path -LiteralPath $_ } | Select-Object -First 1
                if ($wh) { $installed = $true; Write-Ok 'Windhawk found.' } else { Write-Warn2 'I cannot find it yet. Finish the installer, then press y again.' }
            } else { Write-Skip 'Windhawk skipped. You can add it later from windhawk.net.'; break }
        }
    }
    if ($wh -and -not $DryRun -and -not $Yes) {
        if (Confirm-Key 'Install the recommended mods too? (one administrator prompt)' $true) {
            $mods = 'taskbar-empty-space-clicks', 'taskbar-autohide-instant-show', 'windows-11-taskbar-styler', 'windows-11-notification-center-styler', 'windows-11-start-menu-styler', 'custom-corner-radius', 'invisible-borders'
            $script = ($mods | ForEach-Object { "& '$wh' mod install $_ --yes" }) -join "`n"
            $tmpPs = Join-Path $env:TEMP "bento-windhawk-$stamp.ps1"
            Set-Content -LiteralPath $tmpPs -Value $script -Encoding ASCII
            try { Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$tmpPs`""; Write-Ok 'Mods installed (see them in the Windhawk app).' }
            catch { Note 'The administrator prompt was declined, so the Windhawk mods were not installed. Add them from the Windhawk app (see the docs).' }
            Remove-Item -LiteralPath $tmpPs -ErrorAction SilentlyContinue
        }
    }
}

# --- 5. Configuration ----------------------------------------------------------------------------------------------------------------------
if ((Wants 'settings') -or (Wants 'komorebi') -or (Wants 'yasb')) {
    Write-Step 'Configuration'
    Do-It 'Environment: KOMOREBI_CONFIG_HOME, WHKD_CONFIG_HOME' {
        [Environment]::SetEnvironmentVariable('KOMOREBI_CONFIG_HOME', "$cfg\komorebi", 'User')
        [Environment]::SetEnvironmentVariable('WHKD_CONFIG_HOME', $cfg, 'User')
        $env:KOMOREBI_CONFIG_HOME = "$cfg\komorebi"; $env:WHKD_CONFIG_HOME = $cfg
    }
    function Install-File([string]$From, [string]$To) {
        if (-not (Test-Path -LiteralPath $From)) { Note "missing in the package: $From"; return }
        Do-It $To {
            New-Item -ItemType Directory -Force -Path (Split-Path $To) | Out-Null
            if ((Test-Path -LiteralPath $To) -and ((Get-FileHash $From).Hash -ne (Get-FileHash $To).Hash)) { Copy-Item -LiteralPath $To "$To.backup-$stamp" -Force }
            Copy-Item -LiteralPath $From -Destination $To -Force
        }
    }
    if (Wants 'yasb') {
        Do-It 'yasb\config.yaml (made for this user from the template)' {
            New-Item -ItemType Directory -Force -Path "$cfg\yasb" | Out-Null
            $target = "$cfg\yasb\config.yaml"
            if (Test-Path -LiteralPath $target) { Copy-Item -LiteralPath $target "$target.backup-$stamp" -Force }
            & (Join-Path $root 'scripts\Render-Config.ps1') -Target $target | Out-Null
            Remove-Item -LiteralPath "$target.bak" -ErrorAction SilentlyContinue
        }
        foreach ($f in 'styles.css', 'yasb_colors.css') { Install-File "$root\config\yasb\$f" "$cfg\yasb\$f" }
        foreach ($f in 'theme_colors.css', 'yasb_mono.css') { Install-File "$root\config\yasb\themes\$f" "$cfg\yasb\themes\$f" }
    }
    if (Wants 'komorebi') {
        foreach ($f in 'komorebi.json', 'applications.json', 'komorebi-scroll.ahk', 'game-mode.ps1') { Install-File "$root\config\komorebi\$f" "$cfg\komorebi\$f" }
        Install-File "$root\config\whkdrc" "$cfg\whkdrc"
    }
    if (Wants 'tacky') { Install-File "$root\config\tacky-borders\config.yaml" "$cfg\tacky-borders\config.yaml" }
}

# --- 6. The settings app ---------------------------------------------------------------------------------------------------------------------
if (Wants 'settings') {
    Write-Step 'Bento Settings'
    $exeTarget = "$cfg\yasb\yasb-settings.exe"
    if ($SettingsExe) {
        Do-It "Copying $SettingsExe" { New-Item -ItemType Directory -Force -Path (Split-Path $exeTarget) | Out-Null; Copy-Item -LiteralPath $SettingsExe -Destination $exeTarget -Force }
    } elseif (Test-Path -LiteralPath $exeTarget) {
        Write-Ok 'already there'
    } else {
        $url = "https://github.com/$Repo/releases/latest/download/bento-settings.exe"
        Do-It "Downloading $url" {
            New-Item -ItemType Directory -Force -Path (Split-Path $exeTarget) | Out-Null
            try { Invoke-WebRequest -Uri $url -OutFile $exeTarget -UseBasicParsing; Write-Ok 'bento-settings.exe' }
            catch { Note "Could not download the settings app ($($_.Exception.Message)). Build it from settings-app\ (cargo build --release) and run this again with -SettingsExe." }
        }
    }
}

# --- 7. Wallpapers, the Rainmeter skin, Spotify -------------------------------------------------------------------------------------------
if (Wants 'wallpapers') {
    Write-Step 'Wallpapers'
    $wp = Join-Path ([Environment]::GetFolderPath('MyPictures')) 'Bento'
    if (Test-Path -LiteralPath "$root\wallpapers") {
        Do-It "Copying the wallpapers to $wp" { New-Item -ItemType Directory -Force -Path $wp | Out-Null; Copy-Item -Path "$root\wallpapers\*.jpg" -Destination $wp -Force }
        Write-Info 'Pick one in the Settings app (Wallpaper page) or in Windows settings.'
    }
}
if (Wants 'rainmeter') {
    Write-Step 'Rainmeter widget (Bento)'
    $skinTarget = "$env:USERPROFILE\Documents\Rainmeter\Skins\YasbBento"
    if (Test-Path -LiteralPath "$root\rainmeter\YasbBento") {
        Do-It "Copying the skin to $skinTarget" {
            New-Item -ItemType Directory -Force -Path (Split-Path $skinTarget) | Out-Null
            Copy-Item -LiteralPath "$root\rainmeter\YasbBento" -Destination (Split-Path $skinTarget) -Recurse -Force
        }
        Write-Info 'Load it from Rainmeter: Manage > YasbBento > YasbBento.ini > Load (it centres itself on the main screen).'
    }
}
if (Wants 'spices') {
    Write-Step 'Spicetify + Comfy'
    $spicetifyDir = Join-Path $env:APPDATA 'spicetify'
    if ($DryRun) { Write-Skip '[dry run] would add the Comfy theme to Spicetify' }
    elseif (-not (Test-Path -LiteralPath "$env:APPDATA\Spotify\Spotify.exe")) {
        Note 'Spotify (the desktop app from spotify.com, not the Store version) is not installed yet. Install it, then run this again, or add Comfy by hand (see the docs).'
    } else {
        $comfy = Join-Path $spicetifyDir 'Themes\Comfy'
        if (-not (Test-Path -LiteralPath $comfy)) {
            $clone = Join-Path $env:TEMP "bento-comfy-$stamp"
            git clone --depth 1 https://github.com/Comfy-Themes/Spicetify.git $clone 2>&1 | Out-Null
            if (Test-Path "$clone\Comfy") { New-Item -ItemType Directory -Force -Path (Split-Path $comfy) | Out-Null; Copy-Item "$clone\Comfy" $comfy -Recurse -Force; Write-Ok 'Comfy theme added' }
            Remove-Item -LiteralPath $clone -Recurse -Force -ErrorAction SilentlyContinue
        } else { Write-Ok 'Comfy is already there' }
        Write-Info 'Open Bento Settings > Lucent > Apply now: it writes the Bento colour scheme into Comfy and applies it to Spotify.'
    }
}

# --- 8. Start at sign-in ---------------------------------------------------------------------------------------------------------------------
if ((Wants 'komorebi') -or (Wants 'yasb')) {
    Write-Step 'Start at sign-in'
    $startScript = "$cfg\yasb\stagger-startup.ps1"
    Install-File "$root\scripts\stagger-startup.ps1" $startScript
    Do-It 'Scheduled task "Bento Startup" (starts komorebi, YASB, Rainmeter, Tacky Borders in order)' {
        $arg = "--headless `"$env:WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe`" -NoProfile -ExecutionPolicy Bypass -File `"$startScript`""
        $action = New-ScheduledTaskAction -Execute "$env:WINDIR\System32\conhost.exe" -Argument $arg
        $trigger = New-ScheduledTaskTrigger -AtLogOn -User "$env:USERDOMAIN\$env:USERNAME"
        $trigger.Delay = 'PT4S'
        $taskSettings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable -ExecutionTimeLimit (New-TimeSpan -Minutes 10)
        try { Register-ScheduledTask -TaskName 'Bento Startup' -Action $action -Trigger $trigger -Settings $taskSettings -Force | Out-Null; Write-Ok 'task registered' }
        catch { Note "Could not create the sign-in task ($($_.Exception.Message)). Put a shortcut to $startScript in your Startup folder instead." }
    }
}

# --- 9. Start now ----------------------------------------------------------------------------------------------------------------------------
Write-Step 'Starting'
if ($NoStart -or $DryRun -or -not ((Wants 'komorebi') -or (Wants 'yasb'))) { Write-Info 'Not starting anything now (sign out and in, or run the start script).' }
else {
    Write-Info 'Starting komorebi, YASB and the rest...'
    Start-Process -FilePath "$env:WINDIR\System32\WindowsPowerShell\v1.0\powershell.exe" -ArgumentList '-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$cfg\yasb\stagger-startup.ps1`"" -WindowStyle Hidden
}

Write-Host ''
Write-Host ('  ' + (Dim ($script:Line * 62)))
Write-Host ('  ' + (White 'Done.'))
if ($did.Count) { Write-Host ('  ' + (Mute ('Installed: ' + ($did -join ', ')))) }
if ($notes.Count) {
    Write-Host ''
    Write-Host ('  ' + (Soft 'Things to look at:'))
    $notes | ForEach-Object { Write-Host ('    - ' + $_) }
}
Write-Host ''
Write-Host ('  ' + (Mute 'Next'))
Write-Info 'Open the Settings app from the YASB home menu (or ~\.config\yasb\yasb-settings.exe).'
Write-Info 'Lucent page: switch the theme on, pick an accent, press Apply now.'
Write-Info 'Docs and the list of supported programs: https://programmermansudjo.github.io/bento'
Write-Host ''
