# Odysseus Desktop (Windows)

Native Windows shell for Odysseus using **Tauri 2** and **WebView2**.

The desktop app:

1. Finds the Odysseus repository root (`app.py` + `requirements.txt`)
2. Creates `venv/` and installs Python dependencies on first launch
3. Runs `setup.py` when needed
4. Starts the FastAPI server on `http://127.0.0.1:7000`
5. Opens the existing Odysseus web UI in a native window
6. Keeps a system tray icon (close hides to tray; use **Quit** to exit)
7. Stops the Python server when you quit from the tray

## Prerequisites

| Requirement | Notes |
|---|---|
| **Windows 10/11** | WebView2 Runtime (usually preinstalled) |
| **Python 3.11+** | Used at runtime for the backend (`py -3.11` or `python`) |
| **Node.js 18+** | Build-time only |
| **Rust** | Build-time only; `build-windows-app.ps1` can install via rustup |

Optional for full agent/cookbook parity:

- [Git for Windows](https://git-scm.com/download/win) — agent shell tool
- [Ollama](https://ollama.com/download) — local models at `http://localhost:11434/v1`

## Build the installer

From the repository root:

```powershell
powershell -ExecutionPolicy Bypass -File .\build-windows-app.ps1
```

Output:

- `desktop/src-tauri/target/release/bundle/nsis/Odysseus_*_x64-setup.exe`
- `desktop/src-tauri/target/release/bundle/msi/Odysseus_*_x64_en-US.msi`

## Development

```powershell
powershell -ExecutionPolicy Bypass -File .\build-windows-app.ps1 -Dev
```

Or manually:

```powershell
cd desktop
npm install
npx tauri dev
```

Set `ODYSSEUS_ROOT` if the app cannot auto-detect the repository:

```powershell
$env:ODYSSEUS_ROOT = "C:\path\to\odysseus"
```

## Installed app layout

The installer bundles:

1. **Tauri shell** — native window, tray icon, single-instance lock
2. **Odysseus backend source** — copied from this repository into the installer resources

On first launch the app:

1. Materializes the backend into `%LOCALAPPDATA%\com.odysseus.desktop\backend\` (writable)
2. Creates `venv/` there and installs Python dependencies (first run only; can take several minutes)
3. Runs `setup.py` and starts uvicorn on `http://127.0.0.1:7000`
4. Opens the full Odysseus UI in a native WebView2 window

When developing from a git clone (folder contains `desktop/`), the app uses the repository root directly instead of `%LOCALAPPDATA%`.

Override the backend location with:

```powershell
$env:ODYSSEUS_ROOT = "C:\path\to\odysseus"
```

## Logs

| Log | Purpose |
|---|---|
| `logs/odysseus-desktop-server.log` | uvicorn stdout/stderr from the desktop-managed server |
| `logs/` | Standard Odysseus application logs |

## Tray behavior

- **Close window** — hides to tray; server keeps running
- **Tray → Show Odysseus** — restores the window
- **Tray → Quit** — stops the Python server and exits

## Windows SmartScreen / publisher

The installer shows **publisher: Omschrift** in Settings and Add/Remove Programs.

The **"Windows protected your PC" / Run anyway** prompt appears because the installer is **not Authenticode-signed**. Adding a publisher name does not remove SmartScreen; you need a **code signing certificate** from a trusted CA (DigiCert, Sectigo, SSL.com, etc.).

When you have a certificate installed in the Windows cert store:

```powershell
$env:TAURI_SIGNING_CERT_THUMBPRINT = "YOUR_SHA1_THUMBPRINT"
$env:TAURI_SIGNING_TIMESTAMP_URL = "http://timestamp.digicert.com"
powershell -ExecutionPolicy Bypass -File .\build-native-windows.ps1
```

List installed code-signing certs:

```powershell
Get-ChildItem Cert:\CurrentUser\My | Where-Object { $_.EnhancedKeyUsageList -match "Code Signing" } | Format-List Subject, Thumbprint, NotAfter
```

See `desktop/signing.env.example` for more detail. EV certificates usually earn SmartScreen trust faster than standard OV certs.

## License

Odysseus is AGPL-3.0-or-later. Distributing modified builds requires providing corresponding source.
