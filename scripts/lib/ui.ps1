# Small terminal UI for the Bento installer: a banner, single choice and checklist menus, a y/n key prompt and aligned status lines.
# Works in Windows PowerShell 5.1 and PowerShell 7. ASCII only in the source (glyphs are built from char codes), so the file reads the same everywhere.

$script:Esc = [char]27
$script:Dot = [string][char]0x25CF      # a filled dot
$script:Ring = [string][char]0x25CB     # an empty dot
$script:Arrow = [string][char]0x203A    # a thin arrow
$script:Line = [string][char]0x2500
$script:Interactive = $true
try { if ([Console]::IsInputRedirected -or $Host.Name -notmatch 'ConsoleHost|Visual Studio Code Host|Windows Terminal') { $script:Interactive = $false } } catch { $script:Interactive = $false }
try { [Console]::OutputEncoding = [System.Text.Encoding]::UTF8 } catch { }

function Ansi([string]$Code, [string]$Text) { return "$($script:Esc)[$($Code)m$Text$($script:Esc)[0m" }
function Dim([string]$Text) { return (Ansi '38;5;244' $Text) }
function Mute([string]$Text) { return (Ansi '38;5;250' $Text) }
function White([string]$Text) { return (Ansi '1;38;5;255' $Text) }
function Soft([string]$Text) { return (Ansi '38;5;180' $Text) }   # the one warm accent

function Show-Banner([string]$Version) {
    $rows = @(
        '  ###   ####  #   # #####  ###  ',
        '  #  #  #     ##  #   #   #   # ',
        '  ###   ###   # # #   #   #   # ',
        '  #  #  #     #  ##   #   #   # ',
        '  ###   ####  #   #   #    ###  '
    )
    Write-Host ''
    foreach ($r in $rows) {
        $line = ''
        foreach ($c in $r.ToCharArray()) { if ($c -eq '#') { $line += (White $script:Dot) + ' ' } else { $line += (Dim '.') + ' ' } }
        Write-Host $line
    }
    Write-Host ''
    Write-Host ('  ' + (Mute 'A monochrome Windows desktop: komorebi, YASB and the Lucent accent.') + '  ' + (Dim $Version))
    Write-Host ('  ' + (Dim ($script:Line * 62)))
    Write-Host ''
}

function Write-Title([string]$Text) {
    Write-Host ''
    Write-Host ('  ' + (White $Text.ToUpper()))
    Write-Host ('  ' + (Dim ($script:Line * 62)))
}
function Write-Step([string]$Text) { Write-Host ''; Write-Host ('  ' + (Soft $script:Dot) + ' ' + (White $Text)) }
function Write-Info([string]$Text) { Write-Host ('    ' + (Mute $Text)) }
function Write-Ok([string]$Text) { Write-Host ('    ' + (Mute $script:Dot) + ' ' + $Text) }
function Write-Skip([string]$Text) { Write-Host ('    ' + (Dim ($script:Ring + ' ' + $Text))) }
function Write-Warn2([string]$Text) { Write-Host ('    ' + (Soft '!') + ' ' + $Text) }

function Read-Key { return [Console]::ReadKey($true) }

# One choice out of a few. $Options: array of @{ Id; Label; Hint }. Returns the Id.
function Select-One {
    param([string]$Title, [object[]]$Options, [int]$Default = 0)
    Write-Title $Title
    if (-not $script:Interactive) {
        for ($i = 0; $i -lt $Options.Count; $i++) { Write-Host ('    {0}  {1}' -f ($i + 1), $Options[$i].Label) }
        $n = Read-Host '    Number'
        $idx = 0; if ([int]::TryParse($n, [ref]$idx) -and $idx -ge 1 -and $idx -le $Options.Count) { return $Options[$idx - 1].Id }
        return $Options[$Default].Id
    }
    $pos = $Default
    $top = [Console]::CursorTop
    $draw = {
        [Console]::SetCursorPosition(0, $top)
        for ($i = 0; $i -lt $Options.Count; $i++) {
            $o = $Options[$i]
            if ($i -eq $pos) { $mark = (Soft $script:Arrow); $label = White $o.Label } else { $mark = ' '; $label = Mute $o.Label }
            Write-Host ("`r    $mark  $label" + (' ' * 8))
            Write-Host ("`r       " + (Dim $o.Hint) + (' ' * 20))
        }
    }
    [Console]::CursorVisible = $false
    & $draw
    while ($true) {
        $k = Read-Key
        if ($k.Key -eq 'UpArrow') { if ($pos -gt 0) { $pos-- } else { $pos = $Options.Count - 1 } }
        elseif ($k.Key -eq 'DownArrow') { if ($pos -lt $Options.Count - 1) { $pos++ } else { $pos = 0 } }
        elseif ($k.Key -eq 'Enter') { break }
        elseif ($k.KeyChar -ge '1' -and $k.KeyChar -le [char]([int][char]'0' + $Options.Count)) { $pos = [int]::Parse([string]$k.KeyChar) - 1; & $draw; break }
        & $draw
    }
    [Console]::CursorVisible = $true
    return $Options[$pos].Id
}

# A checklist. $Items: array of @{ Id; Label; Hint; On; Locked }. Space toggles, A selects all, N none, Enter confirms. Returns the ids that are on.
function Select-Many {
    param([string]$Title, [object[]]$Items)
    Write-Title $Title
    if (-not $script:Interactive) {
        $ids = @()
        foreach ($it in $Items) {
            if ($it.Locked) { $ids += $it.Id; continue }
            $a = Read-Host ("    {0} ({1}) [Y/n]" -f $it.Label, $it.Hint)
            if ($a -notmatch '^(n|no)$') { $ids += $it.Id }
        }
        return $ids
    }
    $state = @{}
    foreach ($it in $Items) { $state[$it.Id] = [bool]($it.On -or $it.Locked) }
    $pos = 0
    Write-Host ('    ' + (Dim 'Up/Down move   Space switches   A all   N none   Enter continues'))
    Write-Host ''
    $top = [Console]::CursorTop
    $draw = {
        [Console]::SetCursorPosition(0, $top)
        for ($i = 0; $i -lt $Items.Count; $i++) {
            $it = $Items[$i]
            $box = if ($state[$it.Id]) { Soft $script:Dot } else { Dim $script:Ring }
            $cursor = if ($i -eq $pos) { Soft $script:Arrow } else { ' ' }
            $label = if ($it.Locked) { Dim ($it.Label + '  (needed)') } elseif ($i -eq $pos) { White $it.Label } else { Mute $it.Label }
            Write-Host ("`r    $cursor $box  $label" + (' ' * 6))
            Write-Host ("`r         " + (Dim $it.Hint) + (' ' * 20))
        }
    }
    [Console]::CursorVisible = $false
    & $draw
    while ($true) {
        $k = Read-Key
        if ($k.Key -eq 'UpArrow') { if ($pos -gt 0) { $pos-- } else { $pos = $Items.Count - 1 } }
        elseif ($k.Key -eq 'DownArrow') { if ($pos -lt $Items.Count - 1) { $pos++ } else { $pos = 0 } }
        elseif ($k.Key -eq 'Spacebar') { $it = $Items[$pos]; if (-not $it.Locked) { $state[$it.Id] = -not $state[$it.Id] } }
        elseif ($k.KeyChar -eq 'a' -or $k.KeyChar -eq 'A') { foreach ($it in $Items) { $state[$it.Id] = $true } }
        elseif ($k.KeyChar -eq 'n' -or $k.KeyChar -eq 'N') { foreach ($it in $Items) { if (-not $it.Locked) { $state[$it.Id] = $false } } }
        elseif ($k.Key -eq 'Enter') { break }
        & $draw
    }
    [Console]::CursorVisible = $true
    return @($Items | Where-Object { $state[$_.Id] } | ForEach-Object { $_.Id })
}

# Waits for y or n on one key press (Enter takes the default).
function Confirm-Key {
    param([string]$Question, [bool]$Default = $true)
    $hint = if ($Default) { 'Y/n' } else { 'y/N' }
    if (-not $script:Interactive) {
        $a = Read-Host ("    $Question [$hint]")
        if ([string]::IsNullOrWhiteSpace($a)) { return $Default }
        return ($a -match '^(y|yes)$')
    }
    Write-Host -NoNewline ('    ' + (White $Question) + ' ' + (Dim "[$hint] "))
    while ($true) {
        $k = Read-Key
        if ($k.KeyChar -eq 'y' -or $k.KeyChar -eq 'Y') { Write-Host 'y'; return $true }
        if ($k.KeyChar -eq 'n' -or $k.KeyChar -eq 'N') { Write-Host 'n'; return $false }
        if ($k.Key -eq 'Enter') { Write-Host $(if ($Default) { 'y' } else { 'n' }); return $Default }
    }
}
