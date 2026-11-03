---
last_mapped_commit: 9a79f708de1ce8f8b3944dc99afb51a2877fb960
last_mapped_at: 2026-09-22
---
# Technology Stack

**Analysis Date:** 2026-09-22

## Languages

**Primary:**

- Rust (edition 2024, `rust-version = "1.96"` in `Cargo.toml` `[workspace.package]`, pinned toolchain `1.98.1` via `rust-toolchain.toml`) - 138 crates under `crates/*`, the entire compiler, CLI, orchestrator, and runtime
- Vox (`.vox` files) - the language this repo implements; also used as the project's own glue-script language (see `examples/golden/`, `scripts/*.vox`) per the VoxScript-First Glue Code policy in `AGENTS.md`
- TypeScript/React - `crates/vox-gui/ui/` (Tauri 2 desktop GUI) and `docs-astro/` (Starlight documentation site)

**Secondary:**

- Markdown + YAML frontmatter - `docs/src/**/*.md` (documentation corpus, frontmatter-governed, see `docs/src/contributors/documentation-governance.md`)
- TOML - configuration and contracts (`contracts/**/*.toml`, `docs/src/architecture/layers.toml`)

## Runtime

**Environment:**

- Rust toolchain `1.98.1` (`rust-toolchain.toml`), components `rustfmt`, `clippy`, target `wasm32-wasip1` also installed
- Node.js via Homebrew + `pnpm` for the two TS packages (`docs-astro/`, `crates/vox-gui/ui/`) - no root `package.json`; no `.nvmrc`/`packageManager` pin found in-repo (see `~/dev/CLAUDE.md` house rule: Node via Homebrew, `pnpm` for JS)

**Package Manager:**

- Cargo (workspace resolver `"2"`), root `Cargo.toml` - Lockfile: `Cargo.lock` present (committed, diffable per `.gitattributes`)
- pnpm for `docs-astro/` and `crates/vox-gui/ui/` (pnpm `overrides` block present in `crates/vox-gui/ui/package.json`) - Lockfiles not directly inspected but expected per-package

**Build coordination:**

- All `cargo` invocations MUST route through the machine-wide build broker (a `cargo`-named shim ahead of the rustup proxy) — see `~/dev/vox/AGENTS.md` §Build Broker; installed via `scripts/broker-install.vox`

## Frameworks

**Core:**

- `tokio` `1` (rt, macros, sync, time, net, process, io-util, fs, multi-thread, signal) - async runtime for CLI, orchestrator, mesh
- `clap` `4.5` (+`clap_complete`) - CLI argument parsing (`crates/vox-cli/`)
- `axum` `0.8` + `axum-extra`, `hyper` `1`, `tower-http` `0.6.2` - HTTP servers (MCP HTTP transport, internal services)
- `tauri` `2` - desktop shell for `crates/vox-gui/`
- `ratatui` `0.30` + `crossterm` `0.29` + `reedline` `0.48` + `alacritty_terminal` `0.26` - TUI surfaces (interactive CLI/terminal panels)
- `wasmtime` / `wasmtime-wasi` `45.0.2` - WASM execution (plugin sandboxing)
- `tree-sitter` `0.26.9` (+ `tree-sitter-rust`, `tree-sitter-typescript`, `tree-sitter-python`) - code-intelligence graph (`vox graph`)
- `candle-core` / `candle-nn` / `candle-transformers` `0.10`, `tokenizers` `0.21`, `hf-hub` `1`, `qlora-rs`, `peft-rs` - local ML inference/fine-tuning (MENS model work, `crates/vox-plugin-mens-candle-*`)
- `turso` `0.6` (features `sync`) - embedded/replicated SQLite-compatible database driver, `crates/vox-db/`
- React `19.2.8`, `@tanstack/react-query` `5`, `zustand` `5`, `@xyflow/react`, `dockview`, `recharts` - GUI frontend (`crates/vox-gui/ui/package.json`)
- Astro `6` + Starlight `0.38` - documentation site (`docs-astro/package.json`)

**Testing:**

- `cargo nextest` - primary Rust test runner (invoked throughout `.github/workflows/ci.yml`, e.g. `cargo nextest run -p vox-compiler --test ...`)
- `insta` `1` (json), `criterion` `0.5` (html_reports), `serial_test` `3`, `wiremock` `0.6`, `tracing-test` `0.2` - Rust test/bench support
- `vitest` `3.2.6`, `@testing-library/react` `16`, `jsdom` `30` - GUI unit tests (`crates/vox-gui/ui/`)
- `@playwright/test` `1.62.1` (+ `@axe-core/playwright` `4.13`) - GUI E2E and accessibility (`crates/vox-gui/ui/e2e/`), also used by `docs-astro`

**Build/Dev:**

- `vite` `8.2.2` + `@vitejs/plugin-react` `5.2` - GUI frontend bundler
- `style-dictionary` `5.5.2` - design token build (`crates/vox-gui/ui/style-dictionary.config.mjs`)
- `tailwindcss` `4.3.3` + `@tailwindcss/postcss` - GUI styling
- `typify` `0.7`, `schemars` `1`, `jsonschema` `0.46`, `openapiv3` `2` - schema/contract codegen
- `syn` `2`, `prettyplease` `0.2`, `proc-macro2` `1` - Rust codegen/macro tooling (`vox-config-derive`, `vox-codegen`)

## Key Dependencies

**Critical:**

- `vox-secrets` (`crates/vox-secrets/`) - single secret-resolution facade; all API keys/tokens flow through `SecretId` + `SecretSpec` registry in `crates/vox-secrets/src/spec/registry/*.rs`
- `vox-actor-runtime::llm` (facade in `crates/vox-actor-runtime/`, config in `crates/vox-llm-config/`, egress in `crates/vox-llm-egress/`) - model-agnostic LLM call boundary; no direct vendor SDK/hostname calls permitted elsewhere (enforced by `vox-code-audit` detector `llm_provider_call`, `crates/vox-code-audit/src/detectors/llm_provider_call.rs`)
- `vox-crypto` (`crates/vox-crypto/`) - sole permitted entry point for application cryptography (ChaCha20-Poly1305, BLAKE3, SHA3-256, ed25519, X25519); direct imports of `chacha20poly1305`, `ed25519-dalek`, `x25519-dalek`, `blake3`, `sha2`, `sha3` are banned outside this crate
- `rustls` `0.23` (feature `ring`), `tokio-rustls` `0.26` - transport TLS provider (pinned; see `docs/src/architecture/cryptography-ssot-2026.md`); `aws-lc-rs` also present transitively via `reqwest 0.13`-linked deps (`chromiumoxide`, `gix`→`jj-lib`→`vox-vcs`) — two providers coexist deliberately, tracked in `contracts/crypto/transport-providers.v1.json`
- `iroh` `1.1` (+ `iroh-mdns-address-lookup`, `iroh-tickets`) - P2P mesh transport (`crates/vox-mesh-transport/`)
- `jj-lib` `0.42`, `gix` `0.84` - version control integration (`crates/vox-vcs/`, `crates/vox-git/`) for jujutsu/git-colocated repos
- `petgraph` `0.8.3`, `leiden-rs` `0.8.1` - code-intelligence graph engine (`vox graph`, crate-map community detection)
- `tantivy` `0.22` - lexical search index backing `vox-search`'s hybrid retrieval stack

**Infrastructure:**

- `tracing` `0.1` + `tracing-subscriber` `0.3` (env-filter, json) - structured logging/telemetry across all crates
- `vox-telemetry` / `vox-telemetry-otlp` (`crates/vox-telemetry*`) - first-party telemetry emission; ships events to the standalone `server/telemetry` ingest service
- `sysinfo` `0.39`, `nvml-wrapper` `0.12` - host/GPU resource inspection (build broker, model placement)
- `governor` `0.10` - rate limiting
- `parking_lot` `0.12`, `dashmap` `6` - concurrency primitives

## Configuration

**Environment:**

- `.env` (present, git-ignored - not read for this analysis) and `.env.example` (template; canonical keys `GEMINI_API_KEY`, `OPENROUTER_API_KEY`, `VOX_DB_URL`, `VOX_DB_TOKEN`, `VOX_DB_PATH`, `VOX_GITHUB_TOKEN`, `PORT`)
- Full secret surface (100+ keys across LLM providers, mesh, social publishing, scholarly APIs, platform/deploy) defined in `crates/vox-secrets/src/spec/ids.rs` (`SecretId` enum) and resolved per-key in `crates/vox-secrets/src/spec/registry/{llm,mesh,social,scholarly,platform,identity,config,core_ids}.rs`
- Runtime config loading: `crates/vox-config/` (see `bootstrap_inference.rs`), derive macros in `crates/vox-config-derive/`
- Never read secrets directly from `env::get` in consumer code - always `vox_secrets::resolve_secret(...)` (policy in `AGENTS.md` §Secret Management)

**Build:**

- `Cargo.toml` (workspace root) - 138 `crates/*` members, `[workspace.dependencies]` centralizes all external crate versions
- `.cargo/config.toml` - build broker aliases (`gui-build`, `gui-test`, `gui-check`), `jobs = 24`, `rustdocflags = ["-D", "warnings"]`
- `contracts/ci/crate-edges.allow.v1.json`, `contracts/ci/crate-layers.v1.json` - CI-gated dependency/layer ratchets (`vox ci crate-edges`)
- `docs-astro/astro.config.*`, `crates/vox-gui/ui/vite.config.*`, `crates/vox-gui/ui/tauri.conf.json` - frontend build configs

## Platform Requirements

**Development:**

- Rust `1.98.1` pinned toolchain, C compiler (openssl-sys, wry/keyring deps), Node + pnpm for the two JS packages
- Linux GUI dev additionally needs `libdbus-1-dev`, `libglib2.0-dev`, `libgtk-3-dev`, `libwebkit2gtk-4.1-dev`, `libsoup-3.0-dev`, `libjavascriptcoregtk-4.1-dev` (see `Dockerfile`)
- Build-toolchain invariant (per `AGENTS.md`): a clean clone must build with only the pinned Rust toolchain, platform C compiler, and Node+pnpm — no cmake, nasm, Go, perl, or libclang may enter via any dependency
- macOS (this machine): `pwsh` preferred for the two retained launcher scripts (`scripts/windows/vox-dev.ps1`, `scripts/vox-dev.sh`) and interactive work; GNU coreutils `timeout` at `/opt/homebrew/bin/timeout`

**Production:**

- `Dockerfile` - multi-stage build (`rust:1.98.1-slim-bookworm` builder → `debian:bookworm-slim` runtime), builds `vox-cli` binary only, `~50MB` target image, exposes ports `3000`/`9847`, healthcheck via `vox doctor --probe`
- `Dockerfile.ci-runner` - separate image for self-hosted CI runners
- Deployment workflows: `.github/workflows/deploy-hetzner.yml` (Hetzner target), `.github/workflows/deploy-telemetry.yml` + `.github/workflows/docker-telemetry.yml` (telemetry ingest service), `.github/workflows/coolify-eval-sync.yml` (Coolify-based eval environment)
- `server/telemetry/` - standalone Cargo workspace (excluded from root workspace) for the OTLP/HTTP → ClickHouse ingest server, `axum` `0.7` + `clickhouse` `0.13.3`

---

*Stack analysis: 2026-09-22*
