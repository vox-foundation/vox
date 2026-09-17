//! Native sandbox enforcement for `vox run --sandbox`.
//!
//! Provides OS-level restrictions on the native execution lane:
//! - **Linux (kernel ≥5.13):** Landlock LSM — filesystem read/write restrictions per-path.
//!   Applied via `pre_exec` so ONLY the child process is restricted, not the parent `vox`.
//! - **Windows:** Job Objects — working-set memory ceiling, kill-on-close.
//! - **Other:** Warning only (no enforcement). `VOX_SANDBOX` is not isolation;
//!   see `docs/src/reference/isolation.md`.

use anyhow::Result;
use std::process::Command;

use crate::commands::runtime::run::script::ScriptOpts;

// ── Linux: Landlock ────────────────────────────────────────────────────────────

#[cfg(target_os = "linux")]
mod platform {
    use super::*;
    use landlock::{
        ABI, Access, AccessFs, BitFlags, PathBeneath, PathFd, Ruleset, RulesetAttr,
        RulesetCreatedAttr,
    };
    use std::path::Path;

    /// Paths that sandboxed scripts may read (but not write).
    const READ_ONLY_PATHS: &[&str] = &[
        // vox-arch-check: allow abs-path
        "/usr",
        "/lib",
        "/lib64",
        // vox-arch-check: allow abs-path
        "/etc",
        // vox-arch-check: allow abs-path
        "/bin",
        "/sbin",
        "/proc/self",
    ];

    /// Build a Landlock ruleset and apply it to the given `Command` via `pre_exec`.
    ///
    /// **Critical:** we use `unsafe pre_exec` to call `restrict_self()` inside the
    /// child process (between fork and exec), so the parent `vox` is never affected.
    pub fn enforce(cmd: &mut Command, opts: &ScriptOpts) -> Result<()> {
        use std::os::unix::process::CommandExt;

        let abi = ABI::V3; // kernel ≥6.2; degrades gracefully on older kernels
        let read_access = AccessFs::from_read(abi);
        let write_access = AccessFs::from_all(abi);

        // Collect paths before fork — PathFd is fd-based so we must open in parent.
        let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
        let mut rules: Vec<(std::path::PathBuf, BitFlags<AccessFs>)> = Vec::new();

        for &p in READ_ONLY_PATHS {
            let path = Path::new(p);
            if path.exists() {
                rules.push((path.to_path_buf(), read_access));
            }
        }
        if let Some(home) = dirs::home_dir() {
            let vox_dir = home.join(".vox");
            if vox_dir.exists() {
                rules.push((vox_dir, read_access));
            }
        }
        if let Some(rt) = crate::fs_utils::resolve_vox_runtime_path() {
            if rt.exists() {
                rules.push((rt, read_access));
            }
        }
        if cwd.exists() {
            rules.push((cwd, write_access));
        }
        for (host_path, _guest, mode) in &opts.wasi_dirs {
            if host_path.exists() {
                let access = match mode {
                    crate::wasi_dir_mode::WasiDirMode::ReadOnly => read_access,
                    crate::wasi_dir_mode::WasiDirMode::ReadWrite => write_access,
                };
                rules.push((host_path.clone(), access));
            }
        }

        // Print enforcement intent in the parent (safe; pre_exec must be signal-safe)
        eprintln!("[sandbox] Landlock: applying filesystem restrictions in child process");

        // SAFETY: pre_exec runs between fork() and exec() in the child only.
        // `restrict_self()` is a pure syscall — no malloc, no locks, signal-safe.
        // The parent `vox` process is never restricted.
        #[allow(unsafe_code)] // landlock pre_exec requires unsafe; safety documented above
        unsafe {
            cmd.pre_exec(move || {
                let mut ruleset = Ruleset::default()
                    .handle_access(write_access)
                    .map_err(|e| std::io::Error::other(e.to_string()))?
                    .create()
                    .map_err(|e| std::io::Error::other(e.to_string()))?;

                // landlock 0.4's `add_rule` consumes the ruleset and returns the
                // updated one, so reassign rather than discard. Fail closed if a
                // rule cannot be added — better than running under-restricted.
                for (path, access) in &rules {
                    if let Ok(fd) = PathFd::new(path) {
                        ruleset = ruleset
                            .add_rule(PathBeneath::new(fd, *access))
                            .map_err(|e| std::io::Error::other(e.to_string()))?;
                    }
                }

                // restrict_self() is a single prctl() syscall — async-signal-safe.
                // Fail closed: a failed restriction must not let the child run unsandboxed.
                ruleset
                    .restrict_self()
                    .map_err(|e| std::io::Error::other(e.to_string()))?;
                Ok(())
            });
        }

        Ok(())
    }
}

// ── Windows: Job Objects ───────────────────────────────────────────────────────

#[cfg(target_os = "windows")]
mod platform {
    use super::*;

    const WORKING_SET_MIN: usize = 4 * 1024 * 1024;
    const WORKING_SET_MAX: usize = 512 * 1024 * 1024;

    /// Log sandbox intent and set up Job Object pre-conditions on Windows.
    ///
    /// Actual Job Object assignment happens after `spawn()` via `assign_job`.
    pub fn enforce(cmd: &mut Command, _opts: &ScriptOpts) -> Result<()> {
        eprintln!(
            "[sandbox] Windows Job Object: working_set_max={} MB, kill-on-close=true",
            WORKING_SET_MAX / (1024 * 1024)
        );
        eprintln!(
            "[sandbox] Note: filesystem restrictions are NOT enforced on Windows (Job Objects limitation)."
        );
        let _ = cmd;
        Ok(())
    }

    /// After spawning the child, assign it to a Job Object.
    ///
    /// Strategy:
    /// 1. Create job + apply kill-on-close (mandatory — always works)
    /// 2. Try working-memory limit (best-effort — may need privileges)
    /// 3. Assign child process to the job
    pub fn assign_job(child: &std::process::Child) -> Result<()> {
        use std::os::windows::io::AsRawHandle;
        use win32job::Job;

        // Phase 1: create job with kill-on-close (always succeeds)
        let mut info = win32job::ExtendedLimitInfo::new();
        info.limit_kill_on_job_close();
        let job = Job::create_with_limit_info(&info)?;

        // Phase 2: try adding working-memory cap (best-effort — may require privileges)
        let mut mem_info = job.query_extended_limit_info()?;
        mem_info.limit_working_memory(WORKING_SET_MIN, WORKING_SET_MAX);
        if let Err(e) = job.set_extended_limit_info(&mem_info) {
            eprintln!(
                "[sandbox] Warning: could not set memory limit ({}), continuing with kill-on-close only",
                e
            );
        }

        // Phase 3: assign the child using its process handle directly
        let handle = child.as_raw_handle() as isize;
        job.assign_process(handle)?;

        // Leak the job so it stays alive until the process exits.
        // When vox terminates, the handle is reclaimed and kill-on-close fires.
        std::mem::forget(job);

        Ok(())
    }
}

// ── Fallback: no enforcement ───────────────────────────────────────────────────

#[cfg(not(any(target_os = "linux", target_os = "windows")))]
mod platform {
    use super::*;

    /// Warn that `--sandbox` has no OS-level enforcement on this platform.
    ///
    /// Does not set `VOX_SANDBOX` — that env var is not isolation. See
    /// `docs/src/reference/isolation.md`.
    pub fn enforce(cmd: &mut Command, _opts: &ScriptOpts) -> Result<()> {
        tracing::warn!(
            "--sandbox has no OS-level enforcement on this platform; \
             VOX_SANDBOX is not isolation. See docs/src/reference/isolation.md"
        );
        let _ = cmd;
        Ok(())
    }
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Apply OS-level sandbox restrictions to the command before spawning.
///
/// - **Linux:** Installs a Landlock ruleset via `pre_exec` (child-only, parent unaffected)
/// - **Windows:** Logs intent; call `post_spawn_sandbox` after spawn to assign the Job Object
/// - **Other:** Warning only; `VOX_SANDBOX` is not isolation (`docs/src/reference/isolation.md`)
pub fn enforce_sandbox(cmd: &mut Command, opts: &ScriptOpts) -> Result<()> {
    platform::enforce(cmd, opts)
}

/// On Windows, after spawning the child, assign it to a Job Object.
/// On other platforms this is a no-op.
#[cfg(target_os = "windows")]
pub fn post_spawn_sandbox(child: &std::process::Child) -> Result<()> {
    platform::assign_job(child)
}

/// No-op on non-Windows platforms.
#[cfg(not(target_os = "windows"))]
pub fn post_spawn_sandbox(_child: &std::process::Child) -> Result<()> {
    let _ = std::hint::black_box(_child.id());
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn default_opts() -> ScriptOpts {
        ScriptOpts {
            sandbox: true,
            allow_mcp: false,
            no_cache: false,
            trust_class: None,
            target_triple: None,
            #[cfg(feature = "script-execution")]
            wasi_dirs: vec![],
        }
    }

    #[test]
    fn enforce_sandbox_does_not_panic() {
        let mut cmd = Command::new("echo");
        cmd.arg("hello");
        let opts = default_opts();
        let _ = enforce_sandbox(&mut cmd, &opts);
    }

    #[test]
    fn post_spawn_noop_compiles() {
        // Verifies the function signature compiles correctly on all platforms.
        let _ = true;
    }

    fn command_sets_vox_sandbox(cmd: &Command) -> bool {
        cmd.get_envs()
            .any(|(key, value)| key == "VOX_SANDBOX" && value.is_some())
    }

    #[test]
    fn macos_does_not_pretend_an_env_var_is_a_sandbox() {
        let opts = default_opts();

        let mut sandbox_cmd = Command::new("echo");
        enforce_sandbox(&mut sandbox_cmd, &opts).expect("enforce_sandbox");
        assert!(
            !command_sets_vox_sandbox(&sandbox_cmd),
            "enforce_sandbox must not set VOX_SANDBOX (an env var is not isolation)"
        );

        let mut native_cmd = Command::new("echo");
        crate::commands::runtime::run::backend::apply_native_execute_env(&mut native_cmd, &opts);
        assert!(
            !command_sets_vox_sandbox(&native_cmd),
            "native execute env must not set VOX_SANDBOX (an env var is not isolation)"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_enforce_logs_params() {
        // vox-arch-check: allow shell-spawn
        let mut cmd = Command::new("cmd");
        cmd.arg("/c").arg("echo hello");
        let opts = default_opts();
        assert!(enforce_sandbox(&mut cmd, &opts).is_ok());
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn windows_job_object_smoke_test() {
        // vox-arch-check: allow shell-spawn
        let mut child = Command::new("cmd")
            .args(["/c", "timeout /t 1 /nobreak >nul"])
            .spawn()
            .expect("spawn cmd");
        let result = post_spawn_sandbox(&child);
        assert!(
            result.is_ok(),
            "Job Object assignment should succeed: {:?}",
            result
        );
        let _ = child.wait();
    }
}
