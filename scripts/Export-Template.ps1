# Turns the live YASB config into a shareable template: the user's profile folder becomes {{HOME}}.
# Run it after changing config.yaml; the template is what the installer renders on another machine.
param(
    [string]$Source = "$env:USERPROFILE\.config\yasb\config.yaml",
    [string]$Target = (Join-Path $PSScriptRoot "..\config\yasb\config.yaml.template")
)
$homeFwd = $env:USERPROFILE.Replace('\', '/')
$text = [System.IO.File]::ReadAllText($Source)
$text = $text.Replace($homeFwd, '{{HOME}}').Replace($env:USERPROFILE, '{{HOME_WIN}}')
New-Item -ItemType Directory -Force -Path (Split-Path $Target) | Out-Null
[System.IO.File]::WriteAllText($Target, $text, (New-Object System.Text.UTF8Encoding($false)))
"Template written: $Target"
