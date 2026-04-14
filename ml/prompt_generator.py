"""
OptiDock AI — Context Prompt Generator for LLM

Takes classified terminal errors and generates enriched context prompts
that help the connected LLM provide better, more targeted solutions.

Supports all LLM providers:
    - Gemini 2.0 Flash (default, free)
    - OpenAI (GPT-4, GPT-4o)
    - Anthropic (Claude 3.5, Claude Opus)
    - OpenRouter (multi-provider gateway)
    - Groq (fast inference, free tier)
    - Ollama (local LLMs: llama3, mistral, phi)
    - llama.cpp (local GGUF models)
    - LM Studio / vLLM (local OpenAI-compatible)
"""

import json
from typing import Optional

from . import error_store
from .error_learner import classify_error, CATEGORIES


# ── Prompt Templates by Error Category ────────────────────────────────

CATEGORY_CONTEXT = {
    "docker_build": {
        "system_context": (
            "You are OptiDock AI, a Docker optimization expert. The user encountered "
            "a Docker build error. Analyze the error, identify the root cause, and "
            "provide a corrected Dockerfile snippet. Focus on: layer caching, multi-stage "
            "builds, and build context issues. Always explain WHY the fix works."
        ),
        "fix_hints": [
            "Check if the file exists in the build context (.dockerignore may exclude it)",
            "Verify the FROM image tag is correct and available for the target platform",
            "Ensure COPY sources exist relative to the build context root",
            "Check for multi-stage build target name mismatches",
        ],
    },
    "docker_runtime": {
        "system_context": (
            "You are OptiDock AI, a container runtime expert. The user's container "
            "crashed or failed to start. Analyze the error and provide: 1) the likely "
            "root cause, 2) how to fix it, 3) how to prevent it in the Dockerfile. "
            "Consider OOM kills, exec format errors, and port conflicts."
        ),
        "fix_hints": [
            "OOMKilled (exit 137) → increase memory limits or optimize the app",
            "exec format error → wrong architecture, rebuild for the target platform",
            "address already in use → stop the conflicting container or change the port",
            "exec user process caused: no such file → check the ENTRYPOINT/CMD binary path",
        ],
    },
    "network": {
        "system_context": (
            "You are OptiDock AI, a networking and registry expert. The user hit a "
            "network error during build or runtime. This could be DNS, TLS, registry "
            "auth, or firewall issues. Provide step-by-step debugging and the fix."
        ),
        "fix_hints": [
            "DNS failures → check host network config or use --network=host during build",
            "Registry auth → run `docker login` or check credentials",
            "TLS timeout → corporate proxy or firewall may be blocking",
            "Connection refused → target service is down or wrong port",
        ],
    },
    "dependency": {
        "system_context": (
            "You are OptiDock AI, a dependency management expert. The user has a "
            "package resolution or version conflict error. Analyze the error and "
            "provide the correct dependency fix. Consider lockfiles, peer deps, "
            "and workspace configurations."
        ),
        "fix_hints": [
            "Use `npm ci` instead of `npm install` for deterministic builds",
            "Check Cargo.toml workspace member paths and version specifiers",
            "Pin exact versions in requirements.txt for reproducibility",
            "Ensure lockfile is committed and up to date",
        ],
    },
    "permission": {
        "system_context": (
            "You are OptiDock AI, a container security expert. The user hit a "
            "permission error. This often happens when running as non-root or "
            "accessing Docker socket. Provide the minimum-privilege fix."
        ),
        "fix_hints": [
            "Add the user to the 'docker' group: `usermod -aG docker $USER`",
            "Use `chown` in the Dockerfile before switching to non-root USER",
            "Mount volumes with correct UID/GID mapping",
            "Use --user flag with docker run to match file ownership",
        ],
    },
    "configuration": {
        "system_context": (
            "You are OptiDock AI, a configuration and environment expert. The user "
            "is missing an environment variable, config file, or API key. Help them "
            "set it up correctly and securely. Never suggest hardcoding secrets."
        ),
        "fix_hints": [
            "Use a .env file with dotenvy (Rust) or dotenv (Node/Python)",
            "Pass env vars at runtime: `docker run -e KEY=value`",
            "Use Docker secrets or build args for sensitive values",
            "Check for typos in variable names (case-sensitive)",
        ],
    },
    "resource": {
        "system_context": (
            "You are OptiDock AI, a system resource expert. The user is running "
            "out of disk space, memory, or CPU. Help them clean up and optimize."
        ),
        "fix_hints": [
            "Run `docker system prune` to reclaim disk space",
            "Use multi-stage builds to reduce image size",
            "Set memory limits: `docker run --memory 512m`",
            "Clean apt/npm cache in the same RUN layer",
        ],
    },
    "syntax": {
        "system_context": (
            "You are OptiDock AI, a code analysis expert. The user has a syntax "
            "or type error. Provide the corrected code with an explanation of "
            "what went wrong and how to prevent similar errors."
        ),
        "fix_hints": [
            "Check for missing semicolons, brackets, or parentheses",
            "Verify type annotations match the actual values",
            "Look for undefined variables or imports",
            "Run the language's linter for detailed diagnostics",
        ],
    },
    "unknown": {
        "system_context": (
            "You are OptiDock AI, an autonomous Docker optimization agent. The user "
            "encountered an error that hasn't been categorized yet. Analyze the error "
            "carefully, identify the most likely category, and provide a solution."
        ),
        "fix_hints": [
            "Read the full error message including any stack traces",
            "Check the exit code for hints (1=general, 137=OOM, 139=segfault)",
            "Search for the exact error message in issue trackers",
        ],
    },
}


# ── Prompt Generation ─────────────────────────────────────────────────

def generate_error_prompt(
    error_text: str,
    user_question: Optional[str] = None,
    include_history: bool = True,
) -> dict:
    """
    Generate an enriched context prompt for the LLM based on a terminal error.

    Returns:
        {
            "system_prompt": str,    → System message for the LLM
            "user_prompt": str,      → User message with full context
            "category": str,         → Detected error category
            "confidence": float,     → Classification confidence
            "similar_fixes": list,   → Past fixes for similar errors
        }
    """
    # Classify the error
    classification = classify_error(error_text)
    category = classification["category"]
    confidence = classification["confidence"]

    # Get category-specific context
    cat_ctx = CATEGORY_CONTEXT.get(category, CATEGORY_CONTEXT["unknown"])

    # Get similar past fixes
    similar = []
    if include_history:
        similar = error_store.get_similar_errors(category, limit=3)

    # Build system prompt
    system_prompt = cat_ctx["system_context"]

    # Build user prompt with full context
    parts = [
        f"## Terminal Error\n```\n{error_text}\n```\n",
        f"## Error Classification\n- Category: **{category}**\n- Confidence: {confidence:.1%}\n",
    ]

    if cat_ctx["fix_hints"]:
        parts.append("## Known Fix Patterns\n")
        for hint in cat_ctx["fix_hints"]:
            parts.append(f"- {hint}\n")

    if similar:
        parts.append("\n## Similar Past Errors (Resolved)\n")
        for s in similar:
            parts.append(f"- Error: `{s['error_text'][:100]}...`\n  Fix: {s['fix']}\n")

    if user_question:
        parts.append(f"\n## User Question\n{user_question}\n")

    parts.append(
        "\n## Required Response Format\n"
        "1. **Root Cause**: What caused this error\n"
        "2. **Fix**: The exact command or code change to resolve it\n"
        "3. **Prevention**: How to prevent this error in the future\n"
        "4. **Dockerfile Change** (if applicable): Show the corrected Dockerfile snippet"
    )

    user_prompt = "\n".join(parts)

    return {
        "system_prompt": system_prompt,
        "user_prompt": user_prompt,
        "category": category,
        "confidence": confidence,
        "similar_fixes": similar,
    }


def generate_optimization_prompt(
    dockerfile_content: str,
    analysis_findings: Optional[list] = None,
    security_findings: Optional[list] = None,
) -> dict:
    """
    Generate a prompt for the LLM to optimize a Dockerfile, enriched
    with analysis and security findings from the optiock engine.
    """
    system_prompt = (
        "You are OptiDock AI, an expert Dockerfile optimizer. Given a Dockerfile "
        "and analysis findings, produce an optimized version that:\n"
        "1. Uses multi-stage builds\n"
        "2. Minimizes image size (use slim/alpine variants)\n"
        "3. Maximizes layer cache efficiency\n"
        "4. Follows security best practices (non-root user, no secrets in layers)\n"
        "5. Includes HEALTHCHECK and metadata labels\n\n"
        "Return ONLY the optimized Dockerfile in a ```dockerfile code block."
    )

    parts = [
        f"## Current Dockerfile\n```dockerfile\n{dockerfile_content}\n```\n",
    ]

    if analysis_findings:
        parts.append("## Analysis Findings\n")
        for f in analysis_findings:
            parts.append(f"- [{f.get('severity', 'INFO')}] {f.get('title', '')}: {f.get('explanation', '')}\n")

    if security_findings:
        parts.append("\n## Security Findings\n")
        for f in security_findings:
            parts.append(f"- [{f.get('severity', 'INFO')}] [{f.get('category', '')}] {f.get('title', '')}\n")

    parts.append(
        "\n## Optimization Goals\n"
        "- Reduce image size by 50%+ if possible\n"
        "- Fix all CRITICAL and WARNING findings\n"
        "- Maintain application behavior\n"
        "- Ensure the image builds and runs correctly"
    )

    return {
        "system_prompt": system_prompt,
        "user_prompt": "\n".join(parts),
    }


# ── CLI Interface ─────────────────────────────────────────────────────

def format_prompt_for_display(prompt_data: dict) -> str:
    """Format a generated prompt for terminal display."""
    lines = [
        "-----------------------------------------------",
        "         OptiDock AI - LLM Context Prompt      ",
        "-----------------------------------------------",
        "",
        f"  Category:   {prompt_data['category']}",
        f"  Confidence: {prompt_data['confidence']:.1%}",
        "",
        "  ── System Prompt ──",
        f"  {prompt_data['system_prompt'][:200]}...",
        "",
        "  ── User Prompt (first 500 chars) ──",
        f"  {prompt_data['user_prompt'][:500]}...",
    ]

    if prompt_data.get("similar_fixes"):
        lines.append("")
        lines.append(f"  ── {len(prompt_data['similar_fixes'])} Similar Past Fixes Found ──")

    return "\n".join(lines)
