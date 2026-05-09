---
title: "Local CI parity (pre-push)"
description: "Run the merge-blocking subset locally before every push via `vox ci pre-push`. Includes optional `--act` mode for GitHub-hosted exception workflows."
category: "contributors"
status: "current"
last_updated: "2026-05-09"
training_eligible: true
schema_type: "TechArticle"
---

# Local CI parity (pre-push)

`vox ci pre-push` runs the merge-blocking subset of `.github/workflows/ci.yml`
locally so failures show up before the GitHub round-trip.

## Modes

| Mode | Steps | Typical wall-clock |
|------|-------|--------------------|
| `--quick` | fmt-check, line-endings, ssot-drift | ~30 s |
| default | + doc-inventory verify, clippy (workspace), TOESTUB on changed `crates/<x>` | ~2–4 min |
| `--full` | + `cargo nextest run --workspace` | ~10–25 min |
| `--act` | + GH-hosted exception workflows via `act` (composable with any mode above) | +3–8 min |

## Install the git hook (one-time)

```bash
cargo run -q -p vox-cli -- ci install-hooks
```

This writes `.git/hooks/pre-push` as a one-line delegate to
`vox ci pre-push`. The generated stub honours
[AGENTS.md §VoxScript-First Glue Code](../../../AGENTS.md) — no business logic
in shell.

## Bypass

`git push --no-verify` skips the hook. Use sparingly; CI still runs.

## Tuning the diff base

The TOESTUB-scoped step looks at `git diff origin/main...HEAD` by default.
Override with `VOX_PREPUSH_BASE=<ref>` (e.g. `VOX_PREPUSH_BASE=HEAD~1`).

## `--act` mode (GH-hosted exception workflows)

`vox ci pre-push --act` additionally runs the workflows that target
`ubuntu-latest` (documented in [github-hosted-exceptions](../ci/github-hosted-exceptions.md))
inside Docker containers via [nektos/act](https://github.com/nektos/act).
Catches Node/pnpm version mismatches and link-checker regressions before push
instead of after.

**Workflows covered:** `docs-quality.yml`, `link_checker.yml`,
`ts-emit-noemit.yml`. (Workflows that depend on real Chromium, GPU, or
Docker-in-Docker capabilities cannot be reproduced by `act` and stay on the
self-hosted fleet — see [runner-contract](../ci/runner-contract.md).)

**Configuration:** [`.actrc`](../../../.actrc) at the repo root pins the
catalog image (`catthehacker/ubuntu:act-24.04`) and platform mappings.
Maintainers should bump the tag in lockstep with `actions-runner` upgrades.

### Installing `act`

`act` must be on `PATH`, OR available as the `gh` CLI extension `gh act`
(the `vox ci pre-push --act` lookup tries the standalone binary first, then
falls back to `gh act`). Docker Desktop (or another Docker daemon) must be
running.

#### Windows

Pick one of the following installers. All result in `act.exe` on `PATH`.

| Method | Command | Notes |
|--------|---------|-------|
| **WinGet** (recommended — ships with Windows 11) | `winget install nektos.act` | Auto-updates via `winget upgrade nektos.act`. |
| **Scoop** | `scoop install act` | Adds to user PATH; `scoop update act` to upgrade. |
| **Chocolatey** | `choco install act-cli` | Run as Administrator. |
| **GitHub CLI extension** | `gh extension install nektos/gh-act` | No PATH change needed; invoked as `gh act …`. The `vox ci pre-push --act` lookup detects this fallback. |
| **Manual** | Download `act_Windows_x86_64.zip` from [releases](https://github.com/nektos/act/releases), extract `act.exe` into a folder on `PATH` (e.g. `%USERPROFILE%\bin`). | Bump manually. |

After install, verify:

```powershell
act --version
docker version    # daemon must be reachable
```

**Docker requirement on Windows:** Docker Desktop with the WSL 2 backend is
the supported setup. The `--bind` flag in `.actrc` mounts `~/.cargo` into the
container — for that to work, your Windows `%USERPROFILE%\.cargo` must be on
a drive shared with WSL (Docker Desktop → Settings → Resources → File
Sharing). If the bind-mount fails, comment out `--bind` in `.actrc` and
expect slower first runs (cargo registry re-downloads in-container).

**Artifact path on Windows:** `.actrc` defaults to `--artifact-server-path
/tmp/act-artifacts`, which is a WSL path. If you hit permission errors,
override locally with `--artifact-server-path %TEMP%\act-artifacts` (or pass
the flag directly to `act` and skip the rc default).

#### macOS / Linux

| Method | Command |
|--------|---------|
| **Homebrew** (macOS, Linux with Homebrew) | `brew install act` |
| **install.sh** (Linux) | `curl -s https://raw.githubusercontent.com/nektos/act/master/install.sh \| sudo bash` |
| **GitHub CLI extension** (any OS) | `gh extension install nektos/gh-act` |

### Common `--act` failures and fixes

| Symptom | Cause | Fix |
|--------|-------|-----|
| `act: command not found` | Not on PATH and no `gh act` extension. | Install via the table above. |
| `Cannot connect to the Docker daemon` | Docker Desktop / `dockerd` not running. | Start Docker Desktop (Windows/macOS) or `sudo systemctl start docker` (Linux). |
| `unable to find image 'catthehacker/...': pull denied` | Network or rate-limit. | `docker login` (anon pull is rate-limited) or pre-pull the image. |
| Action version mismatch with self-hosted fleet | `.actrc` image tag drifted from `actions-runner`. | Bump the catalog image tag in [`.actrc`](../../../.actrc); see [runner-contract](../ci/runner-contract.md). |
