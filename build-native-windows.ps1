#Requires -Version 5.1
<#
  Build Omschrift Odysseus with a PyInstaller-bundled Python server (no separate
  Python install required at runtime).

  Produces:
    desktop\src-tauri\bundle\omschrift-odysseus-server\   (PyInstaller onedir)
    desktop\src-tauri\target\release\bundle\nsis\*.exe   (Tauri installer)

  Usage:
    powershell -ExecutionPolicy Bypass -File .\build-native-windows.ps1
    powershell -ExecutionPolicy Bypass -File .\build-native-windows.ps1 -SkipTauri
#>
param(
    [switch]$SkipTauri,
    [switch]$SkipIcon
)

$ErrorActionPreference = "Stop"
$RepoRoot = $PSScriptRoot
$DesktopDir = Join-Path $RepoRoot "desktop"
$VenvPy = Join-Path $RepoRoot "venv\Scripts\python.exe"

function Write-Step($msg) {
    Write-Host ""
    Write-Host ("==> " + $msg) -ForegroundColor Cyan
}

function Fail($msg) {
    Write-Host ""
    Write-Host ("ERROR: " + $msg) -ForegroundColor Red
    exit 1
}

function Invoke-VcVars {
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (-not (Test-Path $vswhere)) { return $false }
    $install = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath
    if (-not $install) { return $false }
    $vcvars = Join-Path $install "VC\Auxiliary\Build\vcvars64.bat"
    if (-not (Test-Path $vcvars)) { return $false }
    cmd /c "`"$vcvars`" && set" | ForEach-Object {
        if ($_ -match "^(.*?)=(.*)$") {
            Set-Item -Path "env:$($matches[1])" -Value $matches[2]
        }
    }
    return $true
}

Write-Step "Checking MSVC build tools (for Tauri)"
if (-not $SkipTauri) {
    if (-not (Invoke-VcVars)) {
        Fail "Visual Studio Build Tools with C++ workload are required for the desktop installer."
    }
}

Write-Step "Checking Python venv"
if (-not (Test-Path $VenvPy)) {
    Fail "Run launch-windows.ps1 once to create venv\ before building the native bundle."
}

Write-Step "Installing PyInstaller in venv"
& $VenvPy -m pip install --upgrade pip pyinstaller
if ($LASTEXITCODE -ne 0) { Fail "pip install pyinstaller failed." }

Write-Step "Building PyInstaller server bundle (this can take 10+ minutes)"
& $VenvPy -m PyInstaller (Join-Path $DesktopDir "odysseus-server.spec") --noconfirm --distpath (Join-Path $DesktopDir "pyinstaller-dist") --workpath (Join-Path $DesktopDir "pyinstaller-build")
if ($LASTEXITCODE -ne 0) { Fail "PyInstaller build failed." }

$BuiltDir = Join-Path $DesktopDir "pyinstaller-dist\omschrift-odysseus-server"
if (-not (Test-Path (Join-Path $BuiltDir "omschrift-odysseus-server.exe"))) {
    Fail "Expected omschrift-odysseus-server.exe was not produced."
}

Write-Step "Staging server bundle for Tauri resources"
$StageDir = Join-Path $DesktopDir "src-tauri\bundle\omschrift-odysseus-server"
if (Test-Path $StageDir) { Remove-Item -Recurse -Force $StageDir }
Copy-Item -Recurse -Force $BuiltDir $StageDir

$exe = Get-Item (Join-Path $StageDir "omschrift-odysseus-server.exe")
Write-Host ("Server bundle ready: {0} ({1:N1} MB folder)" -f $StageDir, ((Get-ChildItem $StageDir -Recurse | Measure-Object Length -Sum).Sum / 1MB))

if ($SkipTauri) {
    Write-Host ""
    Write-Host "PyInstaller bundle complete (-SkipTauri)." -ForegroundColor Green
    Write-Host ("Test server: {0} --setup-only" -f $exe.FullName)
    exit 0
}

Write-Step "Building Tauri installer with bundled server"
& (Join-Path $RepoRoot "build-windows-app.ps1") -SkipIcon:$(if ($SkipIcon) { $true } else { $false }) -NativeBundle
if ($LASTEXITCODE -ne 0) { Fail "Tauri build failed." }

Write-Host ""
Write-Host "Native setup build complete." -ForegroundColor Green
Write-Host "Installer includes embedded Python server - users do not need Python installed."
