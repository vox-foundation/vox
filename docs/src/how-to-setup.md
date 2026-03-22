# Setup & Installation

This guide covers everything you need to get Vox running on any platform.

## Quick Install (30 seconds)

```bash
git clone https://github.com/vox-foundation/vox && cd vox

# Linux / macOS / WSL
./scripts/install.sh

# Windows (PowerShell)
.\scripts\install.ps1
```

Both scripts ensure **rustup/cargo**, then run **`vox-bootstrap`** (`cargo run --locked -p vox-bootstrap`): same logic on every OS. See `crates/vox-bootstrap/README.md`.

| Flag / args | Effect |
|-------------|--------|
| `--dev` / `-Dev` (PS1) | Request rustfmt + clippy (with `--apply`) |
| `--install-clang` / `-InstallClang` | Install clang where supported (e.g. winget `LLVM.LLVM` on Windows) |
| `--apply` / `-Apply` | Actually run installs; without it, the tool **plans** only |
| `plan` | Machine plan as **JSON** on stdout (exit 1 if requirements missing); `plan --human` for debug text |

Examples: `./scripts/install.sh --install-clang --apply`, `.\scripts\install.ps1 -InstallClang -Apply`, `./scripts/install.sh plan`.

Then build the CLI with `cargo build -p vox-cli` and run **`vox setup`** for keys, wasm, and project checks.

## Cross-Platform Setup Wizard

After installing `vox`, run the built-in setup wizard:

```bash
vox setup                    # Interactive setup
vox setup --dev              # Include dev tools (clippy, nextest)
vox setup --non-interactive  # CI mode (reads env vars only)
```

The wizard checks:

| Check | Required? | How to Fix |
|---|---|---|
| Rust ≥ 1.90 (workspace `rust-version`) | ✅ | [rustup.rs](https://rustup.rs) |
| Node.js ≥ 18 | Optional | [nodejs.org](https://nodejs.org) |
| Git | ✅ | [git-scm.com](https://git-scm.com) |
| C compiler (MSVC/gcc/clang) | ✅ | Platform-specific (see below) |
| **clang** / **LLVM** (optional) | Optional | The workspace patches **`aegis`** with **`pure-rust`** defaults so typical **Windows + MSVC** builds do **not** require `clang-cl` for Turso. Use `scripts/install.* --install-clang` only if you hit a toolchain that still expects native crypto builds. |
| Google AI Studio Key | Recommended | Free at [aistudio.google.com/apikey](https://aistudio.google.com/apikey) |
| OpenRouter Key | Optional | [openrouter.ai/keys](https://openrouter.ai/keys) |
| Ollama | Optional | [ollama.com](https://ollama.com) |
| VoxDB directory writable | ✅ | `~/.vox/` must exist and be writable |

## AI Provider Keys

Vox uses a **three-layer model cascade** — you get free AI with just a Google account:

### Layer 1: Google AI Studio (Free, Primary)

No credit card required. Provides Gemini 2.5 Flash, Flash-Lite, and Pro.

```bash
# Get your key (takes 10 seconds):
# https://aistudio.google.com/apikey

vox login --registry google YOUR_KEY
```

### Layer 2: OpenRouter (Optional)

Free API key unlocks dozens of `:free` models (Devstral 2, Qwen3 Coder, Llama 4 Scout, Kimi K2). Paid key unlocks SOTA models (DeepSeek v3.2, Claude Sonnet 4.5, GPT-5, O3).

```bash
vox login --registry openrouter YOUR_KEY
```

### Layer 3: Ollama (Optional, Local)

Zero-auth local inference. Install Ollama, pull a model, and Vox auto-detects it.

```bash
ollama pull llama3.2
# Vox detects Ollama on localhost:11434 automatically
```

## Verify Your Environment

```bash
vox doctor
```

Example output:
```
  ✓  Rust / Cargo              cargo 1.82.0
  ✓  Node.js                   v20.11.0 (>= v18)
  ✓  Git                       git version 2.44.0
  ✓  C Compiler                MSVC Build Tools found
  ✓  Google AI Studio Key      configured (free Gemini models available)
  ○  OpenRouter Key (optional) not configured
  ○  Ollama Local (optional)   not running
  ✓  VoxDB directory           C:\Users\you\.vox (writable)

  ✓ All checks passed — you're ready to build with Vox!
```

## Docker

```bash
# Build from source
docker build -t vox .

# Optional: image with `vox mesh` (HTTP control plane)
docker build -t vox:mesh --build-arg VOX_CLI_FEATURES=mesh .

# Run MCP server
docker run -e GEMINI_API_KEY=... -p 3000:3000 vox

# MCP + in-container mesh sidecar (background `vox mesh serve` on 9847)
docker run -e VOX_MESH_MESH_SIDECAR=1 -e GEMINI_API_KEY=... -p 3000:3000 -p 9847:9847 vox:mesh

# Example multi-service mesh compose (see `examples/mesh-compose.yml`)
# docker compose -f examples/mesh-compose.yml up

# Full stack with docker compose
cp .env.example .env  # fill in GEMINI_API_KEY
docker compose up
```

## Platform-Specific Notes

### Windows
- **MSVC (C++):** `winget install -e --id Microsoft.VisualStudio.2022.BuildTools` (include **Desktop development with C++** workload in the installer UI when prompted).
- **clang-cl (Turso / aegis):** `winget install -e --id LLVM.LLVM` so `clang-cl.exe` is on `PATH` (often under `C:\Program Files\LLVM\bin`). Or run `.\scripts\install.ps1 -InstallClang`.
- **One-liner bootstrap:** `.\scripts\install.ps1 -Dev -InstallClang` then `cargo build -p vox-cli`.
- **WSL:** `wsl ./scripts/install.sh --dev --install-clang` avoids MSVC/clang-cl friction for some workflows.

### macOS
- **C Compiler:** `xcode-select --install` (ships `clang` for most crates).
- **Turso:** Usually satisfied by Xcode CLT; if `aegis` still fails, `brew install llvm` and follow Homebrew’s `PATH` notes.

### Linux
- **C Compiler:** `sudo apt-get install build-essential` (Debian/Ubuntu).
- **clang (recommended for Turso):** `sudo apt-get install clang` or `./scripts/install.sh --install-clang`.
