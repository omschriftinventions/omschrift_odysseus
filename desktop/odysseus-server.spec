# -*- mode: python ; coding: utf-8 -*-
"""PyInstaller spec — Omschrift Odysseus native Windows server bundle (onedir)."""
import os
from pathlib import Path

from PyInstaller.utils.hooks import collect_all, collect_submodules

ROOT = Path(SPECPATH).resolve().parent
DESKTOP = ROOT / "desktop"

block_cipher = None

hiddenimports = []
datas = [
    (str(ROOT / "static"), "static"),
    (str(ROOT / "licenses"), "licenses"),
    (str(ROOT / "config"), "config"),
    (str(ROOT / "mcp_servers"), "mcp_servers"),
    (str(ROOT / "integrations"), "integrations"),
    (str(ROOT / "companion"), "companion"),
    (str(ROOT / "services" / "hwfit" / "data"), os.path.join("services", "hwfit", "data")),
]

for pkg in (
    "routes",
    "services",
    "core",
    "src",
    "uvicorn",
    "fastapi",
    "starlette",
    "sqlalchemy",
    "pydantic",
    "chromadb",
    "fastembed",
    "onnxruntime",
    "mcp",
    "caldav",
    "icalendar",
    "nh3",
    "markdown",
    "bcrypt",
    "cryptography",
):
    try:
        hiddenimports.extend(collect_submodules(pkg))
    except Exception:
        pass

for pkg in ("uvicorn", "fastapi", "starlette", "chromadb", "fastembed", "onnxruntime"):
    try:
        tmp = collect_all(pkg)
        datas += tmp[0]
        hiddenimports += tmp[2]
    except Exception:
        pass

for py in ROOT.glob("*.py"):
    hiddenimports.append(py.stem)

hiddenimports += [
    "app",
    "setup",
    "uvicorn.logging",
    "uvicorn.loops",
    "uvicorn.loops.auto",
    "uvicorn.protocols",
    "uvicorn.protocols.http",
    "uvicorn.protocols.http.auto",
    "uvicorn.protocols.websockets",
    "uvicorn.protocols.websockets.auto",
    "uvicorn.lifespan",
    "uvicorn.lifespan.on",
    "engineio.async_drivers.threading",
    "multipart",
    "pkg_resources.py2_warn",
]

a = Analysis(
    [str(DESKTOP / "server_launcher.py")],
    pathex=[str(ROOT)],
    binaries=[],
    datas=datas,
    hiddenimports=hiddenimports,
    hookspath=[],
    hooksconfig={},
    runtime_hooks=[],
    excludes=["pytest", "pytest_asyncio", "httpx2", "tkinter"],
    win_no_prefer_redirects=False,
    win_private_assemblies=False,
    cipher=block_cipher,
    noarchive=False,
)

pyz = PYZ(a.pure, a.zipped_data, cipher=block_cipher)

exe = EXE(
    pyz,
    a.scripts,
    [],
    exclude_binaries=True,
    name="omschrift-odysseus-server",
    debug=False,
    bootloader_ignore_signals=False,
    strip=False,
    upx=False,
    console=True,
    disable_windowed_traceback=False,
    argv_emulation=False,
    target_arch=None,
    codesign_identity=None,
    entitlements_file=None,
)

coll = COLLECT(
    exe,
    a.binaries,
    a.zipfiles,
    a.datas,
    strip=False,
    upx=False,
    upx_exclude=[],
    name="omschrift-odysseus-server",
)
