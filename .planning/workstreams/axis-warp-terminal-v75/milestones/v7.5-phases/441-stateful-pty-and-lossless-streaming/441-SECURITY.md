---
phase: "441"
slug: "stateful-pty-and-lossless-streaming"
status: verified
threats_open: 0
asvs_level: 1
created: "2026-11-14"
---

# Phase 441 — Security

> Per-phase security contract: threat register, accepted risks, and audit trail.

---

## Trust Boundaries

| Boundary | Description | Data Crossing |
|----------|-------------|---------------|
| Shell launch shim (`vox-terminal-core::shell_init`) → Local filesystem / child PTY | Temporary shell bootstrap scripts (`zsh`, `bash`, `fish`, `pwsh`, `nu`) written on disk and sourced at session start | Owner-only `0700` `TempDir` hook scripts, user `$HOME` / `$XDG_CONFIG_HOME` paths |
| Child PTY process stdout/stderr → `Osc633Parser` & `VtShadow` & `LocalHost` | Untrusted byte stream, incomplete/malformed escape sequences, multi-byte UTF-8 splits, and high-throughput output bursts | Raw terminal bytes, `OSC 7` / `OSC 633` markers, `ALT_SCREEN` mode transitions |
| Loopback JSON-RPC client → `TermDispatch` (`vox-orchestrator-d`) | Wire requests (`term.open`, `term.attach`, `term.input`, `term.submit`, `term.signal`, `term.resize`) arriving at the daemon terminal host | Daemon auth token, `origin` attribution, `after_seq` cursor, signal kinds, grid dimensions |
| `SessionHost` byte stream → `VtGrid` (`vox-term`) & Tauri IPC (`vox-gui`) | Live PTY frames and resize events crossing into TUI and GUI renderers | Streaming UTF-8 chunks, `SessionEvent` frames, terminal dimensions |

---

## Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation | Status |
|-----------|----------|-----------|----------|-------------|------------|--------|
| T-441-01 | Tampering / Information Disclosure | `crates/vox-terminal-core/src/shell_init.rs` | high | mitigate | `tempfile::Builder::new().prefix("vox-sh-")`: `0700` owner-only permissions on Unix (`PermissionsExt::from_mode(0o700)`), single-quote/literal path escaping in `fish`/`nu`/`zsh`/`bash`, hex-escaped `OSC 633;P;Cwd` values (`WR-09`), and RAII `TempDir` + `Child` reap on `PtyHandle` drop (`WR-05`). Verified in `shell_init_and_stateful_pty`. | closed |
| T-441-02 | Denial of Service | `pty.rs`, `vt_shadow.rs`, `vox-term/src/vt.rs`, `term_dispatch.rs` | medium | mitigate | `cols.max(1)` and `rows.max(1)` clamped across `PtyHandle::resize`, `VtShadow::new`/`resize`, and `VtGrid::new`/`resize`, plus `dim`/`req_dim` validation in `TermDispatch` (`120×32` defaults). Verified in `shell_init_and_stateful_pty` and `vox-term` `headless` tests. | closed |
| T-441-03 | Denial of Service | `osc633.rs`, `host.rs`, `daemon_host.rs`, `vox-term/src/app.rs`, `vox-gui/src/commands/pty.rs` | medium | mitigate | `Osc633Parser` caps incomplete OSC sequences at `16 KiB` and aborts broken OSC strings on embedded `ESC`/`CAN`/`SUB` (`WR-06`); `LocalHost` bounds `OutputFrame` replay ring at `2 MiB` (`after_seq` delta + `resync_required`); `DaemonHost`, `vox-term`, and `vox-gui` recover from broadcast lag via `SnapshotResync` (`WR-03`). Verified in `streaming_utf8_and_alt_screen` and `term_dispatch` tests. | closed |
| T-441-04 | Elevation of Privilege / Tampering | `host.rs`, `term_dispatch.rs`, `vox-term/src/app.rs` | high | mitigate | `TermDispatch` enforces daemon auth token before dispatch and calls `refuse_agent_origin(p)?` fail-closed at the top of `term.input`, `term.submit`, and `term.signal`; `TermSecurityLevel::allows_agent_command` blocks multi-line/chaining/subshell/redirection/wrapper bypasses and cross-origin stdin injection (`CR-01`). Verified in `term_dispatch` and `vox-terminal-core` `host` tests. | closed |

---

## Accepted Risks Log

No accepted risks.

---

## Security Audit Trail

| Audit Date | Threats Total | Closed | Open | Run By |
|------------|---------------|--------|------|--------|
| 2026-11-14 | 4 | 4 | 0 | Antigravity (`gsd-secure-phase` + `441-REVIEW-FIX`) |

---

## Sign-Off

- [x] All threats have a disposition (mitigate / accept / transfer)
- [x] Accepted risks documented in Accepted Risks Log
- [x] `threats_open: 0` confirmed
- [x] `status: verified` set in frontmatter

**Approval:** verified 2026-11-14
