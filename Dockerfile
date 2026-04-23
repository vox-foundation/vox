# Multi-stage build for minimal production image (~50MB)
# Cross-platform lanes, feature matrix, and env toggles: docs/src/architecture/vox-cross-platform-runbook.md
# Usage:
#   docker build -t vox .
#   docker build -t vox:mesh --build-arg VOX_CLI_FEATURES=populi,script-execution .
#   docker run -e GEMINI_API_KEY=... -p 3000:3000 vox
#
# Optional mesh HTTP control plane + MCP in one container:
#   docker run -e VOX_MESH_MESH_SIDECAR=1 -p 3000:3000 -p 9847:9847 vox:mesh vox mcp

FROM rust:1.92-slim-bookworm AS builder
RUN apt-get update && apt-get install -y pkg-config libssl-dev gcc && rm -rf /var/lib/apt/lists/*
WORKDIR /app

ARG VOX_CLI_FEATURES=
# Cache dependency layer
COPY Cargo.toml Cargo.lock ./
COPY crates/ crates/
COPY patches/ patches/
# Reproducibility: `cargo --locked` pins the Rust resolve graph. Vox PM uses `vox.lock` in projects
# that declare `Vox.toml` dependencies; this image build only compiles the CLI from the workspace
# `Cargo.lock` (no `vox sync` in this Dockerfile).
# `--locked` keeps the Rust dependency graph reproducible (same policy as `vox ci manifest`).
RUN if [ -z "$VOX_CLI_FEATURES" ]; then \
      cargo build --release -j 1 --locked -p vox-cli && strip /app/target/release/vox; \
    else \
      cargo build --release -j 1 --locked -p vox-cli --features "$VOX_CLI_FEATURES" && strip /app/target/release/vox; \
    fi

# Runtime image — no Rust toolchain, just the binary + TLS certs
FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/vox /usr/local/bin/vox
# Tiny script for mesh compose worker (`vox run --mode script`); see examples/mesh-compose.yml.
COPY examples/golden/mesh/noop.vox /opt/vox/mesh-noop.vox
COPY infra/containers/entrypoints/vox-entrypoint.vox /usr/local/bin/vox-entrypoint.vox

# VoxDB data volume mount point
VOLUME /root/.vox
EXPOSE 3000
EXPOSE 9847

# Health check via vox doctor --probe (stable exit code; no string-grep on human output)
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s \
    CMD vox doctor --probe

ENTRYPOINT ["vox", "run", "--interp", "/usr/local/bin/vox-entrypoint.vox"]
CMD ["vox", "mcp"]
