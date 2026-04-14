# ── Stage 1: Build ────────────────────────────────────────────────────
FROM rust:1.82-bookworm AS builder

WORKDIR /build

# Copy manifests first for layer caching
COPY Cargo.toml Cargo.lock rust-toolchain.toml ./
COPY crates/ crates/

# Build release binary
RUN cargo build --release --bin optidock

# ── Stage 2: Runtime ──────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

RUN useradd --create-home --shell /bin/bash optidock

WORKDIR /app

COPY --from=builder /build/target/release/optidock /app/optidock

# Bundle a sample Dockerfile for the /analyze endpoint
COPY crates/optidock-cli/src/main.rs /app/sample/main.rs
RUN printf 'FROM node:20-alpine\nWORKDIR /app\nCOPY . .\nRUN npm ci\nEXPOSE 3000\nCMD ["npm","start"]\n' > /app/sample/Dockerfile

USER optidock

ENV PORT=8080
EXPOSE 8080

CMD ["/app/optidock", "serve", "--port", "8080"]
