"""
OptiDock AI — ML Error Learning Engine

This module learns from terminal errors and Docker build failures to generate
enriched context prompts for the connected LLM provider. Over time, it builds
a knowledge base of error patterns, root causes, and successful fixes.

Architecture:
    error_learner.py   → TF-IDF + classifier for error categorization
    prompt_generator.py → Generates context-enriched prompts for the LLM
    error_store.py     → JSON-based persistent error knowledge base
    train.py           → Training pipeline with built-in error corpus
"""
