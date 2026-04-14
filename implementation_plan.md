# Connect OptiDock AI to Google Cloud Console

Deploy the OptiDock AI Rust CLI and Next.js landing page to Google Cloud Platform using Cloud Run, Artifact Registry, and GitHub Actions CI/CD — ready for your April 19 submission.

## What This Achieves

Your OptiDock AI project will be:
1. **Containerized** with production-grade multi-stage Dockerfiles (Rust CLI + Next.js landing)
2. **Deployed to Google Cloud Run** — containers accessible via public URLs
3. **CI/CD automated** via GitHub Actions — push to `main` → auto-deploy to GCP
4. **Integrated with GCP services** — Artifact Registry for images, Cloud Build, Cloud Run for hosting
5. **GCP Console visible** — services, logs, and metrics all visible in Google Cloud Console

---

## User Review Required

> [!IMPORTANT]
> **You need a Google Cloud account with billing enabled.** Cloud Run has a generous free tier (2 million requests/month), so costs should be minimal or zero for a project submission.

> [!WARNING]
> **You'll need to run some `gcloud` CLI commands manually** to set up initial authentication and create the GCP project. I'll provide exact commands and guide you through it.

> [!IMPORTANT]
> **GCP Project ID required.** Before I start executing, I need you to either:
> - A) Share your existing GCP project ID, **OR**
> - B) Confirm you want me to guide you through creating a new GCP project

---

## Proposed Changes

### Component 1: Production Dockerfiles

#### [NEW] [Dockerfile](file:///c:/Users/snghs/Documents/GitHub/opti-doc/Dockerfile)
Multi-stage Dockerfile for the Rust CLI binary:
- **Stage 1 (builder):** Uses `rust:1.82-bookworm`, copies workspace, builds `--release`
- **Stage 2 (runtime):** Uses `debian:bookworm-slim`, copies only the compiled binary
- Exposes port `8080` (Cloud Run requirement), sets `PORT` env var
- Since OptiDock is a CLI tool (not an HTTP server), we'll add a lightweight health-check HTTP endpoint to satisfy Cloud Run's requirements

#### [NEW] [landing/Dockerfile](file:///c:/Users/snghs/Documents/GitHub/opti-doc/landing/Dockerfile)
Multi-stage Dockerfile for the Next.js landing page:
- **Stage 1 (deps):** Installs npm dependencies
- **Stage 2 (builder):** Builds the Next.js production bundle
- **Stage 3 (runner):** Uses `node:20-alpine`, runs standalone Next.js output
- Exposes port `3000`, respects `PORT` env var

---

### Component 2: Cloud Run Configuration

#### [NEW] [cloud-run/cli-service.yaml](file:///c:/Users/snghs/Documents/GitHub/opti-doc/cloud-run/cli-service.yaml)
Cloud Run service definition for the Rust CLI container:
- 512MB memory, 1 vCPU
- Min 0 / max 3 instances (cost-effective for demo)
- Environment variables for provider config

#### [NEW] [cloud-run/landing-service.yaml](file:///c:/Users/snghs/Documents/GitHub/opti-doc/cloud-run/landing-service.yaml)
Cloud Run service definition for the Next.js landing:
- 256MB memory, 1 vCPU
- Min 0 / max 5 instances
- Supabase env vars injection

---

### Component 3: GitHub Actions CI/CD Pipeline

#### [NEW] [.github/workflows/deploy-gcp.yml](file:///c:/Users/snghs/Documents/GitHub/opti-doc/.github/workflows/deploy-gcp.yml)
Complete CI/CD pipeline that triggers on `push to main`:
1. **Authenticate** to GCP using Workload Identity Federation (or service account key)
2. **Build** both Docker images (CLI + landing)
3. **Push** to Google Artifact Registry
4. **Deploy** both services to Cloud Run
5. **Output** the live URLs

---

### Component 4: GCP Setup Script

#### [NEW] [scripts/gcp-setup.sh](file:///c:/Users/snghs/Documents/GitHub/opti-doc/scripts/gcp-setup.sh)
One-time setup script you'll run locally to:
- Enable required GCP APIs (Cloud Run, Artifact Registry, Cloud Build, IAM)
- Create Artifact Registry Docker repository
- Create a service account for GitHub Actions
- Generate and display the service account key (to add as GitHub Secret)
- Print exact GitHub Secrets you need to configure

#### [NEW] [scripts/gcp-setup.ps1](file:///c:/Users/snghs/Documents/GitHub/opti-doc/scripts/gcp-setup.ps1)
PowerShell equivalent for Windows users (you!)

---

### Component 5: Rust CLI Health Endpoint

#### [MODIFY] [main.rs](file:///c:/Users/snghs/Documents/GitHub/opti-doc/crates/optidock-cli/src/main.rs)
Add a lightweight HTTP health server mode:
- New `serve` subcommand: `optidock serve --port 8080`
- Exposes `GET /` → JSON status report (version, uptime, provider config)
- Exposes `GET /health` → `200 OK`
- Exposes `GET /analyze` → runs analysis on a bundled sample Dockerfile
- This lets Cloud Run keep the container alive and lets you demo the tool via HTTP

#### [MODIFY] [Cargo.toml](file:///c:/Users/snghs/Documents/GitHub/opti-doc/crates/optidock-cli/Cargo.toml)
Add `axum` and `tower-http` for the lightweight HTTP server

---

### Component 6: Landing Page GCP Integration

#### [MODIFY] [landing/next.config.mjs](file:///c:/Users/snghs/Documents/GitHub/opti-doc/landing/next.config.mjs)
Enable standalone output mode for Docker-optimized Next.js builds

---

### Component 7: Documentation

#### [NEW] [docs/gcp-deployment.md](file:///c:/Users/snghs/Documents/GitHub/opti-doc/docs/gcp-deployment.md)
Complete GCP deployment guide documenting:
- Prerequisites (GCP account, gcloud CLI, Docker)
- Step-by-step setup instructions
- How CI/CD works
- How to access services in GCP Console
- Troubleshooting common issues

#### [MODIFY] [README.md](file:///c:/Users/snghs/Documents/GitHub/opti-doc/README.md)
Add GCP deployment section with badges and quick-start commands

---

## Architecture Diagram

```
┌─────────────┐     push to main     ┌──────────────────┐
│   GitHub     │ ──────────────────▶  │  GitHub Actions   │
│   Repo       │                      │  CI/CD Pipeline   │
└─────────────┘                      └────────┬─────────┘
                                              │
                                    ┌─────────▼──────────┐
                                    │  Google Artifact    │
                                    │  Registry           │
                                    │  (Docker images)    │
                                    └─────────┬──────────┘
                                              │
                              ┌───────────────┼───────────────┐
                              │               │               │
                    ┌─────────▼─────┐ ┌───────▼───────┐      │
                    │  Cloud Run    │ │  Cloud Run    │      │
                    │  optidock-cli │ │  landing-page │      │
                    │  (Rust API)   │ │  (Next.js)    │      │
                    └───────────────┘ └───────────────┘      │
                                                             │
                    ┌────────────────────────────────────────┘
                    │  All visible in Google Cloud Console:
                    │  • Services, logs, metrics, revisions
                    │  • Container images in Artifact Registry
                    │  • Build history
                    └────────────────────────────────────────
```

---

## Open Questions

> [!IMPORTANT]
> **1. Do you have a Google Cloud account?** If not, you can create one at [console.cloud.google.com](https://console.cloud.google.com) — new accounts get $300 free credits.

> [!IMPORTANT]
> **2. Do you have `gcloud` CLI installed?** I'll need you to run a few setup commands. If not, I'll add installation steps.

> [!IMPORTANT]
> **3. Authentication method preference:** 
> - **Option A: Service Account Key** (simpler, fine for a school project) — I'll generate a JSON key and you'll add it as a GitHub Secret
> - **Option B: Workload Identity Federation** (more secure, recommended by Google) — requires more GCP setup but no keys to manage
> 
> **I recommend Option A** for your submission deadline.

> [!IMPORTANT]
> **4. GCP Region preference?** I'll default to `us-central1` (cheapest and most available). Let me know if you need a different region.

---

## Verification Plan

### Automated Tests
- Run `cargo build --release` to verify Rust compilation
- Run `cargo test` to ensure all existing tests pass
- Build Docker images locally with `docker build` 
- Verify Cloud Run services respond with `curl`

### Manual Verification
- Confirm both services are visible in Google Cloud Console
- Verify CI/CD pipeline runs successfully on `git push`
- Test the live URLs returned by Cloud Run
- Check Artifact Registry shows the pushed images
