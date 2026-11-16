//! Post-queue child execution, non-blocking file tail-teeing, and active-compile timeout supervision.

use std::collections::BTreeMap;
use std::io::{IsTerminal, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

const CHILD_POLL_MS: u64 = 50;

pub struct RunOutcome {
    pub exit_code: i32,
    pub compile_ms: u64,
    pub timed_out: bool,
}

fn parse_secs_opt(raw: Option<&str>) -> Option<u64> {
    raw.map(str::trim)
        .filter(|s| !s.is_empty())
        .and_then(|s| s.parse::<u64>().ok())
}

fn resolve_timeout_secs_from(
    cli_secs: Option<u64>,
    run_env: Option<&str>,
    build_env: Option<&str>,
) -> Option<u64> {
    cli_secs
        .or_else(|| parse_secs_opt(run_env))
        .or_else(|| parse_secs_opt(build_env))
        .filter(|&n| n > 0)
}

/// Strip `--broker-run-timeout <secs>` and `--broker-run-timeout=<secs>` before `--`
/// and compute effective active-compile timeout seconds (`None` when disabled or `0`).
pub fn strip_broker_flags(args: &[String]) -> (Vec<String>, Option<u64>) {
    let (out, cli_secs) = strip_broker_cli_args(args);
    let run_env = std::env::var("VOX_BROKER_RUN_TIMEOUT_SECS").ok();
    let build_env = std::env::var("VOX_BROKER_BUILD_TIMEOUT_SECS").ok();
    let effective = resolve_timeout_secs_from(cli_secs, run_env.as_deref(), build_env.as_deref());
    (out, effective)
}

fn strip_broker_cli_args(args: &[String]) -> (Vec<String>, Option<u64>) {
    let mut out = Vec::with_capacity(args.len());
    let mut cli_secs: Option<u64> = None;
    let mut after_dashdash = false;
    let mut i = 0;
    while i < args.len() {
        let cur = &args[i];
        if after_dashdash {
            out.push(cur.clone());
            i += 1;
            continue;
        }
        if cur == "--" {
            after_dashdash = true;
            out.push(cur.clone());
            i += 1;
            continue;
        }
        if cur == "--broker-run-timeout" {
            if let Some(val) = args.get(i + 1)
                && let Ok(parsed) = val.trim().parse::<u64>()
            {
                cli_secs = Some(parsed);
                i += 2;
                continue;
            }
            i += 1;
            continue;
        }
        if let Some(val) = cur.strip_prefix("--broker-run-timeout=") {
            if let Ok(parsed) = val.trim().parse::<u64>() {
                cli_secs = Some(parsed);
            }
            i += 1;
            continue;
        }
        out.push(cur.clone());
        i += 1;
    }
    (out, cli_secs)
}

fn should_force_cargo_color(
    env: &BTreeMap<String, String>,
    no_color_os: bool,
    stderr_is_tty: bool,
) -> bool {
    stderr_is_tty
        && !no_color_os
        && !env.contains_key("CARGO_TERM_COLOR")
        && !env.contains_key("NO_COLOR")
}

fn tail_stream(path: &Path, offset: u64, is_stderr: bool) -> u64 {
    let Ok(mut f) = std::fs::File::open(path) else {
        return offset;
    };
    if f.seek(SeekFrom::Start(offset)).is_err() {
        return offset;
    }
    let (mut cur, mut buf) = (offset, [0u8; 8192]);
    while let Ok(n) = f.read(&mut buf) {
        if n == 0 {
            break;
        }
        let chunk = &buf[..n];
        if is_stderr {
            let mut err = std::io::stderr();
            let _ = err.write_all(chunk);
            let _ = err.flush();
        } else {
            let mut out = std::io::stdout();
            let _ = out.write_all(chunk);
            let _ = out.flush();
        }
        cur += n as u64;
    }
    cur
}

/// Spawn `real` strictly AFTER queue and slot locks are held. When `tee_files` is set,
/// Cargo writes directly to unlocked `.stdout`/`.stderr` files while this loop tails them
/// non-blockingly so background grandchildren (`sccache` daemon, etc.) inheriting stdio
/// descriptors can never hang post-exit joins or hold `FairQueue` locks.
#[allow(clippy::too_many_arguments)]
pub fn run_child_post_queue(
    real: &Path,
    run_cmd_args: &[String],
    cwd: &Path,
    env: &BTreeMap<String, String>,
    depth: u32,
    toolchain: Option<&str>,
    depth_var: &str,
    timeout_secs: Option<u64>,
    queue_wait_ms: u64,
    tee_files: Option<(&Path, &Path)>,
) -> anyhow::Result<RunOutcome> {
    let mut cmd = Command::new(real);
    cmd.args(run_cmd_args).current_dir(cwd);
    cmd.env_clear();
    cmd.envs(env);
    if should_force_cargo_color(
        env,
        std::env::var_os("NO_COLOR").is_some(),
        std::io::stderr().is_terminal(),
    ) {
        cmd.env("CARGO_TERM_COLOR", "always");
    }
    cmd.env(depth_var, (depth + 1).to_string());
    if let Some(tc) = toolchain {
        cmd.env("RUSTUP_TOOLCHAIN", tc);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }

    if let Some((out_p, err_p)) = tee_files {
        let out_f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(out_p)?;
        let err_f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(err_p)?;
        cmd.stdout(Stdio::from(out_f)).stderr(Stdio::from(err_f));
    } else {
        cmd.stdout(Stdio::inherit()).stderr(Stdio::inherit());
    }

    let t_run = Instant::now();
    let mut child = cmd.spawn()?;
    drop(cmd);

    let deadline = timeout_secs.filter(|&s| s > 0).map(Duration::from_secs);
    let mut timed_out = false;
    let (mut out_off, mut err_off) = (0u64, 0u64);
    let exit_code = loop {
        if let Some((out_p, err_p)) = tee_files {
            out_off = tail_stream(out_p, out_off, false);
            err_off = tail_stream(err_p, err_off, true);
        }
        if let Some(status) = child.try_wait()? {
            break status.code().unwrap_or(1);
        }
        if let Some(limit) = deadline
            && t_run.elapsed() >= limit
        {
            timed_out = true;
            vox_build_queue::resolve::kill_process_tree(child.id());
            let _ = child.kill();
            let _ = child.wait();
            break 124;
        }
        std::thread::sleep(Duration::from_millis(CHILD_POLL_MS));
    };
    let compile_ms = t_run.elapsed().as_millis() as u64;

    if let Some((out_p, err_p)) = tee_files {
        let _ = tail_stream(out_p, out_off, false);
        let _ = tail_stream(err_p, err_off, true);
    }

    if timed_out {
        let secs = timeout_secs.unwrap_or(0);
        eprintln!(
            "vox-broker: active build exceeded timeout ({secs}s, excluding {queue_wait_ms}ms queue wait); exiting 124"
        );
    }

    Ok(RunOutcome {
        exit_code,
        compile_ms,
        timed_out,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sv(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| (*s).to_string()).collect()
    }

    #[test]
    fn strip_flags_preserves_non_numeric_next_arg_and_zero_disables_fallback() {
        let raw = sv(&[
            "test",
            "--broker-run-timeout",
            "45",
            "-p",
            "demo",
            "--",
            "--broker-run-timeout=999",
        ]);
        let (cleaned, cli) = strip_broker_cli_args(&raw);
        assert_eq!(
            cleaned,
            sv(&["test", "-p", "demo", "--", "--broker-run-timeout=999"])
        );
        assert_eq!(cli, Some(45));

        let missing_num = sv(&["check", "--broker-run-timeout", "-p", "vox-cli"]);
        let (cleaned_no_swallow, cli_none) = strip_broker_cli_args(&missing_num);
        assert_eq!(cleaned_no_swallow, sv(&["check", "-p", "vox-cli"]));
        assert_eq!(cli_none, None);

        assert_eq!(
            resolve_timeout_secs_from(None, Some("120"), Some("300")),
            Some(120)
        );
        assert_eq!(
            resolve_timeout_secs_from(None, Some("0"), Some("300")),
            None,
            "VOX_BROKER_RUN_TIMEOUT_SECS=0 must explicitly disable timeout without falling back"
        );
        assert_eq!(
            resolve_timeout_secs_from(None, None, Some("300")),
            Some(300)
        );
        assert_eq!(resolve_timeout_secs_from(Some(0), Some("120"), None), None);
    }

    #[test]
    fn color_policy_respects_caller_overrides_no_color_and_tty() {
        let empty = BTreeMap::new();
        assert!(should_force_cargo_color(&empty, false, true));
        assert!(!should_force_cargo_color(&empty, false, false));
        assert!(!should_force_cargo_color(&empty, true, true));
        let never = BTreeMap::from([("CARGO_TERM_COLOR".into(), "never".into())]);
        assert!(!should_force_cargo_color(&never, false, true));
        let no_col = BTreeMap::from([("NO_COLOR".into(), "1".into())]);
        assert!(!should_force_cargo_color(&no_col, false, true));
    }

    #[cfg(unix)]
    #[test]
    fn background_grandchild_inheriting_stdio_never_blocks_runner() {
        let tmp = tempfile::tempdir().unwrap();
        let out_p = tmp.path().join("leader.stdout");
        let err_p = tmp.path().join("leader.stderr");
        let pid_p = tmp.path().join("bg.pid");
        let script = format!(
            "echo child-out; echo child-err >&2; sleep 10 & echo $! > '{}'; exit 0",
            pid_p.display()
        );
        let args = sv(&["-c", &script]);
        let env = BTreeMap::new();

        let t0 = Instant::now();
        let res = run_child_post_queue(
            Path::new("/bin/sh"),
            &args,
            tmp.path(),
            &env,
            0,
            None,
            "VOX_BROKER_DEPTH",
            Some(5),
            0,
            Some((&out_p, &err_p)),
        )
        .unwrap();
        let elapsed = t0.elapsed();

        if let Ok(raw) = std::fs::read_to_string(&pid_p)
            && let Ok(pid) = raw.trim().parse::<i32>()
        {
            let _ = Command::new("/bin/kill")
                .args(["-9", &pid.to_string()])
                .status();
        }

        assert_eq!(res.exit_code, 0);
        assert!(!res.timed_out);
        assert!(
            elapsed < Duration::from_secs(2),
            "runner hung waiting on background grandchild stdio: {elapsed:?}"
        );
        assert_eq!(std::fs::read_to_string(&out_p).unwrap(), "child-out\n");
        assert_eq!(std::fs::read_to_string(&err_p).unwrap(), "child-err\n");
    }
}
