---
phase: 316-before-after-comparison-bench-01
reviewed: 2026-10-27T05:13:57Z
depth: standard
files_reviewed: 8
files_reviewed_list:
  - crates/vox-eval/src/reasoning_bench.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs
  - crates/vox-ml-cli/src/commands/mens/reasoning_eval.rs
  - crates/vox-ml-cli/src/commands/mens/mod.rs
  - crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs
  - crates/vox-ml-cli/src/commands/mens/populi/mens_tail_subcommands.rs
  - docs/src/how-to/how-to-train-mens-macos-metal.md
findings:
  critical: 1
  warning: 6
  info: 7
  total: 14
status: issues_found
---

# Phase 316: Code Review Report

**Reviewed:** 2026-10-27T05:13:57Z
**Depth:** standard (plus targeted cross-file checks into the candle-metal loader, `vox_populi::mens::hub` and hf-hub 1.0)
**Files Reviewed:** 8
**Status:** issues_found

Scope: the plan commits 88656e612, db5463ed9 and 2b4721d29 (`git diff 35dd5d679..2b4721d29 -- crates docs ':(exclude)crates/vox-orchestrator'`). 4bf115518, 35dd5d679 and 09a012bcc were not reviewed.

## Summary

Most of the focus areas hold up.
- **Absence record:** it never carries score keys.
- **Incomplete runs:** an `incomplete` status returns Err.
- **Pin parsing:** it requires exactly 40 lowercase hex characters.
- **Snapshot detection:** it joins the exact pinned revision and only stats files. It reads the index JSON but never a weight file.
- **Adapter check:** the manifest `base_model` is canonicalized against the snapshot. This matches what the trainer writes (`finalize.rs::resolved_serve_base_model` writes a local dir).
- **Identical items:** both sides get the same `loaded` slice vector.

`cargo nextest run -p vox-ml-cli --features gpu --lib reasoning_compare` passed 28 of 28 on this tree.

The `--out` guard is the weak point. It compares paths lexically, so it can be bypassed through a symlink, a `..` after a symlink, or a different letter case on macOS's default case-insensitive APFS. I reproduced the bypass with the real binary: the command deleted and overwrote a file inside the pinned HF snapshot (CR-01). The other problems:
- A non-`gpu` build silently exits 0.
- Hub-root resolution disagrees with the hf-hub client that `vox mens download` uses.
- The shard-completeness rule disagrees with the downloader's own shard filter.
- The compared report does not identify the adapter.
- `remove_file` runs on any unguarded `--out` path, including the bench slices.
- An unresolvable `--base` exits 0.

## Narrative Findings (AI reviewer)

## Critical Issues

### CR-01: `--out` guard is lexical; symlink, `..`-through-symlink and case variants write (and first delete) files inside the hub snapshot

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs:67-80, 100-111` (and the unconditional `remove_file` at `:133-137`)

**Issue:** `normalized()` uses `std::path::absolute` plus a lexical `..` pop. `Path::starts_with` then compares components byte for byte. Neither step resolves symlinks, folds case, or recognises `/tmp` as the same place as `/private/tmp`. Containment is therefore decided on the spelling of the path, not on where the file lands. `run_before_after_with` then calls `remove_file(out)` and writes the report there. Any bypass deletes a real file and replaces it with JSON.

I reproduced both cases with `cargo run -p vox-ml-cli --features gpu -- mens eval-reasoning-compare` against a scratch hub. The scratch hub held a complete fake `models--Qwen--Qwen3-8B/snapshots/b968826d…/`:
- `--out <scratch>/linkparent/snap/config.json`, where `linkparent/snap` is a symlink to the snapshot. Result: exit 0, `NOT COMPARED: adapter_absent`, and the snapshot's `config.json` was replaced by the 763-byte absence record.
- `--out <scratch>/HUB/models--Qwen--Qwen3-8B/snapshots/<rev>/tokenizer.json` with `--hub-root <scratch>/hub` (only the letter case differs). Result: exit 0, and the snapshot's `tokenizer.json` was overwritten.

In a real HF cache these files are symlinks into `blobs/`. `remove_file` unlinks the snapshot entry, which corrupts the cached base: later loads fail, or detection reports `base_snapshot_incomplete`. The same bypass also reaches the adapter dir (`adapter_manifest.json`, `candle_qlora_adapter.safetensors`). This breaks the must-have "any path under the hub root or the adapter dir is refused before anything is written". Mutation test 1 and `out_inside_hub_root_or_adapter_is_refused` only cover lexical paths, so the gap is untested.

The macOS case is the default configuration on the target platform (APFS is case-insensitive). A typo in letter case is enough to trigger it; no deliberate symlink is needed.

**Fix:** Decide containment by filesystem identity, not by spelling. Canonicalize the deepest existing ancestor of `out` and each root, then compare. Compare `(dev, ino)` as well, so case-insensitive volumes are covered:
```rust
fn resolved(p: &Path) -> Result<PathBuf> {
    let abs = std::path::absolute(p)?;
    // Walk up to the deepest existing ancestor, canonicalize it, re-append the rest.
    let mut tail = Vec::new();
    let mut cur = abs.as_path();
    while !cur.exists() {
        tail.push(cur.file_name().map(|n| n.to_owned()).unwrap_or_default());
        cur = cur.parent().context("no existing ancestor")?;
    }
    let mut base = cur.canonicalize()?;
    for seg in tail.into_iter().rev() { base.push(seg); }
    Ok(base)
}

fn is_under(out: &Path, root: &Path) -> Result<bool> {
    let Ok(root_meta) = std::fs::metadata(root) else { return Ok(false) };
    // Identity walk: catches case-insensitive and hard-to-spell aliases.
    let mut cur = Some(resolved(out)?);
    while let Some(p) = cur {
        if let Ok(m) = std::fs::metadata(&p) {
            #[cfg(unix)]
            {
                use std::os::unix::fs::MetadataExt;
                if m.dev() == root_meta.dev() && m.ino() == root_meta.ino() { return Ok(true); }
            }
        }
        cur = p.parent().map(Path::to_path_buf);
    }
    Ok(resolved(out)?.starts_with(resolved(root)?))
}
```
Also refuse an existing `out` that is a symlink: check `symlink_metadata(out)?.file_type().is_symlink()` before the `remove_file`. Add tests for a directory-symlink parent, a `link/..` path and (on macOS) a case-variant root. Each test must leave the target file byte-identical.

## Warnings

### WR-01: Non-`gpu` build accepts `eval-reasoning-compare`, does nothing, and exits 0

**File:** `crates/vox-ml-cli/src/commands/mens/populi/dispatch.rs:733-754`
**Issue:** The clap variant is not cfg-gated. The arm's non-gpu body is `let _ = (base, adapter, hub_root, bench_root, out); Ok(())`. `vox-ml-cli`'s default features are `["mens-base"]` with no `gpu`, so a default build parses the command, writes no record and prints nothing. It still exits 0. Phases 317-321 script this command once per size, and that loop would treat the silent no-op as success. The no-op also contradicts the contract that the absent branch always writes a record and prints `NOT COMPARED`. The sibling `EvalCollateralDamage` arm has the same flaw, but this phase copied it into a new command.
**Fix:** Under `#[cfg(not(feature = "gpu"))]`, return an error instead:
```rust
#[cfg(not(feature = "gpu"))]
{
    let _ = (base, adapter, hub_root, bench_root, out);
    anyhow::bail!("eval-reasoning-compare requires a vox-ml-cli build with --features gpu");
}
```

### WR-02: Hub-root order disagrees with the hf-hub 1.0 client that `vox mens download` writes through

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs:36-48`
**Issue:** `vox mens download` (`vox_populi::mens::hub::download_model`) uses `hf_hub::HFClient::new()`. hf-hub 1.0 (`constants.rs:33-52`) resolves the hub cache in this order: `HF_HUB_CACHE`, then `HUGGINGFACE_HUB_CACHE`, then `<hf_home>/hub`, where `hf_home` is `HF_HOME`, then `XDG_CACHE_HOME/huggingface`, then `~/.cache/huggingface`. `resolve_hub_root` skips both `HUGGINGFACE_HUB_CACHE` and `XDG_CACHE_HOME`. With either variable set, a snapshot that vox itself downloaded is reported as `base_snapshot_absent`. The run exits 0 and no comparison happens. This fails safe (no false score), but it gives a false negative on the exact base the operator just downloaded.
**Fix:** Mirror hf-hub's order. Pass in `HUGGINGFACE_HUB_CACHE` and `XDG_CACHE_HOME` the same way the other variables are passed (to keep tests env-free), and add the two cases to `hub_root_order`.

### WR-03: Shard completeness rule disagrees with the downloader's shard filter

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs:164-192`
**Issue:** `snapshot_complete` requires every distinct `weight_map` value to be a file. `download_model` (`vox-populi/src/mens/hub.rs:326-349`) deliberately downloads only the shards that carry text-tower tensors and skips vision- and MTP-only shards (`required_shard_filenames`). For any repo whose index names such shards, a snapshot fetched by `vox mens download` is complete for training and inference. Detection still reports it as `base_snapshot_incomplete`. `resolve_base_pin` already resolves `--base Qwen3.8-27B` (gpu-specs line 299), so this becomes reachable as soon as a non-dense rung is compared. The six dense Qwen3 sizes are not affected.
**Fix:** Use the same predicate as the downloader. Require only shards that hold at least one key where `!vox_hf_layout::is_vision_or_mtp_key(key)` is true, or move `required_shard_filenames` into a shared helper and call it here. Add a test with an index that names an MTP-only shard that is absent.

### WR-04: Compared report does not identify the adapter, and the per-base default path lets a second adapter overwrite the first

**File:** `crates/vox-eval/src/reasoning_bench.rs:308-339`; `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs:297-301`
**Issue:** `compared_report_json` records the base pin but not which adapter produced the `adapter` scores: no path, no manifest hash, no run id. The default `--out` is keyed only by base (`<size>-<rev12>.json`). Comparing two adapters trained on the same base therefore replaces the first record silently, and the surviving file cannot say which adapter it measured. For a before/after record that phases 317-321 will cite, the "after" side has no provenance.
**Fix:** Pass the adapter path (and ideally a BLAKE3 digest of `adapter_manifest.json` via `vox-crypto`) into `compared_report_json`, and emit it under `"adapter": {...}`. Either key the default file on the adapter run dir name as well, or document that `--out` is required when a base has more than one adapter.

### WR-05: `--out` may delete any unguarded file, including the checked-in bench slices it is about to read

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs:83-113, 133-137, 150-155`
**Issue:** The guard only knows the hub root and the adapter dir. `run_before_after_with` calls `remove_file(out)` before it loads anything, so `--out mens/data/gsm8k_think/reasoning.jsonl`, or any other existing file, is deleted and replaced with the report. On the present branch the slice is then loaded from the path that was just removed, so that slice becomes `not_scored` on both sides. On the absent branch the checked-in slice is replaced by the absence record. Both are silent data loss from a single mistyped flag.
**Fix:** Also refuse `--out` under `args.bench_root` (with the CR-01 identity check). Refuse an existing `out` unless it is a regular file that parses as JSON with `"schema": "reasoning_before_after.v1"`. That is the only stale file this command should replace.

### WR-06: An unresolvable `--base` (a typo or an unknown size) exits 0 as "not compared"

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs:76-79, 115-117`; `reasoning_compare.rs:305-311`
**Issue:** `resolve_base_pin` returns unknown or ambiguous input unchanged. Detection maps it to `PinUnverifiable`, and the entry returns `Ok` with exit 0. `--base Qwen3-8b` (lowercase), `--base Qwen3-1.7B` (no rung yet) and `--base Qwen3-8B@main` all write `<x>-unpinned.json` and succeed. That is an operator input error, not an absent snapshot. A per-size loop in phases 317-321 cannot tell "nothing on disk" apart from "asked for something that doesn't exist".
**Fix:** Keep the record if it is useful, but return Err for `AbsenceReason::PinUnverifiable`. Alternatively, reject an unresolvable `--base` in `resolve_base_pin`, before anything is written, with the list of known sizes. Reserve exit 0 for real on-disk absence.

## Info

### IN-01: `let _plugin = loaded;` contradicts its own comment

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs:276-278`
**Issue:** The comment says `loaded` "must outlive `handle`". Moving `loaded` into a binding declared after `handle` makes it drop *first*, because locals drop in reverse declaration order. This is harmless today: `unload_model` already ran, and `cached_code_plugin` keeps the library resident. But the stated invariant is the opposite of what the code does.
**Fix:** Delete the two lines and the comment, or bind `loaded` before `handle` if the order ever matters.

### IN-02: `slices` has a different JSON type per status within one schema

**File:** `crates/vox-eval/src/reasoning_bench.rs:291` vs `:336`
**Issue:** In `not_compared` records, `slices` is an array of names. In `compared` and `incomplete` records it is an object of entries. Any consumer that reads `v["slices"][name]` gets `null` from an absence record and must branch on `status` first.
**Fix:** Rename the absence field to `slice_names`, or emit an object with no score entries.

### IN-03: Unpinned report file names collide across owners

**File:** `crates/vox-eval/src/reasoning_bench.rs:198-217`
**Issue:** `foo/X` and `bar/X` both map to `X-unpinned.json`, and so do all pinned bases that share a last segment and the same first 12 hex characters. One record silently replaces the other.
**Fix:** Include a sanitized owner segment (`<owner>--<name>-<rev12>.json`).

### IN-04: `snapshot_dir` does not sanitize `repo_id` the way `report_file_name` does

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare/detect.rs:83-91`
**Issue:** Only `/` is replaced. On Windows a `\` in `--base` (`a\..\..\x@<sha>`) is a path separator, so detection can point outside the hub root. The out guard's hub-root containment then does not cover the directory that was actually used. Unix is unaffected.
**Fix:** Reject a `repo_id` that is not `[A-Za-z0-9._-]+/[A-Za-z0-9._-]+` in `BaseLabel::from_pin` and give it `revision: None`.

### IN-05: Base-side load failure still runs the full adapter side

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs:157-164`
**Issue:** If the base side comes back all `NotScored` (plugin missing, load error), the outcome is already `incomplete` and the run returns Err. The adapter side still loads and generates 96 items first.
**Fix:** Skip `score_side(Side::Adapter, ..)` when every base outcome is `NotScored` with a load-time reason, and mark the adapter side `not_scored` with "base side not scored".

### IN-06: Report write is not atomic

**File:** `crates/vox-ml-cli/src/commands/mens/reasoning_compare.rs:116-123`
**Issue:** `fs::write` straight to `out` after the stale file was removed. A kill during the write leaves truncated JSON at the canonical per-base path.
**Fix:** Write to `out.with_extension("json.tmp")` and then `rename`.

### IN-07: How-to text is imprecise and leaves out the feature requirement

**File:** `docs/src/how-to/how-to-train-mens-macos-metal.md:93-99`
**Issue:** The text says `<size>-<revision>.json`, but the name uses the first 12 hex characters of the revision. It also does not say the command does nothing unless the binary was built with `--features gpu` (see WR-01).
**Fix:** Write `<size>-<first 12 hex of revision>.json` and add the gpu-feature note, or drop the note once WR-01 makes non-gpu builds error.

---

_Reviewed: 2026-10-27T05:13:57Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
