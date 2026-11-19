# Writes the capsule colours into the Windhawk styler mods (taskbar, start menu, notification center).
# This runs elevated, as a scheduled task that YASB Settings starts on demand. It only takes colours, and checks them: nothing else is read from the user's files.
$ErrorActionPreference = 'Stop'
$data = Join-Path $env:LOCALAPPDATA 'yasb-settings'
$result = Join-Path $data 'windhawk-result.txt'
try {
    $in = Get-Content -LiteralPath (Join-Path $data 'windhawk.json') -Raw | ConvertFrom-Json
    $colours = @{}
    foreach ($k in 'rest', 'active', 'clock') {
        $v = [string]$in.$k
        if ($v -notmatch '^#[0-9a-fA-F]{6}$') { throw "not a colour: $k" }
        $colours[$k] = $v
    }
    # which of the mods' style constants are capsule colours, and which colour each takes
    $names = @{
        ElementSysColor = 'rest'; ButtonAccent = 'rest'; ButtonAccentHover = 'rest'; ButtonAccentPressed = 'rest'; ClockBG = 'rest'
        ElementSysColor4 = 'active'; ButtonAccentActive = 'active'
        ButtonClock = 'clock'
    }
    $cli = Join-Path $env:ProgramFiles 'Windhawk\windhawk-cli.exe'
    $changed = 0
    foreach ($mod in 'windows-11-taskbar-styler', 'windows-11-start-menu-styler', 'windows-11-notification-center-styler') {
        $json = & $cli --json mod settings get $mod | ConvertFrom-Json
        $pairs = @()
        foreach ($p in $json.data.settings.PSObject.Properties) {
            if (-not $p.Name.StartsWith('styleConstants[')) { continue }
            if ($p.Value -match '^(\w+)=<SolidColorBrush Color="#[0-9a-fA-F]{6}"(.*)$' -and $names.ContainsKey($Matches[1])) {
                $new = '{0}=<SolidColorBrush Color="{1}"{2}' -f $Matches[1], $colours[$names[$Matches[1]]], $Matches[2]
                if ($new -ne $p.Value) { $pairs += ('{0}={1}' -f $p.Name, $new.Replace('"', '\"')) }   # (Windows PowerShell drops bare quotes in arguments)
            }
        }
        if ($pairs.Count) {
            & $cli mod settings set $mod @pairs | Out-Null
            if ($LASTEXITCODE -ne 0) { throw "windhawk-cli could not set $mod" }
            $changed += $pairs.Count
        }
    }
    Set-Content -LiteralPath $result -Value "ok $changed"
} catch {
    Set-Content -LiteralPath $result -Value "error: $($_.Exception.Message)"
}
