"""
OptiDock AI — ML Error Learner

TF-IDF + SGD Classifier that learns from terminal errors and categorizes them
into actionable error types. The model improves over time as more errors are
recorded through the optidock CLI.

Categories:
    docker_build      — Dockerfile syntax, build failures, layer issues
    docker_runtime    — Container crash, OOM, port conflicts
    network           — DNS, connectivity, registry auth failures
    dependency        — Missing packages, version conflicts, lockfile issues
    permission        — File access, Docker socket, privilege errors
    configuration     — Env vars, config files, missing secrets
    resource          — Disk space, memory limits, CPU throttling
    syntax            — Code syntax errors caught during build
    unknown           — Uncategorized errors (needs human review)
"""

import json
import os
from pathlib import Path
from typing import Optional

import joblib
from sklearn.feature_extraction.text import TfidfVectorizer
from sklearn.linear_model import SGDClassifier
from sklearn.pipeline import Pipeline

from . import error_store


MODEL_DIR = Path.home() / ".optidock" / "ml"
MODEL_PATH = MODEL_DIR / "error_classifier.joblib"
VECTORIZER_PATH = MODEL_DIR / "tfidf_vectorizer.joblib"

# ── Error Categories ──────────────────────────────────────────────────

CATEGORIES = [
    "docker_build",
    "docker_runtime",
    "network",
    "dependency",
    "permission",
    "configuration",
    "resource",
    "syntax",
    "unknown",
]

# ── Built-in Training Corpus ──────────────────────────────────────────
# Seed data so the model works out of the box, even before user errors

SEED_CORPUS = [
    # docker_build
    ("COPY failed: file not found in build context", "docker_build"),
    ("Step 5/12 : RUN npm install returned a non-zero code: 1", "docker_build"),
    ("failed to solve: dockerfile parse error", "docker_build"),
    ("Error response from daemon: dockerfile syntax error", "docker_build"),
    ("COPY --from=builder /app/target missing", "docker_build"),
    ("no matching manifest for linux/amd64", "docker_build"),
    ("failed to compute cache key: not found", "docker_build"),
    ("ARG variable not set during build", "docker_build"),
    ("multi-stage build target not found", "docker_build"),
    ("RUN cargo build returned exit code 101", "docker_build"),
    ("error building image: command '/bin/sh -c apt-get install' returned non-zero", "docker_build"),
    ("failed to create LLB definition: circular dependency", "docker_build"),

    # docker_runtime
    ("container exited with code 137 OOMKilled", "docker_runtime"),
    ("Error starting userland proxy: listen tcp 0.0.0.0:8080: bind: address already in use", "docker_runtime"),
    ("container is unhealthy", "docker_runtime"),
    ("exec /app/optidock: exec format error", "docker_runtime"),
    ("Exited (1) 3 seconds ago", "docker_runtime"),
    ("OCI runtime exec failed: exec failed: unable to start container process", "docker_runtime"),
    ("docker: Error response from daemon: Conflict container name already in use", "docker_runtime"),
    ("standard_init_linux.go:228: exec user process caused: no such file", "docker_runtime"),

    # network
    ("Get https://registry-1.docker.io/v2/: dial tcp: lookup registry-1.docker.io: no such host", "network"),
    ("error pulling image: unauthorized: authentication required", "network"),
    ("Could not resolve host: github.com", "network"),
    ("TLS handshake timeout", "network"),
    ("connection refused 127.0.0.1:5432", "network"),
    ("Error response from daemon: Get https://gcr.io/v2/: net/http: request canceled", "network"),
    ("npm ERR! network request to https://registry.npmjs.org failed", "network"),
    ("failed to fetch https://dl-cdn.alpinelinux.org: connection timed out", "network"),

    # dependency
    ("npm ERR! peer dep missing: react@^18.0.0, required by", "dependency"),
    ("error: package `serde v2.0.0` cannot be found", "dependency"),
    ("ModuleNotFoundError: No module named 'torch'", "dependency"),
    ("Could not find a version that satisfies the requirement", "dependency"),
    ("npm WARN deprecated package: use xyz instead", "dependency"),
    ("error[E0433]: failed to resolve: use of undeclared crate or module", "dependency"),
    ("yarn.lock is out of date", "dependency"),
    ("Cargo.lock needs to be updated but --locked was passed", "dependency"),

    # permission
    ("PermissionError: [Errno 13] Permission denied: '/app/data'", "permission"),
    ("Got permission denied while trying to connect to the Docker daemon socket", "permission"),
    ("error: EACCES: permission denied, mkdir '/usr/local/lib'", "permission"),
    ("cannot open '/etc/ssl/certs/ca-certificates.crt': Permission denied", "permission"),
    ("chown: changing ownership of '/app': Operation not permitted", "permission"),

    # configuration
    ("Error: NEXT_PUBLIC_SUPABASE_URL is not defined", "configuration"),
    ("Missing required environment variable: DATABASE_URL", "configuration"),
    ("OPENAI_API_KEY not set", "configuration"),
    ("invalid configuration: port must be a number", "configuration"),
    (".env file not found", "configuration"),
    ("GEMINI_API_KEY environment variable is not set", "configuration"),
    ("Error: Cannot find module './config/settings'", "configuration"),

    # resource
    ("no space left on device", "resource"),
    ("Cannot allocate memory", "resource"),
    ("docker: Error response from daemon: not enough memory", "resource"),
    ("OOMKilled: true", "resource"),
    ("disk quota exceeded", "resource"),
    ("Error processing tar file: no space left on device", "resource"),

    # syntax
    ("SyntaxError: Unexpected token }", "syntax"),
    ("error[E0308]: mismatched types", "syntax"),
    ("IndentationError: unexpected indent", "syntax"),
    ("error: expected `;`, found `}`", "syntax"),
    ("TypeError: Cannot read properties of undefined", "syntax"),
    ("ReferenceError: variable is not defined", "syntax"),
]


# ── Model Training ────────────────────────────────────────────────────

def train_model(additional_data: Optional[list] = None) -> dict:
    """
    Train the error classifier with seed data + any user-recorded errors.
    Returns training stats.
    """
    MODEL_DIR.mkdir(parents=True, exist_ok=True)

    # Combine seed corpus + user data + any additional data
    corpus = list(SEED_CORPUS)

    user_data = error_store.get_all_training_data()
    corpus.extend(user_data)

    if additional_data:
        corpus.extend(additional_data)

    texts = [t for t, _ in corpus]
    labels = [l for _, l in corpus]

    pipeline = Pipeline([
        ("tfidf", TfidfVectorizer(
            max_features=5000,
            ngram_range=(1, 3),
            analyzer="word",
            sublinear_tf=True,
        )),
        ("clf", SGDClassifier(
            loss="modified_huber",
            max_iter=1000,
            class_weight="balanced",
            random_state=42,
        )),
    ])

    pipeline.fit(texts, labels)
    joblib.dump(pipeline, MODEL_PATH)

    return {
        "total_samples": len(corpus),
        "seed_samples": len(SEED_CORPUS),
        "user_samples": len(user_data),
        "categories": list(set(labels)),
        "model_path": str(MODEL_PATH),
    }


def load_model() -> Optional[Pipeline]:
    """Load the trained model, or train one if it doesn't exist."""
    if not MODEL_PATH.exists():
        train_model()
    try:
        return joblib.load(MODEL_PATH)
    except Exception:
        train_model()
        return joblib.load(MODEL_PATH)


def classify_error(error_text: str) -> dict:
    """
    Classify a terminal error and return category + confidence.
    Also records the error in the knowledge base for future learning.
    """
    model = load_model()
    if model is None:
        return {"category": "unknown", "confidence": 0.0}

    category = model.predict([error_text])[0]
    probabilities = model.predict_proba([error_text])[0]
    confidence = max(probabilities)

    # Record in knowledge base
    error_store.record_error(error_text, category, source="terminal")

    return {
        "category": category,
        "confidence": round(float(confidence), 3),
        "all_scores": {
            cat: round(float(prob), 3)
            for cat, prob in zip(model.classes_, probabilities)
        },
    }


def retrain_with_correction(error_text: str, correct_category: str):
    """Retrain the model after a user corrects a classification."""
    error_store.record_error(error_text, correct_category, source="correction")
    return train_model()
