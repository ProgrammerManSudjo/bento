# stagger-startup.ps1
# Priority order: komorebi, YASB, Rainmeter. Everything else follows. (komotray is no longer started.)
# Shortcuts (.lnk) are launched from C:\Users\<you>\StartupDelayed
# Problems are logged to %TEMP%\stagger-startup.log

$delayed = "$env:USERPROFILE\StartupDelayed"
$log = "$env:TEMP\stagger-startup.log"

# Where things are: found on PATH first, then in the usual places (scoop, Program Files, per-user installs). Nothing here names a user.
$scoop = if ($env:SCOOP) { $env:SCOOP } else { "$env:USERPROFILE\scoop" }
function Find-Exe {
    param([string]$Name, [string[]]$Candidates = @())
    $found = Get-Command $Name -CommandType Application -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($found) { return $found.Source }
    foreach ($c in $Candidates) { if (Test-Path -LiteralPath $c) { return $c } }
    return $Name   # not found: Start-Item will log it as missing
}
$komorebic = Find-Exe "komorebic-no-console.exe" @("$env:ProgramFiles\komorebi\bin\komorebic-no-console.exe", "$scoop\apps\komorebi\current\komorebic-no-console.exe")
$komorebicCli = Find-Exe "komorebic.exe" @("$env:ProgramFiles\komorebi\bin\komorebic.exe", "$scoop\apps\komorebi\current\komorebic.exe")

# Timing trace (seconds since this script started and since boot), rewritten every run: %TEMP%\stagger-timing.log
$t0 = Get-Date
$bootAge = [int]((Get-Date) - (Get-CimInstance Win32_OperatingSystem).LastBootUpTime).TotalSeconds
Set-Content "$env:TEMP\stagger-timing.log" "script started $bootAge s after boot"
function Mark([string]$what) { Add-Content "$env:TEMP\stagger-timing.log" ("{0,6:N1} s  {1}" -f ((Get-Date) - $t0).TotalSeconds, $what) }

# Seconds to wait before starting, so explorer and the taskbar can load.
# Lower it (even to 0) if everything starts reliably.
$InitialDelay = 0

function Start-Item {
    param(
        [string]$Path,
        [string]$ArgList = "",
        [int]$DelayAfter = 0
    )
    if (Test-Path -LiteralPath $Path) {
        if ($ArgList) {
            Start-Process -FilePath $Path -ArgumentList $ArgList
        } else {
            Start-Process -FilePath $Path
        }
    } else {
        Add-Content $log "Missing: $Path"
    }
    if ($DelayAfter -gt 0) { Start-Sleep -Seconds $DelayAfter }
}

# Polls until a process with this name exists (without .exe), or the timeout hits.
function Wait-ForProcess {
    param([string]$Name, [int]$TimeoutSec = 10)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        if (Get-Process -Name $Name -ErrorAction SilentlyContinue) { return }
        Start-Sleep -Milliseconds 200
    }
    Add-Content $log "Timed out waiting for process: $Name"
}

if ($InitialDelay -gt 0) { Start-Sleep -Seconds $InitialDelay }

# This runs from a logon task, i.e. before Explorer has finished building the desktop. Programs that need the taskbar
# (YASB, AutoHotkey's tray icon, komorebi's window tracking) misbehave if they start earlier, so wait for it.
Add-Type -Namespace Native -Name Win -MemberDefinition '[DllImport("user32.dll", CharSet=CharSet.Unicode)] public static extern IntPtr FindWindow(string cls, string title);'
function Wait-ForTaskbar {
    param([int]$TimeoutSec = 40)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        if ([Native.Win]::FindWindow("Shell_TrayWnd", $null) -ne [IntPtr]::Zero) { return }
        Start-Sleep -Milliseconds 250
    }
    Add-Content $log "Timed out waiting for the taskbar"
}

# komorebi creates its process before it listens on its socket; YASB and the hotkeys only work once it answers.
function Wait-ForKomorebi {
    param([int]$TimeoutSec = 30)
    $deadline = (Get-Date).AddSeconds($TimeoutSec)
    while ((Get-Date) -lt $deadline) {
        & $komorebicCli state *> $null
        if ($LASTEXITCODE -eq 0) { return }
        Start-Sleep -Milliseconds 300
    }
    Add-Content $log "Timed out waiting for komorebi to answer"
}

Wait-ForTaskbar
Mark "taskbar is up"

# komorebi can't start if a stale komorebi.sock (left by an unclean exit) survives a reboot:
# Windows refuses to open or delete it (error 1920), so sidestep it by renaming its folder.
if (-not (Get-Process komorebi -ErrorAction SilentlyContinue)) {
    $kd = "$env:LOCALAPPDATA\komorebi"
    if (Test-Path -LiteralPath "$kd\komorebi.sock") {
        try { [System.IO.File]::Delete("$kd\komorebi.sock") }
        catch {
            try {
                Rename-Item -LiteralPath $kd -NewName "komorebi.stale-$(Get-Date -Format yyyyMMddHHmmss)" -ErrorAction Stop
                Add-Content $log "Renamed stale komorebi dir"
            } catch { Add-Content $log "Could not clear stale komorebi.sock: $($_.Exception.Message)" }
        }
    }
}

# --- Priority 1: komorebi (with whkd, the hotkeys) ---
Mark "starting komorebi"
Start-Item $komorebic -ArgList "start --whkd"
Wait-ForProcess "komorebi" 15
Mark "komorebi process exists"
Wait-ForKomorebi                 # YASB needs its socket, not just its process
Mark "komorebi answers"

# Focus follows mouse is OFF for now. masir (installed in Program Files\masir) is the tool for it; to turn it back on, uncomment:
# if (-not (Get-Process masir -ErrorAction SilentlyContinue)) {
#     # masir is a console program: run it through a headless conhost so no terminal window appears.
#     Start-Item "$env:WINDIR\System32\conhost.exe" -ArgList "--headless `"$env:ProgramFiles\masir\bin\masir.exe`""
# }

# --- Scroll hotkeys: on again if the settings app left them on (Home > Scroll hotkeys) ---
$yasbSettings = "$env:LOCALAPPDATA\yasb-settings\settings.txt"
if ((Test-Path -LiteralPath $yasbSettings) -and (Select-String -LiteralPath $yasbSettings -Pattern '^scroll_hotkeys=1' -Quiet)) {
    $ahk = Find-Exe "AutoHotkey64.exe" @("$env:LOCALAPPDATA\Programs\AutoHotkey\v2\AutoHotkey64.exe", "$env:ProgramFiles\AutoHotkey\v2\AutoHotkey64.exe", "$scoop\apps\autohotkey\current\v2\AutoHotkey64.exe", "$scoop\apps\autohotkey\current\AutoHotkey64.exe")
    Start-Item $ahk -ArgList "`"$env:USERPROFILE\.config\komorebi\komorebi-scroll.ahk`""
    Mark "scroll hotkeys started"
}

# --- Priority 2: YASB ---
$yasbExe = "$scoop\apps\yasb\current\yasb.exe"
if (-not (Test-Path -LiteralPath $yasbExe)) { $yasbExe = Find-Exe "yasb.exe" @("$env:ProgramFiles\YASB\yasb.exe") }
Start-Item $yasbExe
Wait-ForProcess "yasb" 10
Mark "yasb process exists"

# --- Priority 3: Rainmeter ---
Start-Item (Find-Exe "Rainmeter.exe" @("$env:ProgramFiles\Rainmeter\Rainmeter.exe", "${env:ProgramFiles(x86)}\Rainmeter\Rainmeter.exe"))

# --- Everything else, short gaps ---
# Depends on komorebi
# Start-Item "$delayed\Komorebi Scroll.lnk" -DelayAfter 1

# Window borders (the High impact one)
Start-Item (Find-Exe "tacky-borders.exe" @("$scoop\apps\tacky-borders\current\tacky-borders.exe")) -DelayAfter 2

Mark "all started"

# Optional, only if you want it at login:
# Start-Item "$delayed\Playnite.lnk"


