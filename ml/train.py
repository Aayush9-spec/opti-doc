#!/usr/bin/env python3
"""
OptiDock AI — ML Training Pipeline

Train the error classifier and prompt generator. Run this after installing
dependencies to bootstrap the model, or re-run to retrain with new data.

Usage:
    python -m ml.train              # Train and show stats
    python -m ml.train --classify "error message"   # Classify an error
    python -m ml.train --prompt "error message"     # Generate LLM prompt
    python -m ml.train --stats      # Show knowledge base stats
"""

import argparse
import json
import sys
from pathlib import Path

# Add parent directory to path
sys.path.insert(0, str(Path(__file__).parent.parent))

from ml.error_learner import train_model, classify_error, retrain_with_correction, CATEGORIES
from ml.prompt_generator import generate_error_prompt, format_prompt_for_display
from ml.error_store import get_stats


def main():
    parser = argparse.ArgumentParser(
        description="OptiDock AI — ML Error Learning Pipeline"
    )
    parser.add_argument(
        "--classify", type=str, metavar="ERROR",
        help="Classify a terminal error message",
    )
    parser.add_argument(
        "--prompt", type=str, metavar="ERROR",
        help="Generate an LLM context prompt for an error",
    )
    parser.add_argument(
        "--stats", action="store_true",
        help="Show knowledge base statistics",
    )
    parser.add_argument(
        "--retrain", action="store_true",
        help="Force retrain the model with latest data",
    )

    args = parser.parse_args()

    if args.classify:
        result = classify_error(args.classify)
        print("\n-----------------------------------------------")
        print("       OptiDock AI - Error Classification")
        print("-----------------------------------------------")
        print(f"\n  Category:   {result['category']}")
        print(f"  Confidence: {result['confidence']:.1%}")
        print("\n  All Scores:")
        for cat, score in sorted(result["all_scores"].items(), key=lambda x: -x[1]):
            bar = "█" * int(score * 30)
            print(f"    {cat:20s} {score:.1%} {bar}")
        return

    if args.prompt:
        prompt_data = generate_error_prompt(args.prompt)
        print(format_prompt_for_display(prompt_data))
        print("\n  ── Full Prompt (JSON) ──")
        print(json.dumps({
            "system_prompt": prompt_data["system_prompt"],
            "user_prompt": prompt_data["user_prompt"][:1000] + "...",
            "category": prompt_data["category"],
            "confidence": prompt_data["confidence"],
        }, indent=2))
        return

    if args.stats:
        stats = get_stats()
        print("\n-----------------------------------------------")
        print("       OptiDock AI - Knowledge Base Stats")
        print("-----------------------------------------------")
        print(f"\n  Total errors:    {stats['total_errors']}")
        print(f"  Resolved:        {stats['resolved']}")
        print(f"  Unresolved:      {stats['unresolved']}")
        print(f"  Total fixes:     {stats['total_fixes']}")
        print("\n  Categories:")
        for cat, count in sorted(stats["categories"].items(), key=lambda x: -x[1]):
            print(f"    {cat:20s} {count}")
        return

    print("\n-----------------------------------------------")
    print("      OptiDock AI - Training ML Model")
    print("-----------------------------------------------\n")

    result = train_model()

    print(f"  + Total training samples: {result['total_samples']}")
    print(f"  + Seed corpus samples:    {result['seed_samples']}")
    print(f"  + User-recorded samples:  {result['user_samples']}")
    print(f"  + Categories: {len(result['categories'])}")
    print(f"  + Model saved to: {result['model_path']}")

    print("\n  Supported Categories:")
    for cat in sorted(result["categories"]):
        print(f"    - {cat}")

    # Quick test
    print("\n  -- Quick Self-Test --")
    test_cases = [
        "COPY failed: file not found in build context",
        "container exited with code 137 OOMKilled",
        "npm ERR! peer dep missing: react@^18",
        "PermissionError: Permission denied: '/app'",
        "OPENAI_API_KEY not set",
    ]
    for test in test_cases:
        result = classify_error(test)
        print(f"    {result['category']:20s} ({result['confidence']:.0%}) <- {test[:50]}")

    print("\n  + Training complete. Model is ready.\n")


if __name__ == "__main__":
    main()
