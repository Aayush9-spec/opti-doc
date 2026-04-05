# OptiDock AI Project Context

This document is the source of truth for product direction, architecture, scope, and implementation priorities.

## 1. Project Identity

### Name

OptiDock AI

### One-Line Pitch

A Rust-first autonomous Docker optimization and deployment agent with a powerful terminal experience.

### Product Positioning

OptiDock AI should feel like a real systems product:

- fast enough to trust from the terminal
- smart enough to optimize container workflows
- opinionated enough to make useful decisions
- safe enough to validate and roll back changes

### Core USP

AI-native container operations with benchmark-backed optimization, delivered through a serious CLI and agent runtime.

## 2. Strategic Direction

The project direction is now explicitly:

- Rust-first
- terminal-first
- systems-oriented
- benchmark-driven
- agentic, but grounded in deterministic validation

The product should not feel like a thin wrapper around prompts. It should feel like a new kind of developer infrastructure tool.

## 3. Big Vision

OptiDock AI should eventually change how developers work with Docker and container agents by owning the optimization lifecycle end to end:

1. understand repository and container context
2. detect inefficiencies and risks
3. propose or generate improved Dockerfiles
4. benchmark candidate builds
5. deploy the best variant safely
6. observe outcomes
7. re-optimize when conditions change

## 4. Product Principles

- Terminal is a first-class product surface
- AI should amplify deterministic systems logic, not replace it
- Every generated change should be explainable
- Validation is mandatory before promotion
- Rollback should be built in, not bolted on
- Real performance wins matter more than flashy breadth

## 5. Primary User Experience

The CLI is the main entry point.

Key commands should eventually include:

- `optidock init`
- `optidock analyze <path>`
- `optidock optimize <path>`
- `optidock benchmark <path>`
- `optidock deploy <path>`
- `optidock monitor`
- `optidock rollback`

The UX goal is:

- compact output for normal use
- rich explanations when requested
- visible diffs and benchmark comparisons
- confidence-oriented messaging

## 6. MVP Scope

The MVP should prove one high-value loop:

1. read a project with a Dockerfile
2. analyze its container setup
3. produce findings from deterministic rules
4. generate an optimized Dockerfile candidate
5. build baseline and optimized versions
6. compare image size and build results
7. present a polished terminal report

Optional MVP+:

- deploy optimized candidate locally
- expose rollback command

## 7. Non-Goals For Early Versions

Do not over-expand into:

- full Kubernetes orchestration
- multi-cluster scheduling
- service mesh behavior
- cloud abstraction layers
- enterprise platform features
- broad web dashboard dependency for core workflows

The terminal product must stand on its own.

## 8. Why Rust Is The Right Fit

Rust supports the project goals well:

- low-latency CLI and live terminal rendering
- memory safety in orchestration code
- structured concurrency for parallel builds and monitoring
- strong typing for analysis, reports, and decisions
- a clean path to a daemon or background agent later

## 9. Product Surfaces

### A. CLI

Responsibilities:

- command entry point
- user-friendly terminal output
- flags, config, and workflow control

### B. Core Domain

Responsibilities:

- shared types
- finding models
- optimization report models
- decision engine data structures

### C. Analyzer

Responsibilities:

- Dockerfile parsing
- repository context inspection
- deterministic rule checks

### D. Optimizer

Responsibilities:

- prepare LLM context
- request candidate Dockerfile rewrites
- parse structured AI output
- remain provider-agnostic across hosted and local model backends

### E. Runner

Responsibilities:

- build images
- capture size and build time
- run startup checks and simple benchmarks

### F. Agent Runtime

Responsibilities:

- orchestrate multi-step flows
- own autonomous optimize-and-validate loops
- manage deployment decisions

### G. Optional UI Later

A dashboard can exist later, but it is not the primary interface for the early product.

## 10. Target Rust Workspace

```text
Cargo.toml
crates/
  optidock-cli/
  optidock-core/
  optidock-analyzer/
  optidock-runner/
  optidock-agent/
docs/
  architecture/
  prompts/
examples/
```

## 11. Suggested Crate Responsibilities

### `optidock-cli`

- parse commands
- render human-readable output
- call orchestration flows

### `optidock-core`

- canonical domain models
- config structures
- error types

### `optidock-analyzer`

- Dockerfile parser
- rule registry
- findings engine

### `optidock-runner`

- Docker command execution
- metrics collection
- process supervision

### `optidock-agent`

- optimization plans
- AI orchestration
- decision making

## 12. Recommended Rust Ecosystem

- `clap` for CLI parsing
- `tokio` for async runtime
- `serde` and `serde_json` for reports
- `anyhow` or `thiserror` for error handling
- `tracing` and `tracing-subscriber` for logs
- `ratatui` later if a richer TUI becomes useful
- provider adapters for OpenAI-compatible and vendor-specific APIs

## 13. AI Provider Strategy

The AI layer must be portable.

OptiDock should support:

- OpenAI
- Anthropic / Claude
- Gemini
- OpenRouter
- local OpenAI-compatible endpoints
- Ollama or other local model runners

Design rule:

- the rest of the system should talk to a provider-agnostic optimization interface
- provider-specific request translation should live behind adapters
- switching models should not require rewriting orchestration logic
- local models should remain first-class for privacy and offline workflows

## 14. Domain Model Direction

The project should converge on structured types such as:

- `ProjectContext`
- `DockerfileAnalysis`
- `Finding`
- `OptimizationProposal`
- `BuildResult`
- `BenchmarkResult`
- `DeploymentDecision`
- `RunSummary`

These should live in shared core modules so every layer uses the same language.

## 15. Analyzer Design

The analyzer should be valuable even with no LLM connected.

Initial rule ideas:

- missing `WORKDIR`
- early `COPY . .`
- package-manager anti-patterns
- large base image usage
- missing multi-stage opportunity
- cache-unfriendly layer ordering
- root user risk
- package cache not cleaned

Each finding should include:

- id
- severity
- title
- explanation
- suggested fix

## 15. AI Optimization Strategy

The AI optimizer should only run after deterministic analysis has prepared context.

The optimizer must return:

- candidate Dockerfile
- summary of changes
- assumptions
- expected impact
- risks

The system should prefer structured output plus a separate Dockerfile body.

## 16. Runner And Benchmark Strategy

The runner should compare baseline and optimized variants.

Initial metrics:

- build success
- image size
- build duration
- startup success
- startup latency

Later metrics:

- CPU under simple load
- peak memory
- request latency for HTTP apps

## 17. Decision Engine

Promotion logic should stay conservative.

A candidate should only be approved when:

- build succeeds
- smoke test succeeds
- startup behavior is acceptable
- no severe regression is detected
- image size or build efficiency improves enough to matter

## 18. Deployment Direction

Early deployment should be intentionally simple:

- local Docker deployment
- named environment support
- exposed port mapping
- last-known-good rollback

Do not build a fake orchestrator too early. Build a reliable deployment loop first.

## 19. Roadmap

### Phase 1: Real Foundation

- create Rust workspace
- implement CLI shell
- define domain models
- add config and logging

### Phase 2: Analyzer

- parse Dockerfiles
- implement rule checks
- print findings report

### Phase 3: Optimizer

- design AI prompt contract
- generate candidate Dockerfiles
- persist proposal reports

### Phase 4: Validation

- build baseline and optimized images
- capture metrics
- compare outcomes

### Phase 5: Deployment

- deploy approved image
- add rollback flow
- track run history

### Phase 6: Agentic Loop

- schedule or trigger re-optimization
- monitor changes
- re-run decisions based on signals

## 20. Immediate Next Build Targets

These are the most important next implementation tasks:

1. scaffold Cargo workspace
2. create `optidock` CLI entrypoint
3. define core domain structs
4. implement initial analyzer command
5. print structured findings in terminal output

## 21. Demo Story

The first convincing demo is terminal-driven:

1. run `optidock analyze`
2. show findings on a weak Dockerfile
3. run `optidock optimize`
4. show rewritten Dockerfile and rationale
5. run validation
6. show measurable improvement

The story should feel like a developer using a real tool, not watching a concept slide.

## 22. Success Criteria

- the CLI feels polished and intentional
- analyzer results are useful before AI integration
- optimized candidates are benchmarked, not blindly trusted
- the project architecture looks credible for long-term growth
- the product narrative is distinct from generic DevOps tooling

## 23. Guardrails

- prefer depth over breadth
- prefer terminal excellence over premature UI work
- prefer reliable analysis over speculative automation
- prefer measurable wins over architecture theater

## 24. Working Rule

When choosing what to build next, ask:

`Does this make OptiDock feel more like a real Rust container operations product?`

If not, it is probably not the highest-priority task right now.
