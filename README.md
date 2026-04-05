# OptiDock AI

OptiDock AI is a Rust-first autonomous Docker optimization and container operations agent. It is designed to feel fast, reliable, and production-minded from the terminal while still using AI where it creates leverage: Dockerfile rewriting, optimization reasoning, deployment decisions, and container intelligence.

## Vision

Build a serious container agent platform that helps developers and small teams:

- analyze Dockerfiles and container projects
- generate safer and smaller image builds
- benchmark baseline versus optimized variants
- deploy winning candidates with rollback support
- manage container workflows from a polished terminal experience

This project should feel closer to a real systems tool than a lightweight demo.

## Why Rust

Rust is the right foundation for the direction you want:

- fast and reliable terminal UX
- strong concurrency for builds, logs, and live metrics
- safer systems programming for Docker and process orchestration
- easier path to a robust CLI, daemon, and agent runtime

## Product Direction

OptiDock AI is not just another CI helper and not just a mini Kubernetes clone.

The core idea is:

`AI-native container operations for developers who want autonomous optimization without infrastructure bloat.`

That means the project should eventually own the full optimization loop:

1. inspect repository and container context
2. detect bad Docker patterns
3. generate improved Dockerfiles
4. build and benchmark alternatives
5. choose the best candidate
6. deploy, observe, and roll back if needed
7. keep learning from container outcomes

## Core Experience

The first-class interface should be the terminal.

Example commands:

```bash
optidock init
optidock analyze ./my-app
optidock optimize ./my-app
optidock benchmark ./my-app
optidock deploy ./my-app
optidock monitor
```

The CLI should feel:

- fast
- trustworthy
- clear about what changed
- useful even before AI is enabled

## High-Level Architecture

```text
User Repo
  -> Rust CLI
  -> Context Scanner
  -> Rule Engine
  -> AI Optimizer
  -> Build Runner
  -> Benchmark Runner
  -> Decision Engine
  -> Deployment Agent
  -> Monitoring + Reports
```

## Rust Workspace Shape

```text
crates/
  optidock-cli/        # terminal interface
  optidock-core/       # shared domain types and business logic
  optidock-analyzer/   # Dockerfile and repo analysis
  optidock-runner/     # build, benchmark, command execution
  optidock-agent/      # orchestration and autonomous flows
```

## MVP

The MVP should prove one strong end-to-end story:

1. analyze a Docker project
2. report optimization issues
3. generate an improved Dockerfile
4. compare image size and build behavior
5. show a clean terminal report

If time allows, add deploy and rollback after validation.

## What Makes It Feel Real

- Rust workspace with clear module boundaries
- terminal-first UX instead of a slide-demo-only UI
- deterministic analyzer before any LLM call
- benchmark-backed decisions instead of blind generation
- reports, histories, and rollback logic

## Immediate Build Order

1. Rust workspace and CLI shell
2. Dockerfile analyzer engine
3. optimization report format
4. AI rewrite flow
5. build and benchmark runner
6. deploy and monitor loop

## Source Of Truth

Use [PROJECT_CONTEXT.md](PROJECT_CONTEXT.md) as the canonical product and architecture reference while building. It contains the detailed roadmap, module responsibilities, and decision rules for the project.
