---
phase: 442-bidirectional-chat-harness-and-ast-security
plan: 01
subsystem: security
tags: [terminal, exec-policy, security-tier, ast, oversight, floor, vox-orchestrator, vox-cli-research]

requires:
  - phase: 441-stateful-pty-and-lossless-streaming
    provides: Stateful SessionHost, SecurityLevel, and ControlMode foundations
provides:
  - Pure-Rust POSIX + PowerShell quote-aware command tokenizer and segment splitter (tokenize_and_split_segments) in vox_orchestrator::oversight::exec_policy
  - ExecPolicyV1 schema/policy evaluator (default_exec_policy_v1, evaluate_command_ast) and recursive wrapper unwrapping (extract_effective_commands)
  - L1-L5 SecurityTier command policy evaluator (evaluate_security_tier) rejecting subshell $(), backticks, redirections, encoded wrappers, and find -exec bypasses
  - Multi-segment max-floor shell classification in vox_orchestrator::oversight::floor::classify_floor across compound shell pipelines and wrappers
  - Thin check_terminal.rs delegation to vox_orchestrator::oversight::exec_policy preserving vox shell check and vox ci exec-policy-contract parity
affects: [442-03, 442-04, 442-05, 445, vox-orchestrator, vox-terminal-core, vox-orchestrator-d, vox-orchestrator-mcp, vox-cli-research]

actuals:
  tokens: 8420
  tasks: 2
  commits: 0

tech-stack:
  added: []
  patterns:
    - "Pure-Rust quote-aware POSIX + PowerShell lexical segmenter handling |, ||, &&, ;, newlines, backticks, $(), ${}, > / >> / 2> redirections, and fail-closed unclosed quotes"
    - "Recursive wrapper binary unwrapping (sudo, env, nohup, timeout, nice, xargs, watch, sh/bash/zsh -c, pwsh/powershell -Command/-EncodedCommand, cmd /c, find -exec/-execdir/-delete)"
    - "Effective oversight floor = max(ActionFloor across all extracted segments and unwrapped inner commands)"

key-files:
  created:
    - crates/vox-orchestrator/src/oversight/exec_policy.rs
  modified:
    - crates/vox-orchestrator/src/oversight/mod.rs
    - crates/vox-orchestrator/src/oversight/floor.rs
    - crates/vox-cli-research/src/check_terminal.rs

key-decisions:
  - "Hosted ExecPolicyV1, tokenize_and_split_segments, extract_effective_commands, evaluate_command_ast, and evaluate_security_tier in vox_orchestrator::oversight::exec_policy so vox-terminal-core, vox-orchestrator-mcp, and vox-cli-research share one SSOT with zero new crate-edges"
  - "Evaluated L1-L5 SecurityTier directly from AST structure + read-only/workspace allowlists rather than raw exec-policy.v1.yaml command-not-allowed lists so benign shell builtins (cd, pwd, echo) never false-park in L1/L2 while subshells, redirections, and encoded wrappers fail closed"
  - "Kept PowerShell System.Management.Automation.Language.Parser AST extraction in vox-cli-research::check_terminal layered atop shared evaluate_command_ast for defense-in-depth when pwsh is installed"

patterns-established:
  - "All terminal and shell tool command strings pass through multi-segment wrapper-unwrapping AST inspection before tier classification or PTY execution"

requirements-completed:
  - TERM-HARNESS-03

coverage:
  - id: D1
    description: "Pure-Rust POSIX + PowerShell quote-aware ExecPolicyV1 parser, wrapper unwrapping, and L1-L5 SecurityTier evaluator blocking $(), backticks, redirections, pwsh -EncodedCommand, and find -exec bypasses"
    requirement: TERM-HARNESS-03
    verification:
      - kind: unit
        ref: "crates/vox-orchestrator/src/oversight/exec_policy.rs#tests"
        status: pass
    human_judgment: false
  - id: D2
    description: "Multi-segment classify_floor shell command escalation across compound pipelines/wrappers and check_terminal.rs delegation"
    requirement: TERM-HARNESS-03
    verification:
      - kind: unit
        ref: "crates/vox-orchestrator/src/oversight/floor.rs#tests"
        status: pass
      - kind: integration
        ref: "crates/vox-cli/src/commands/ci/exec_policy_contract.rs#run"
        status: pass
    human_judgment: false

duration: 185min
completed: 2026-11-15
status: complete
---

# Phase 442 Plan 01: Pure-Rust AST Exec-Policy, `L1–L5` Security Tiers & Multi-Segment Floor Summary

**Quote-aware POSIX + PowerShell `ExecPolicyV1` and `L1–L5` `SecurityTier` evaluator in `vox-orchestrator::oversight::exec_policy`, multi-segment wrapper-unwrapping `classify_floor`, and `check_terminal.rs` delegation (`TERM-HARNESS-03`)**

## Performance

- **Tasks:** 2 / 2 completed
- **Files created/modified:** 4 (`crates/vox-orchestrator/src/oversight/{exec_policy.rs,mod.rs,floor.rs}`, `crates/vox-cli-research/src/check_terminal.rs`)

## Verification Evidence

- `cargo test -p vox-orchestrator --lib oversight`: **`24 passed; 0 failed`** (`task-498`, covering quote-aware splitting, unclosed quote fail-closed errors, `$()`/backtick/redirection detection, `sudo`/`env`/`sh -c`/`pwsh -EncodedCommand`/`find -exec` unwrapping, `L1`–`L5` tier boundaries, and multi-segment `classify_floor` escalation).
- `crates/vox-cli-research/src/check_terminal.rs`: compiled with `0` errors and `0` warnings (`task-601`).
- `vox ci exec-policy-contract`: **`exec-policy-contract: rust fallback OK (6 hardcoded + 10 disk)`** (`exit 0`).
- `vox ci secret-env-guard` & `vox-arch-check`: **`0` violations** across all Phase 442 modified files (`0` new `crate-edges`).
