#!/usr/bin/env python3
"""Frozen/desktop entry point for Omschrift Odysseus (PyInstaller)."""
from __future__ import annotations

import argparse
import json
import os
import sys


def _is_frozen() -> bool:
    return bool(getattr(sys, "frozen", False))


def _app_root() -> str:
    if _is_frozen():
        return getattr(sys, "_MEIPASS", os.path.dirname(sys.executable))
    return os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def _default_data_dir() -> str:
    if _is_frozen():
        base = os.environ.get("LOCALAPPDATA") or os.path.expanduser("~")
        return os.path.join(base, "com.odysseus.desktop", "data")
    return os.path.join(_app_root(), "data")


def _configure_environment() -> str:
    root = _app_root()
    os.chdir(root)
    if root not in sys.path:
        sys.path.insert(0, root)

    data_dir = os.environ.get("ODYSSEUS_DATA_DIR", _default_data_dir())
    os.makedirs(data_dir, exist_ok=True)
    os.environ["ODYSSEUS_DATA_DIR"] = data_dir
    os.environ.setdefault("DATABASE_URL", f"sqlite:///{os.path.join(data_dir, 'app.db')}")
    os.environ.setdefault("HF_HUB_DISABLE_SYMLINKS", "1")
    os.environ.setdefault("HF_HUB_DISABLE_SYMLINKS_WARNING", "1")
    if os.name == "nt":
        os.environ.setdefault("ODYSSEUS_SKIP_ADMIN_PROMPT", "1")
    return data_dir


def _creds_file(data_dir: str) -> str:
    return os.path.join(data_dir, ".desktop-setup-creds.json")


def _needs_setup(data_dir: str) -> bool:
    auth_path = os.path.join(data_dir, "auth.json")
    db_path = os.path.join(data_dir, "app.db")
    return not os.path.isfile(auth_path) and not os.path.isfile(db_path)


def _capture_setup_credentials(data_dir: str) -> None:
    auth_path = os.path.join(data_dir, "auth.json")
    if os.path.isfile(auth_path):
        return

    import secrets

    import bcrypt

    username = os.environ.get("ODYSSEUS_ADMIN_USER", "").strip().lower() or "admin"
    password = os.environ.get("ODYSSEUS_ADMIN_PASSWORD", "").strip() or secrets.token_urlsafe(18)
    hashed = bcrypt.hashpw(password.encode(), bcrypt.gensalt()).decode()

    auth_data = {"users": {username: {"password_hash": hashed, "is_admin": True}}}
    with open(auth_path, "w", encoding="utf-8") as handle:
        json.dump(auth_data, handle, indent=2)

    creds = {"username": username, "password": password}
    with open(_creds_file(data_dir), "w", encoding="utf-8") as handle:
        json.dump(creds, handle)

    print(f"  [ok] Initial admin user created ({username})")
    print(f"        Temporary password: {password}")


def _bootstrap_data_dir(data_dir: str) -> None:
    if not _needs_setup(data_dir):
        return

    import setup

    setup.create_dirs()
    setup.init_database()
    try:
        setup.create_env()
    except Exception:
        pass
    _capture_setup_credentials(data_dir)


def run_setup_only() -> int:
    data_dir = _configure_environment()
    _bootstrap_data_dir(data_dir)
    print(json.dumps({"data_dir": data_dir, "ready": True}))
    return 0


def run_server(host: str, port: int) -> int:
    data_dir = _configure_environment()
    _bootstrap_data_dir(data_dir)

    try:
        import app
    except ImportError as exc:
        print(f"ERROR: Failed to import Odysseus app module: {exc}", file=sys.stderr)
        print(f"  sys.path={sys.path}", file=sys.stderr)
        print(f"  _MEIPASS={getattr(sys, '_MEIPASS', None)}", file=sys.stderr)
        return 1

    import uvicorn

    uvicorn.run(app.app, host=host, port=port, log_level="info")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description="Omschrift Odysseus desktop server")
    parser.add_argument("--setup-only", action="store_true", help="Initialize data dir and exit")
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=7000)
    args = parser.parse_args()

    if args.setup_only:
        return run_setup_only()
    return run_server(args.host, args.port)


if __name__ == "__main__":
    raise SystemExit(main())
