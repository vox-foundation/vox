# Summary 38-01: KB Bounded Session-Pinned Context Injection (`INJ-01..09`)

- **Status:** Complete (2026-10-15)
- **Requirements:** `INJ-01`, `INJ-02`, `INJ-03`, `INJ-04`, `INJ-05`, `INJ-06`, `INJ-07`, `INJ-08`, `INJ-09`
- **Changes:**
  - Implemented [`crates/vox-search/src/kb_injection.rs`](file:///Users/brbrainerd/dev/vox/crates/vox-search/src/kb_injection.rs):
    - `compute_kb_token_ceiling` (`INJ-01`) and `RELEVANCE_FLOOR` gating so off-topic turns inject 0 bytes.
    - Merged ranked list with summary-first ordering, `MAX_CHUNKS_PER_DOC = 2`, and lost-in-the-middle edge ordering (`INJ-02`).
    - `apply_cutoff_boost` (`INJ-03`) restricted to `scope: "external"` using `verified_at` (never git `mtime`).
    - Per-turn untrusted spotlight boundary delimiters (`<<<KB_UNTRUSTED_BOUNDARY:{nonce}>>>`) and `should_gate_exfiltration_tools` (`INJ-04`).
    - Post-generation `[kb:<id>]` / `[@kb:<id>]` citation validator (`validate_kb_citations`, `INJ-05`).
    - Structured `KbContextEvent` with `injected` items and `dropped` candidates + `DropReason` (`INJ-06`).
    - `PINNED_KB_TOOLS` surviving per-turn tool caps (`INJ-07`), remote `sensitivity: private` egress blocking (`INJ-08`), and `KbSessionScope` (`INJ-09`).
  - Verified in `cargo --config 'build.rustc-wrapper=""' test -p vox-search --lib` (`267 passed, 0 failed`).
