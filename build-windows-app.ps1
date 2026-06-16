#Requires -Version 5.1
<#
  Build the Odysseus Windows desktop app (Tauri + WebView2).

  Produces an installer under:
    desktop\src-tauri\target\release\bundle\nsis\*.exe
    desktop\src-tauri\target\release\bundle\msi\*.msi

  Usage:
    powershell -ExecutionPolicy Bypass -File .\build-windows-app.ps1
    powershell -ExecutionPolicy Bypass -File .\build-windows-app.ps1 -Dev

  Prerequisites (installed automatically when possible):
    - Node.js 18+
    - Rust toolchain (rustup)
    - WebView2 Runtime (preinstalled on Windows 10/11)
    - Python 3.11+ (for the Odysseus backend at runtime, not for this build)
#>
param(
    [switch]$Dev,
    [switch]$SkipIcon,
    [switch]$NativeBundle
)

$ErrorActionPreference = "Stop"
$RepoRoot = $PSScriptRoot
$DesktopDir = Join-Path $RepoRoot "desktop"

function Write-Step($msg) {
    Write-Host ""
    Write-Host ("==> " + $msg) -ForegroundColor Cyan
}

function Fail($msg) {
    Write-Host ""
    Write-Host ("ERROR: " + $msg) -ForegroundColor Red
    exit 1
}

function Set-TauriBundleResources {
    param(
        [string]$TauriConfPath,
        [hashtable]$Resources
    )
    $conf = Get-Content $TauriConfPath -Raw | ConvertFrom-Json
    $conf.bundle.resources = $Resources
    $json = $conf | ConvertTo-Json -Depth 20
    $utf8NoBom = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($TauriConfPath, $json, $utf8NoBom)
}

function Apply-WindowsCodeSigning {
    param([string]$TauriConfPath)

    $thumb = $env:TAURI_SIGNING_CERT_THUMBPRINT
    if (-not $thumb) {
        Write-Host "Code signing skipped (set TAURI_SIGNING_CERT_THUMBPRINT to sign installers)." -ForegroundColor Yellow
        return
    }

    $timestamp = $env:TAURI_SIGNING_TIMESTAMP_URL
    if (-not $timestamp) {
        $timestamp = "http://timestamp.digicert.com"
    }

    $conf = Get-Content $TauriConfPath -Raw | ConvertFrom-Json
    if (-not $conf.bundle.windows) {
        $conf.bundle | Add-Member -NotePropertyName windows -NotePropertyValue ([PSCustomObject]@{})
    }
    $conf.bundle.windows | Add-Member -NotePropertyName certificateThumbprint -NotePropertyValue $thumb -Force
    $conf.bundle.windows | Add-Member -NotePropertyName digestAlgorithm -NotePropertyValue "sha256" -Force
    $conf.bundle.windows | Add-Member -NotePropertyName timestampUrl -NotePropertyValue $timestamp -Force

    $json = $conf | ConvertTo-Json -Depth 20
    $utf8NoBom = New-Object System.Text.UTF8Encoding $false
    [System.IO.File]::WriteAllText($TauriConfPath, $json, $utf8NoBom)
    Write-Host ("Code signing enabled (thumbprint {0})." -f $thumb)
}

function Ensure-Command($name, $installHint) {
    if (-not (Get-Command $name -ErrorAction SilentlyContinue)) {
        Fail ("Required command '$name' was not found. " + $installHint)
    }
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

Write-Step "Checking MSVC build tools"
if (-not (Invoke-VcVars)) {
    Fail "Visual Studio Build Tools with the C++ workload are required. Install from https://visualstudio.microsoft.com/visual-cpp-build-tools/ and include 'Desktop development with C++'."
}
Write-Step "Checking Node.js"
Ensure-Command "node" "Install Node.js 18+ from https://nodejs.org/"
Ensure-Command "npm" "Install Node.js 18+ from https://nodejs.org/"
Write-Host ("Node " + (node --version))

Write-Step "Checking Rust toolchain"
if (-not (Get-Command "cargo" -ErrorAction SilentlyContinue)) {
    Write-Host "Rust not found. Installing via rustup (one-time)…" -ForegroundColor Yellow
    $rustup = Join-Path $env:TEMP "rustup-init.exe"
    Invoke-WebRequest -Uri "https://win.rustup.rs/x86_64" -OutFile $rustup
    & $rustup -y --default-toolchain stable --profile minimal
    if ($LASTEXITCODE -ne 0) { Fail "rustup installation failed." }
    $cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
    $env:Path = "$cargoBin;" + $env:Path
}
Ensure-Command "cargo" "Restart the terminal after installing Rust, then re-run this script."
Write-Host ("Cargo " + (cargo --version))

Write-Step "Installing desktop npm dependencies"
Push-Location $DesktopDir
try {
    npm install
    if ($LASTEXITCODE -ne 0) { Fail "npm install failed in desktop/." }

    if (-not $SkipIcon) {
        Write-Step "Generating Tauri icon set"
        $sourceIcon = Join-Path $DesktopDir "icons\icon-512.png"
        if (-not (Test-Path $sourceIcon)) {
            $sourceIcon = Join-Path $RepoRoot "static\icons\icon-512.png"
        }
        if (Test-Path $sourceIcon) {
            npx tauri icon $sourceIcon
            if ($LASTEXITCODE -ne 0) { Fail "tauri icon generation failed." }
        } else {
            Write-Host "WARNING: icon-512.png not found; using placeholder icons." -ForegroundColor Yellow
        }
    }

    if (-not $Dev) {
        if ($NativeBundle) {
            Write-Step "Using PyInstaller native server bundle"
            $nativeExe = Join-Path $DesktopDir "src-tauri\bundle\omschrift-odysseus-server\omschrift-odysseus-server.exe"
            if (-not (Test-Path $nativeExe)) {
                Fail "Native server bundle not found. Run build-native-windows.ps1 -SkipTauri first."
            }
            $tauriConfPath = Join-Path $DesktopDir "src-tauri\tauri.conf.json"
            Set-TauriBundleResources -TauriConfPath $tauriConfPath -Resources @{
                "bundle/omschrift-odysseus-server/" = "omschrift-odysseus-server/"
            }
        } else {
            Write-Step "Staging Odysseus backend for installer bundle"
            & (Join-Path $RepoRoot "scripts\stage-desktop-bundle.ps1") -RepoRoot $RepoRoot
            if ($LASTEXITCODE -ne 0) { Fail "Backend staging failed." }
            $tauriConfPath = Join-Path $DesktopDir "src-tauri\tauri.conf.json"
            Set-TauriBundleResources -TauriConfPath $tauriConfPath -Resources @{
                "bundle/odysseus/" = "odysseus/"
            }
        }
        $tauriConfPath = Join-Path $DesktopDir "src-tauri\tauri.conf.json"
        Apply-WindowsCodeSigning -TauriConfPath $tauriConfPath
    }

    if ($Dev) {
        Write-Step "Starting Odysseus desktop in development mode"
        Write-Host "The Python backend must be reachable from the repo root."
        Write-Host "Press Ctrl+C to stop."
        npx tauri dev
        if ($LASTEXITCODE -ne 0) { Fail "tauri dev failed." }
    } else {
        Write-Step "Building Odysseus desktop installer (release)"
        npm run build
        if ($LASTEXITCODE -ne 0) { Fail "tauri build failed." }

        $bundleRoot = Join-Path $DesktopDir "src-tauri\target\release\bundle"
        Write-Host ""
        Write-Host "Build complete." -ForegroundColor Green
        if (Test-Path $bundleRoot) {
            Get-ChildItem -Path $bundleRoot -Recurse -Include *.exe, *.msi | ForEach-Object {
                Write-Host ("  " + $_.FullName)
            }
        }
        Write-Host ""
        Write-Host "Install the NSIS .exe or MSI, then launch Omschrift Odysseus from the Start Menu."
        if ($NativeBundle) {
            Write-Host "This installer bundles Python - no separate Python install is required."
        } else {
            Write-Host "On first run the app creates venv/, installs Python deps, and starts the server."
        }
    }
}
finally {
    Pop-Location
}
