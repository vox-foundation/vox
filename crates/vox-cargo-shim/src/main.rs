//! Daemonless `cargo` shim for the vox build broker.
//!
//! Placed on PATH ahead of the rustup cargo proxy. For build subcommands it
//! serializes per effective `CARGO_TARGET_DIR` via `FairQueue` BEFORE claiming a
//! slot from the machine-wide concurrency cap, coalesces identical idempotent
//! in-flight builds, injects `sccache` + optional workstream target isolation,
//! enforces post-queue `VOX_BROKER_RUN_TIMEOUT_SECS`, and records split telemetry.
//! For everything else — or on any coordinator error — it transparently runs real
//! cargo.
//!
//! See `docs/superpowers/specs/2026-06-18-unified-build-broker-design.md`.

mod runner;

use std::collections::BTreeMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use vox_build_queue::{coalesce, env_filter, global, metrics, queue, resolve};

/// Env var tracking shim nesting depth; a hard backstop against fork bombs.
const DEPTH_VAR: &str = "VOX_BROKER_DEPTH";

/// Ask rustup for the active toolchain's cargo (honors `rust-toolchain.toml` /
/// overrides via cwd, and an explicit toolchain via `RUSTUP_TOOLCHAIN`). Used
/// when the shim is installed AS the cargo proxy, so the real cargo can't be
/// found by name on PATH (that name is us).
fn rustup_which_cargo(toolchain: Option<&str>) -> Option<PathBuf> {
    let mut c = Command::new("rustup");
    if let Some(tc) = toolchain {
        c.env("RUSTUP_TOOLCHAIN", tc);
    }
    let out = c.args(["which", "cargo"]).output().ok()?;
    if !out.status.success() {
        return None;
    }
    let p = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim().to_string());
    p.is_file().then_some(p)
}

/// Resolve the real cargo. Two install modes:
/// - PATH-prepend: a DISTINCT rustup proxy exists at `~/.cargo/bin` -> delegate
///   to it (it handles toolchain selection).
/// - Proxy-shadow: the shim IS `~/.cargo/bin/cargo.exe` -> ask rustup for the
///   toolchain cargo. `RUSTUP_TOOLCHAIN` (set by the caller for `+toolchain`)
///   makes both modes honor an explicit toolchain.
fn real_cargo(toolchain: Option<&str>) -> Option<PathBuf> {
    let own = std::env::current_exe().ok()?;
    let own_canon = own.canonicalize().ok();
    if let Some(proxy) = resolve::cargo_home_proxy()
        && proxy.canonicalize().ok() != own_canon
    {
        return Some(proxy);
    }
    if let Some(c) = rustup_which_cargo(toolchain) {
        return Some(c);
    }
    let path = std::env::var("PATH").unwrap_or_default();
    resolve::resolve_real_cargo(&path, &own)
}

/// Run real cargo with the given args, propagating its exit code. Never returns.
fn exec_real(
    real: &Path,
    args: &[String],
    depth: u32,
    toolchain: Option<&str>,
    extra_env: Option<&BTreeMap<String, String>>,
) -> ! {
    let mut c = Command::new(real);
    if let Some(env_map) = extra_env {
        c.env_clear();
        c.envs(env_map);
    }
    c.args(args).env(DEPTH_VAR, (depth + 1).to_string());
    if let Some(tc) = toolchain {
        c.env("RUSTUP_TOOLCHAIN", tc);
    }
    let code = c.status().ok().and_then(|s| s.code()).unwrap_or(1);
    std::process::exit(code);
}

fn write_stdout(bytes: &[u8]) {
    let mut out = std::io::stdout();
    let _ = out.write_all(bytes);
    let _ = out.flush();
}

fn write_stderr(bytes: &[u8]) {
    let mut err = std::io::stderr();
    let _ = err.write_all(bytes);
    let _ = err.flush();
}

#[allow(clippy::too_many_arguments)]
fn record_telemetry(
    root: &Path,
    worktree: &Path,
    sub: &str,
    argv_hash: u64,
    env_hash: u64,
    target_wait_ms: u64,
    slot_wait_ms: u64,
    compile_ms: u64,
    busy: usize,
    cap: usize,
    would_coalesce: bool,
    coalesced_hit: bool,
    timed_out: bool,
    exit_code: i32,
) {
    let queue_wait_ms = target_wait_ms + slot_wait_ms;
    let rec = metrics::MetricRecord {
        ts_ms: metrics::now_ms(),
        worktree: worktree.display().to_string(),
        subcmd: sub.to_string(),
        queue_wait_ms,
        ran_ms: compile_ms,
        argv_hash,
        env_hash,
        would_coalesce,
        target_wait_ms,
        slot_wait_ms,
        compile_ms,
        coalesced_hit,
        timed_out,
    };
    let _ = metrics::append(&root.join("metrics.jsonl"), &rec);
    let line = format!(
        "{ts} {sub:<6} wait={queue_wait_ms:>6}ms target_wait={target_wait_ms}ms slot_wait={slot_wait_ms}ms compile={compile_ms}ms ahead={busy} cap={cap} coalesce={would_coalesce} coalesced={coalesced_hit} timeout={timed_out} exit={exit_code} {wt}\n",
        ts = rec.ts_ms,
        wt = worktree.display(),
    );
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(root.join("broker.log"))
    {
        let _ = f.write_all(line.as_bytes());
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let depth: u32 = std::env::var(DEPTH_VAR)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(0);
    if depth >= 2 {
        eprintln!("vox-broker: recursion detected (depth {depth}); aborting to avoid fork bomb");
        std::process::exit(70);
    }

    let (toolchain, raw_rest): (Option<String>, Vec<String>) =
        match args.first().and_then(|a| a.strip_prefix('+')) {
            Some(tc) => (Some(tc.to_string()), args[1..].to_vec()),
            None => (None, args.clone()),
        };
    let tc = toolchain.as_deref();

    let real = match real_cargo(tc) {
        Some(r) => r,
        None => {
            eprintln!("vox-broker: real cargo not found (rustup unavailable?); aborting");
            std::process::exit(127);
        }
    };

    let (rest, timeout_secs) = runner::strip_broker_flags(&raw_rest);

    if std::env::var_os("VOX_BROKER_DEBUG").is_some() {
        eprintln!(
            "vox-broker-debug: own={:?} real={:?} toolchain={:?} args={:?}",
            std::env::current_exe(),
            real,
            toolchain,
            rest
        );
        std::process::exit(0);
    }

    let sub = rest.first().map(String::as_str).unwrap_or("");

    if !resolve::is_build_subcommand(sub)
        || std::env::var("VOX_BROKER_BYPASS").is_ok_and(|v| {
            matches!(
                v.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    {
        exec_real(&real, &rest, depth, tc, None);
    }

    match run_global(&real, &rest, sub, depth, tc, timeout_secs) {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("vox-broker: coordinator error ({e}); running cargo directly");
            exec_real(&real, &rest, depth, tc, None);
        }
    }
}

fn compute_coalesce_key(
    effective_sub: &str,
    argv_hash: u64,
    env_hash: u64,
    target_hash: &str,
    cwd: &Path,
    toolchain: Option<&str>,
    env: &BTreeMap<String, String>,
) -> String {
    let canon_cwd = resolve::normalize_target_path(cwd, Path::new("."));
    let cwd_hash = queue::hash_path(&canon_cwd);
    let tc_tag = toolchain
        .or_else(|| env.get("RUSTUP_TOOLCHAIN").map(String::as_str))
        .unwrap_or("")
        .trim();
    queue::hash_path(Path::new(&format!(
        "{effective_sub}:{argv_hash:016x}:{env_hash:016x}:{target_hash}:{cwd_hash}:{tc_tag}"
    )))
}

fn run_global(
    real: &Path,
    args: &[String],
    sub: &str,
    depth: u32,
    toolchain: Option<&str>,
    timeout_secs: Option<u64>,
) -> anyhow::Result<i32> {
    let root = global::global_root();
    let n = global::effective_max_concurrent();
    let cwd = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));

    let mut env = env_filter::passthrough_env(std::env::vars());
    if resolve::should_inject_sccache(&env)
        && let Some(sc) = resolve::resolve_sccache_wrapper(
            &std::env::var("PATH").unwrap_or_default(),
            std::env::current_exe().ok().as_deref(),
        )
    {
        env.insert("RUSTC_WRAPPER".into(), sc.to_string_lossy().into_owned());
    }
    let sccache_active = env
        .get("RUSTC_WRAPPER")
        .is_some_and(|v| !v.trim().is_empty());
    let _ = resolve::apply_workstream_target_isolation(&cwd, args, &mut env, sccache_active);

    let effective_target = resolve::effective_target_dir(&cwd, args, &env);
    let target_hash = queue::hash_path(&effective_target);
    let build_args = resolve::build_args_for_run(args);
    let run_cmd_args: &[String] = match &build_args {
        Some(b) => b.as_slice(),
        None => args,
    };
    let effective_sub = if build_args.is_some() { "build" } else { sub };
    let argv_hash = env_filter::argv_hash(run_cmd_args);
    let env_hash = env_filter::env_hash(&env);
    let worktree = resolve::worktree_root_of(&cwd).unwrap_or_else(|| cwd.clone());
    let coalesce_key = compute_coalesce_key(
        effective_sub,
        argv_hash,
        env_hash,
        &target_hash,
        &cwd,
        toolchain,
        &env,
    );
    let can_coalesce = coalesce::is_coalesceable(sub, args, &env);
    let src_mtime = if can_coalesce {
        coalesce::latest_source_mtime_ms(&worktree)
    } else {
        0
    };
    let arrival_ts_ms = metrics::now_ms();

    // Stage A: Attach directly to an active in-flight leader without taking any lock or slot.
    if can_coalesce
        && let Some((code, waited_ms)) = coalesce::try_follow_inflight(
            &root,
            &coalesce_key,
            src_mtime,
            write_stdout,
            write_stderr,
        )
    {
        record_telemetry(
            &root, &worktree, sub, argv_hash, env_hash, waited_ms, 0, 0, 0, n, true, true, false,
            code,
        );
        if build_args.is_some() && code == 0 {
            exec_real(real, args, depth, toolchain, Some(&env));
        }
        return Ok(code);
    }

    // Stage B: Acquire per-target FairQueue BEFORE claiming a global adaptive slot.
    let fq = queue::FairQueue::new(&root.join("targets"), &target_hash)?;
    let seq = fq.take_ticket(&coalesce_key)?;
    let ticket_ts_ms = fq
        .ticket_timestamp_ms(seq)
        .map(|ts| ts.min(arrival_ts_ms))
        .unwrap_or(arrival_ts_ms);
    let would_coalesce_q = fq.coalesce_opportunity(seq, &coalesce_key);
    let t_target = Instant::now();
    let _target_ticket = fq.acquire(seq)?;
    let target_wait_ms = t_target.elapsed().as_millis() as u64;
    let src_mtime_post_wait = if can_coalesce {
        coalesce::latest_source_mtime_ms(&worktree)
    } else {
        0
    };

    if can_coalesce
        && let Some(code) = coalesce::try_replay_recent_leader(
            &root,
            &coalesce_key,
            ticket_ts_ms,
            src_mtime_post_wait,
            write_stdout,
            write_stderr,
        )
    {
        drop(_target_ticket);
        record_telemetry(
            &root,
            &worktree,
            sub,
            argv_hash,
            env_hash,
            target_wait_ms,
            0,
            0,
            0,
            n,
            true,
            true,
            false,
            code,
        );
        if build_args.is_some() && code == 0 {
            exec_real(real, args, depth, toolchain, Some(&env));
        }
        return Ok(code);
    }

    // Stage C: Claim leader role + acquire machine-wide adaptive slot.
    let leader = if can_coalesce {
        coalesce::claim_leader(&root, &coalesce_key, effective_sub, &effective_target)?
    } else {
        None
    };
    let (_inflight, would_coalesce_g) = global::register_inflight(&root, &coalesce_key)?;
    let t_slot = Instant::now();
    let (_slot, _, busy) = global::acquire_slot_adaptive(&root, n)?;
    let slot_wait_ms = t_slot.elapsed().as_millis() as u64;
    if busy > 0 || target_wait_ms > 0 {
        eprintln!(
            "vox-broker: queued (cap {n}, {busy} build(s) ahead, target_wait={target_wait_ms}ms) — {sub} in {}",
            cwd.display()
        );
    }

    // Stage D: Active execution with post-queue timeout supervision.
    let queue_wait_ms = target_wait_ms + slot_wait_ms;
    let outcome = runner::run_child_post_queue(
        real,
        run_cmd_args,
        &cwd,
        &env,
        depth,
        toolchain,
        DEPTH_VAR,
        timeout_secs,
        queue_wait_ms,
        leader.as_ref().map(|l| l.stream_paths()),
    )?;

    if let Some(l) = leader {
        l.finish(outcome.exit_code, outcome.timed_out);
    }
    drop(_slot);
    drop(_inflight);
    drop(_target_ticket);

    record_telemetry(
        &root,
        &worktree,
        sub,
        argv_hash,
        env_hash,
        target_wait_ms,
        slot_wait_ms,
        outcome.compile_ms,
        busy,
        n,
        would_coalesce_q || would_coalesce_g,
        false,
        outcome.timed_out,
        outcome.exit_code,
    );

    if build_args.is_some() && outcome.exit_code == 0 {
        exec_real(real, args, depth, toolchain, Some(&env));
    }

    Ok(outcome.exit_code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coalesce_key_distinguishes_sibling_cwds_and_toolchains() {
        let tmp = tempfile::tempdir().unwrap();
        let crate_a = tmp.path().join("crates/crate-a");
        let crate_b = tmp.path().join("crates/crate-b");
        std::fs::create_dir_all(&crate_a).unwrap();
        std::fs::create_dir_all(&crate_b).unwrap();

        let empty_env = BTreeMap::new();
        let nightly_env = BTreeMap::from([("RUSTUP_TOOLCHAIN".into(), "nightly".into())]);
        let stable_env = BTreeMap::from([("RUSTUP_TOOLCHAIN".into(), "stable".into())]);

        let ka = compute_coalesce_key("check", 11, 22, "tgt0", &crate_a, None, &empty_env);
        let kb = compute_coalesce_key("check", 11, 22, "tgt0", &crate_b, None, &empty_env);
        assert_ne!(
            ka, kb,
            "sibling member directories must not share coalesce_key"
        );

        let k_plus_nightly = compute_coalesce_key(
            "check",
            11,
            22,
            "tgt0",
            &crate_a,
            Some("nightly"),
            &empty_env,
        );
        let k_env_nightly =
            compute_coalesce_key("check", 11, 22, "tgt0", &crate_a, None, &nightly_env);
        let k_env_stable =
            compute_coalesce_key("check", 11, 22, "tgt0", &crate_a, None, &stable_env);
        assert_ne!(
            ka, k_plus_nightly,
            "+nightly must differ from default toolchain"
        );
        assert_ne!(
            k_env_nightly, k_env_stable,
            "RUSTUP_TOOLCHAIN=nightly vs stable must differ"
        );
        assert_eq!(
            k_plus_nightly, k_env_nightly,
            "+nightly and RUSTUP_TOOLCHAIN=nightly match"
        );
    }
}
