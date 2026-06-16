#Requires -Version 5.1
<#
  Stage a minimal Odysseus backend tree for the desktop installer bundle.
  Output: desktop/src-tauri/bundle/odysseus/
#>
param(
    [string]$RepoRoot = (Split-Path -Parent $PSScriptRoot)
)

$ErrorActionPreference = "Stop"
$OutRoot = Join-Path $RepoRoot "desktop\src-tauri\bundle\odysseus"

$PathsToCopy = @(
    "app.py",
    "setup.py",
    "requirements.txt",
    "requirements-optional.txt",
    "pyproject.toml",
    "core",
    "src",
    "routes",
    "services",
    "static",
    "config",
    "mcp_servers",
    "integrations",
    "companion",
    "licenses"
)

if (Test-Path $OutRoot) {
    Remove-Item -Recurse -Force $OutRoot
}
New-Item -ItemType Directory -Force -Path $OutRoot | Out-Null

foreach ($relative in $PathsToCopy) {
    $source = Join-Path $RepoRoot $relative
    if (-not (Test-Path $source)) {
        Write-Warning "Skipping missing path: $relative"
        continue
    }
    $dest = Join-Path $OutRoot $relative
    $parent = Split-Path -Parent $dest
    if ($parent -and -not (Test-Path $parent)) {
        New-Item -ItemType Directory -Force -Path $parent | Out-Null
    }
    if (Test-Path $source -PathType Container) {
        Copy-Item -Path $source -Destination $dest -Recurse -Force
    } else {
        Copy-Item -Path $source -Destination $dest -Force
    }
}

Write-Host "Staged Odysseus backend at $OutRoot"
