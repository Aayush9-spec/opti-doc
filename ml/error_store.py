"""
OptiDock AI — Persistent Error Knowledge Base

Stores terminal errors, their categories, root causes, and fixes in a local
JSON database. The store grows as the user runs optidock commands and the ML
model learns from new error patterns.
"""

import json
import os
from datetime import datetime
from pathlib import Path
from typing import Optional


STORE_DIR = Path.home() / ".optidock" / "ml"
ERROR_DB = STORE_DIR / "error_knowledge.json"


def _ensure_store():
    STORE_DIR.mkdir(parents=True, exist_ok=True)
    if not ERROR_DB.exists():
        ERROR_DB.write_text(json.dumps({"errors": [], "fixes": []}, indent=2))


def load_store() -> dict:
    _ensure_store()
    return json.loads(ERROR_DB.read_text())


def save_store(store: dict):
    _ensure_store()
    ERROR_DB.write_text(json.dumps(store, indent=2))


def record_error(
    error_text: str,
    category: str,
    source: str = "terminal",
    context: Optional[str] = None,
    fix: Optional[str] = None,
):
    """Record a new error into the knowledge base."""
    store = load_store()
    entry = {
        "id": len(store["errors"]) + 1,
        "timestamp": datetime.now().isoformat(),
        "error_text": error_text.strip(),
        "category": category,
        "source": source,
        "context": context or "",
        "fix": fix or "",
        "resolved": fix is not None,
    }
    store["errors"].append(entry)
    save_store(store)
    return entry


def record_fix(error_id: int, fix_text: str):
    """Record a fix for a previously stored error."""
    store = load_store()
    for err in store["errors"]:
        if err["id"] == error_id:
            err["fix"] = fix_text
            err["resolved"] = True
            break

    store["fixes"].append({
        "error_id": error_id,
        "fix_text": fix_text,
        "timestamp": datetime.now().isoformat(),
    })
    save_store(store)


def get_similar_errors(category: str, limit: int = 5) -> list:
    """Retrieve recent errors of the same category with their fixes."""
    store = load_store()
    matching = [e for e in store["errors"] if e["category"] == category and e["resolved"]]
    return matching[-limit:]


def get_all_training_data() -> list:
    """Get all errors as (text, category) pairs for ML training."""
    store = load_store()
    return [(e["error_text"], e["category"]) for e in store["errors"] if e["category"]]


def get_stats() -> dict:
    """Get knowledge base statistics."""
    store = load_store()
    errors = store["errors"]
    categories = {}
    for e in errors:
        cat = e["category"]
        categories[cat] = categories.get(cat, 0) + 1

    return {
        "total_errors": len(errors),
        "resolved": sum(1 for e in errors if e["resolved"]),
        "unresolved": sum(1 for e in errors if not e["resolved"]),
        "categories": categories,
        "total_fixes": len(store["fixes"]),
    }
