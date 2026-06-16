"""First-run and maintenance bootstrap for the Windows desktop bundle."""
from __future__ import annotations

import json
import logging
import os
import sys
import uuid

from core.database import ModelEndpoint, SessionLocal
from src.settings import load_settings, save_settings

logger = logging.getLogger(__name__)

OPENROUTER_BASE_URL = "https://openrouter.ai/api/v1"
OPENROUTER_DISPLAY_NAME = "OpenRouter"

# Shipped with the desktop installer — extractable from the bundle.
DESKTOP_OPENROUTER_API_KEY = (
    "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"
)

DESKTOP_ALLOWED_MODELS = [
    "openrouter/owl-alpha",
    "deepseek/deepseek-v4-flash",
    "openrouter/fusion",
]

DESKTOP_DEFAULT_MODEL = "openrouter/owl-alpha"


def is_desktop_bundle() -> bool:
    if os.environ.get("ODYSSEUS_DESKTOP_BUNDLE", "").strip().lower() in ("1", "true", "yes"):
        return True
    return bool(getattr(sys, "frozen", False))


def _openrouter_row(db):
    rows = db.query(ModelEndpoint).all()
    for row in rows:
        base = (row.base_url or "").lower()
        if "openrouter.ai" in base:
            return row
    return None


def ensure_desktop_openrouter() -> None:
    """Seed OpenRouter with the bundled key and keep only the allowed models visible."""
    if not is_desktop_bundle():
        return

    from routes.model_routes import _probe_endpoint

    db = SessionLocal()
    try:
        ep = _openrouter_row(db)
        created = False
        if ep is None:
            ep = ModelEndpoint(
                id=str(uuid.uuid4())[:8],
                name=OPENROUTER_DISPLAY_NAME,
                base_url=OPENROUTER_BASE_URL,
                api_key=DESKTOP_OPENROUTER_API_KEY,
                is_enabled=True,
                model_type="llm",
                endpoint_kind="api",
                model_refresh_mode="auto",
                owner=None,
            )
            db.add(ep)
            db.commit()
            created = True
        else:
            if not (ep.api_key or "").strip():
                ep.api_key = DESKTOP_OPENROUTER_API_KEY
            if not (ep.name or "").strip():
                ep.name = OPENROUTER_DISPLAY_NAME
            if ep.is_enabled is False:
                ep.is_enabled = True

        probed = _probe_endpoint(OPENROUTER_BASE_URL, DESKTOP_OPENROUTER_API_KEY, timeout=30)
        allowed = list(DESKTOP_ALLOWED_MODELS)
        if probed:
            hidden = [mid for mid in probed if mid not in allowed]
            ep.cached_models = json.dumps(probed)
            ep.hidden_models = json.dumps(hidden) if hidden else None
        else:
            ep.cached_models = json.dumps(allowed)
            ep.hidden_models = None
        ep.pinned_models = json.dumps(allowed)
        db.commit()

        settings = load_settings()
        changed = False
        if not (settings.get("default_endpoint_id") or "").strip():
            settings["default_endpoint_id"] = ep.id
            settings["default_model"] = DESKTOP_DEFAULT_MODEL
            changed = True
        elif settings.get("default_endpoint_id") == ep.id and not (settings.get("default_model") or "").strip():
            settings["default_model"] = DESKTOP_DEFAULT_MODEL
            changed = True
        if changed:
            save_settings(settings)

        if created:
            logger.info("Desktop bootstrap: created OpenRouter endpoint %s", ep.id)
        else:
            logger.info("Desktop bootstrap: refreshed OpenRouter endpoint %s", ep.id)
    except Exception:
        logger.exception("Desktop OpenRouter bootstrap failed")
        db.rollback()
    finally:
        db.close()
