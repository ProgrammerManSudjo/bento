# Game mode on/off, for Playnite's global scripts ("before starting a game": game-mode.ps1 on, "after exiting a game": game-mode.ps1 off).
# It only leaves "on" or "off" in a small file; the running komorebi-scroll.ahk picks it up within 2 seconds and does the work
# (komorebi + whkd, YASB, AltSnap, scroll hotkeys, taskbar auto-hide).
param([Parameter(Mandatory)][ValidateSet('on', 'off')][string]$State)
Set-Content -Path (Join-Path $env:APPDATA 'game-mode-flag.txt') -Value $State -NoNewline
