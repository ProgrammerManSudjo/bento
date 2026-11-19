# Fills in the template for this machine: {{HOME}} becomes this user's profile folder (forward slashes), {{HOME_WIN}} the same with backslashes.
# An existing config.yaml is kept as config.yaml.bak first.
param(
    [string]$Template = (Join-Path $PSScriptRoot "..\config\yasb\config.yaml.template"),
    [string]$Target = "$env:USERPROFILE\.config\yasb\config.yaml"
)
$text = [System.IO.File]::ReadAllText($Template)
$text = $text.Replace('{{HOME}}', $env:USERPROFILE.Replace('\', '/')).Replace('{{HOME_WIN}}', $env:USERPROFILE)
New-Item -ItemType Directory -Force -Path (Split-Path $Target) | Out-Null
if (Test-Path -LiteralPath $Target) { Copy-Item -LiteralPath $Target "$Target.bak" -Force }
[System.IO.File]::WriteAllText($Target, $text, (New-Object System.Text.UTF8Encoding($false)))
"Config written: $Target"
