---
title: "CI runner contract"
description: "Official documentation for CI runner contract for the Vox language. Detailed technical reference, architecture guides, and implementation"
category: "CI & Quality"
training_eligible: true

schema_type: "TechArticle"
---

# CI runner contract

## Runners

**Default to a GitHub-hosted runner** — `ubuntu-latest`, with
`windows-latest` / `macos-latest` where a job needs them. Every gate, every
nightly lane, and every release lane is hosted: there is no capacity pool to
keep warm and no ledger of "hosted exceptions" to register. vox is a public
repo, so hosted minutes are free, and GitHub keeps the images and the runner
application current, so there is no runner-version floor to check.

**Two GPU lanes still carry a `self-hosted` label, and both starve today.**
`ml_data_extraction.yml`'s `extract` (`[self-hosted, linux]`) and `train`
(`[self-hosted, linux, x64, gpu]`) jobs, and the `mens-candle-cuda` row of
`nightly-artifacts.yml`'s plugin matrix, name labels with **zero registered
runners**. They are kept as documentation of the target shape — the CUDA
lanes cannot run on a hosted runner — and the plugin row is explicitly
`if:`-skipped so it does not queue forever and starve its matrix.
`ml_data_extraction.yml` is `schedule` + `workflow_dispatch` only, so a
starved run costs nothing on the PR path. Do **not** copy this label onto a
new job; run CUDA work locally instead.

> **Historical note.** Through 2026-09 this contract described a local
> self-hosted fleet (Docker ephemeral runners on the operator's WSL2 VM,
> labelled `linux` / `docker` / `browser` / `gpu`) as the default, with only
> the merge gate and the deploy path on hosted runners. That fleet is
> **retired**: the hosted-primary migration moved every workflow to hosted
> runners and Tasks 9/10/14 of
> `docs/superpowers/plans/2026-09-22-ci-audit-remediation.md` deleted the
> fleet's tooling (`vox ci queue`, `runner-scale`, `runner-preflight`,
> `runner-status`, the runner image and its scripts). Do not add a
> runner-version check or an operator host-hygiene step; there is no host to
> keep hygienic. The two starved GPU labels above are the only survivors.

## Local-first CI (required policy, ENFORCED)

Hosted CI is the **gate**, not the **inner loop**. A push that fails a gate
costs minutes of round-trip, so reproduce the gates locally first and push
only once they are green:

- **`vox ci pre-push`** (fast / `--complete` / `--full`) is the supported
  entry point and mirrors the `linux` leg's fast tier.
- **`act pull_request -j linux`** runs the PR gate's heavy leg in Docker
  when you want the real workflow rather than the tier.
- For a single failing job, run that job's exact command (e.g.
  `cargo run -q -p vox-arch-check`) instead of re-pushing to see.

See [local CI parity](../contributors/local-ci-pre-push.md) for the tier
table and wall-clock expectations.

## Workspace root manifest (fix forward)

Do **not** depend on git history to recover the root `Cargo.toml`. SSOT and repair steps: [workspace root manifest](workspace-root-manifest.md). Verify resolution with **`vox ci manifest`** (CI runs this via `cargo run -p vox-cli --quiet -- ci manifest`).

## Agent / local terminal vs CI shell

- **CI jobs** in this repository run on GitHub-hosted Linux (`ubuntu-latest`) and use **`bash`** for workflow steps unless a job sets `shell: pwsh` (see individual workflows). That is a runner convenience, not a contradiction of contributor policy.
- **Local work and coding agents** should prefer **[PowerShell 7 (`pwsh`)](https://github.com/PowerShell/PowerShell)** on **any OS** when it is installed, consistent with [`AGENTS.md`](../../../AGENTS.md) and machine-checked terminal policy (`vox shell check`, [`contracts/terminal/exec-policy.v1.yaml`](../../../contracts/terminal/exec-policy.v1.yaml)).

## Canonical `vox ci` vs shell scripts

Guard logic lives in **`vox ci`** (`crates/vox-cli/src/commands/ci`). Shell scripts under `scripts/` are **optional thin delegates** for local POSIX ergonomics; **prefer `vox ci …`** when the `vox` binary is on `PATH`. Mapping table: [scripts/README.md](../adr/index.md). Machine-readable registry: [`docs/agents/script-registry.json`](../../agents/script-registry.json).

## Pre-push validation (Linux CI mirror)

Use **`vox ci pre-push`** to run the merge-blocking subset locally. It **always** runs `cargo fmt --check`, **`vox ci line-endings`**, **`vox ci ssot-drift`**, **`vox-doc-pipeline --lint-only`** (frontmatter + fenced code), **`vox ci doctest-md --strict`**, and **`vox-drift-check`** so a green pre-push matches the **docs-quality** CI lane. Unless **`--quick`**, it also runs **`vox ci doc-inventory verify`**, workspace **`cargo clippy --all-targets -D warnings`**, and scoped TOESTUB on changed **`crates/<name>`** paths. **`--quick`** skips **only** doc-inventory, clippy, and TOESTUB — not the doc lint / doctest-md / drift steps (see [local CI parity](../contributors/local-ci-pre-push.md) for accurate wall-clock expectations).

**`--full`** appends workspace **`cargo nextest run --workspace --profile ci --no-fail-fast`**, matching the **`ci`** nextest profile in **`.config/nextest.toml`** (same choice as GitHub `ci.yml` when running plain nextest / llvm-cov nextest).

**Structured timings:** **`--report-json <path>`** writes **`contracts/reports/pre-push-report.v1.schema.json`**. Env **`VOX_PREPUSH_AUDIT_LOG=<path>`** appends one JSON line per **successful** run (omit on **`--dry-run`**) to detect repeated heavy pre-push usage during iteration.

**Doctests:** keep **`cargo test --workspace --doc`** for workspace doctest discovery; **`cargo-nextest`** does not run Rust doctests, so CI keeps doctests on the built-in harness until a verified doctest runner path exists for nextest. **`vox ci pre-push --full`** inherits that gap: the extra nextest pass does not substitute for **`cargo test --workspace --doc`**.

<a id="cargo-incremental-cache-troubleshooting-ai-multi-terminal"></a>

### Cargo incremental cache: troubleshooting (AI / multi-terminal)

Repeated “full rebuild” symptoms are often **cache fragmentation**, not Rust forcing a clean build:

- **Unified target dir:** repo **`.cargo/config.toml`** sets **`CARGO_TARGET_DIR`** to **`target/`** (relative to the repo root) so worktrees share one cache.
- **Anti-pattern:** different shells export different **`CARGO_TARGET_DIR`** values (**`target-agent-ssot`**, **`target-ci-prepush`**, etc.). Each distinct root **does not** reuse incremental artifacts from **`target/`**.
- **Audit:** run **`vox ci dev-loop-audit`** (or **`--json`**) before a long session; prefer **one** target dir per task, or **unset** **`CARGO_TARGET_DIR`** for inner-loop edits.
- **Inner loop:** **`cargo check -p <crate>`** → **`cargo nextest run -p <crate> --profile ci`** (or filtered **`cargo test`**); reserve **`vox ci pre-push`** for push readiness. See [AI dev loop overhead (2026)](../architecture/ai-dev-loop-overhead-2026.md).

## Line endings (cross-platform)

- **Policy:** LF for tracked source/docs/config (see root [`.gitattributes`](../../../.gitattributes) and [`.editorconfig`](../../../.editorconfig)). **`*.ps1`** uses CRLF on checkout / in editors that respect EditorConfig.
- **CI gate:** **`vox ci line-endings`** — forward-only by default (diff vs `GITHUB_BASE_SHA`…`GITHUB_SHA` in GitHub Actions, else `HEAD~1`…`HEAD` locally). Audit whole tree with **`--all`**. Override base with **`VOX_LINE_ENDINGS_BASE`** or **`--base <ref>`** (optional **`VOX_LINE_ENDINGS_HEAD`**, default `HEAD`).
- **TOESTUB:** rule id **`cross-platform/line-endings`** / finding **`cross-platform/crlf`** (warning) on scanned languages — see [governance](../../agents/governance.md).

**ML / repo hygiene (Rust, not shell):**

- **`vox ci grammar-export-check`** — wired in **`nightly.yml`**; asserts grammar exports are non-empty (EBNF/GBNF/Lark/JSON-Schema).
- **`vox ci grammar-drift`** — SHA-256 of the EBNF export vs `mens/data/grammar_fingerprint.txt` (and Populi twin); updates the file when drift is detected. The **`ml_data_extraction.yml`** workflow runs this with **`--emit github`** (stdout: `drift=true|false` only, for `GITHUB_OUTPUT`). (The former `--emit gitlab` output channel was removed when the GitLab CI mirror was retired.)
- **`vox ci repo-guards`** — replaces ad-hoc `grep`/`find` blocks: no `TypeVar(0)` in **`vox-codegen-rust` / `vox-codegen-ts` sources** (typechecker uses that sentinel legitimately), filtered `opencode` references under `crates/`, and no stray root clutter files (same policy as the former GitLab `guards` job).

## Build timings (wall-clock `cargo check`)

**Canonical:** **`vox ci build-timings`** — prints duration for `cargo check -p vox-cli` (default features) and `cargo check -p vox-cli --features gpu,mens-qlora,stub-check`, plus an optional CUDA lane when `nvcc` is available (**`PATH`** or **`CUDA_PATH`** / **`CUDA_HOME`** pointing at the toolkit root; same skip rules as `cuda-features`). Use **`--json`** for one JSON object per line. **`--crates`** adds isolated `cargo check` lanes for `vox-cli --no-default-features`, `vox-db`, `vox-oratio`, `vox-populi --features mens-train`, and **`vox-cli --features oratio`** (see [crate-build-lanes migration](../archive/research-2026-q1/crate-build-lanes-migration.md)). Soft budgets: `docs/ci/build-timings/budgets.json`; optional env **`VOX_BUILD_TIMINGS_BUDGET_WARN=1`** (stderr when a lane exceeds its soft max) and **`VOX_BUILD_TIMINGS_BUDGET_FAIL=1`** (fail the command after successful checks — use only with tuned budgets). Pair committed **`latest.jsonl`** with **`docs/ci/build-timings/snapshot-metadata.json`** (`rustc` / host / CUDA / cache note). Skip CUDA lane when **`SKIP_CUDA_FEATURE_CHECK=1`**. **`nightly.yml`**'s `audits` job runs **`build-timings --crates`**. See [vox-cli build feature inventory](../archive/research-2026-q1/vox-cli-build-feature-inventory.md).

## Optional CUDA compile gate

**Canonical:** **`vox ci cuda-features`** (wired in `nightly.yml`'s `audits` job). It **no-ops** when `nvcc` is absent, which is always the case on GitHub-hosted runners. When `nvcc` is on `PATH` — i.e. locally — it runs:

- `cargo check -p vox-oratio --features cuda` — typechecks Oratio's `#[cfg(feature = "cuda")]` paths.
- `cargo check -p vox-cli --features gpu,mens-candle-cuda` — typechecks Mens Candle qlora with CUDA.

Thin delegate: `scripts/check_cuda_feature_builds.sh` (optional POSIX wrapper around the same checks). Local escape hatch (e.g. Windows with CUDA installed but no MSVC host for `nvcc`): `SKIP_CUDA_FEATURE_CHECK=1 vox ci cuda-features` or the same env with `bash scripts/check_cuda_feature_builds.sh`. On PowerShell, use `bash -c 'export SKIP_CUDA_FEATURE_CHECK=1; ./scripts/check_cuda_feature_builds.sh'` so the variable reaches Bash.

## GPU / CUDA jobs

GitHub-hosted runners have no GPU, so **`vox ci cuda-features`** no-ops where
it is wired (`nightly.yml`'s `audits` job) — it skips when `nvcc` is absent.
Run it locally on a CUDA machine for real signal. Keep workflow
`runs-on` **explicit per job** (do not hide runner choice behind
reusable-only defaults).

## Optional: strict parse for all examples

Set **`VOX_EXAMPLES_STRICT_PARSE=1`** when running **`cargo test -p vox-compiler --test golden_examples_strict_parse`** so every `.vox` under **`examples/golden/`** parses with the production parser (see [`crates/vox-compiler/tests/golden_examples_strict_parse.rs`](../../../crates/vox-compiler/tests/golden_examples_strict_parse.rs)). Default CI keeps the **golden-only** gate. Status: [`examples/PARSE_STATUS.md`](../../../examples/PARSE_STATUS.md).

## Test hangs: `cargo test` vs `cargo nextest`

Rust’s built-in harness (**`cargo test`**) does **not** enforce per-test timeouts. After ~60 seconds it may print *“has been running for over 60 seconds”* — that is only a **warning**; the test keeps running until it finishes or you interrupt it.

**`cargo nextest run`** (used in GitHub `ci.yml`) reads **`.config/nextest.toml`**. There, **`slow-timeout`** marks slow tests and, with **`terminate-after`**, ends a stuck test after roughly **`terminate-after × period`** wall time (see [nextest slow tests](https://nexte.st/docs/features/slow-tests/)). The **`global-timeout`** setting caps the **entire** test run duration for a binary, not each case.

For local debugging of a single crate, prefer:

```bash
cargo nextest run -p vox-compiler --profile ci
```

Individual async tests can still wrap work in **`tokio::time::timeout`** so plain **`cargo test`** fails instead of hanging indefinitely.

### JUnit output (slow-test reporting)

Nextest writes JUnit when a profile defines **`[profile.<name>.junit]`** with **`path = "…"`** (see [JUnit support](https://nexte.st/docs/machine-readable/junit/)); output lands under **`target/nextest/<profile>/`** (e.g. **`target/nextest/ci/junit.xml`** for profile **`ci`**).

This repo does **not** commit that block in **`.config/nextest.toml`** by default. CI injects it **only in GitHub Actions** via **`--tool-config-file`** so the main config stays minimal: the **`tests`** job writes a tiny TOML fragment (same **`[profile.ci.junit]`** / **`path = "junit.xml"`**) under **`${RUNNER_TEMP}`**, then runs **`cargo llvm-cov nextest`** / **`cargo nextest run`** with **`--tool-config-file "vox-ci:${RUNNER_TEMP}/…"`**. The workflow uploads **`target/nextest/ci/junit.xml`** as artifact **`nextest-junit`** when present.

The same **`tests`** job now derives a runtime governance input artifact from that JUnit file when available:

- `cargo run -p vox-cli -- ci test-runtime-report --junit target/nextest/ci/junit.xml --json --top 20 > target/nextest/ci/runtime-report.json`
- `cargo run -p vox-cli -- ci flake-budget --report-json target/nextest/ci/runtime-report.json --max-candidates 20 --mode warn`
- `cargo run -p vox-cli -- ci ignored-test-age --mode warn`

All governance commands above are wired as **warn/non-blocking** in CI (they emit warnings/notices, never fail the job). If JUnit is absent (for example docs-only changes or early test failure), the workflow emits a notice and skips the governance/report generation path. CI uploads **`target/nextest/ci/runtime-report.json`** as artifact **`nextest-runtime-report`** when present.

CI also runs a **blocking snapshot drift gate** in the **`compiler-gates`** Rust job:

- `cargo run -p vox-cli -- ci test-inventory --check contracts/reports/test-inventory.v1.json`

Regenerate that committed snapshot when inventory rules change:

- `cargo run -p vox-cli -- ci test-inventory --output contracts/reports/test-inventory.v1.json`

`runtime-regress` is intentionally skipped in default `ci.yml` because this workflow does not currently materialize a stable baseline JSON artifact path for cross-run comparison. If a durable baseline artifact contract is introduced later, wire: `vox ci runtime-regress --baseline <stable-baseline.json> --current target/nextest/ci/runtime-report.json --mode warn`.

Summarize an artifact locally:

```bash
cargo run -p vox-cli -- ci test-runtime-report --junit target/nextest/ci/junit.xml --markdown /tmp/runtime.md
```

Optional governance gates (default **warn**, non-blocking) reuse that JSON or JUnit: `vox ci flake-budget --junit target/nextest/ci/junit.xml`, or `vox ci flake-budget --report-json /tmp/runtime.json` after capturing **`test-runtime-report --json`**. Compare slow-test regressions between CI runs with **`vox ci runtime-regress --baseline baseline.json --current current.json`** (both files from **`test-runtime-report --json`** with the same **`--top`** when possible).

## Targeted backend reruns

For routing/telemetry/capability-policy changes, prefer narrow reruns before full workspace passes:

- `cargo test -p vox-actor-runtime`
- `cargo test -p vox-db`
- `cargo test -p vox-orchestrator`

Use these focused lanes during iteration, then finish with `vox ci pre-push` (or CI lane equivalent) before merge.

## Merge-queue gate

The `main-merge-queue` ruleset is active and serializes every merge through a `merge_group`
`ci.yml` run. The sole required context is **`Check, Build, and Test (Rust)`**, entirely on
GitHub-hosted (`ubuntu-latest`) runners — there is no self-hosted fleet dependency, so there
is no outage valve to reach for. If the queue is wedged for an unrelated reason, temporarily
relax the ruleset. `PUT /rulesets/{id}` replaces the whole resource, so a partial
`-f enforcement=evaluate` would wipe other fields — GET the full ruleset, change only
`enforcement`, and PUT the complete body back:

```bash
RID=$(gh api repos/vox-foundation/vox/rulesets --jq '.[] | select(.name=="main-merge-queue") | .id')
# Relax:
gh api repos/vox-foundation/vox/rulesets/$RID \
  | jq '{name, target, enforcement: "evaluate", conditions, rules, bypass_actors}' \
  | gh api -X PUT repos/vox-foundation/vox/rulesets/$RID --input -
# ...merge..., then restore:
gh api repos/vox-foundation/vox/rulesets/$RID \
  | jq '{name, target, enforcement: "active", conditions, rules, bypass_actors}' \
  | gh api -X PUT repos/vox-foundation/vox/rulesets/$RID --input -
```

## Workflow list

See [workflow enumeration](workflow-enumeration.md).

